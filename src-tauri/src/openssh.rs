use std::path::PathBuf;
use std::process::Command;

use crate::error::{codes, AppResult, CommandError};
use crate::model::OpenSshInfo;
#[cfg(not(windows))]
use crate::util::truncate;
use crate::util::{combined_output, run_output};

#[derive(Debug, Clone)]
pub struct OpenSshPaths {
    pub ssh: PathBuf,
    pub keygen: PathBuf,
    pub add: PathBuf,
    #[cfg_attr(windows, allow(dead_code))]
    pub agent: PathBuf,
}

#[derive(Debug, Clone)]
pub struct OpenSsh {
    pub info: OpenSshInfo,
    pub paths: Option<OpenSshPaths>,
}

impl OpenSsh {
    pub fn detect() -> Self {
        let ssh = find_binary("ssh");
        let keygen = find_binary("ssh-keygen");
        let add = find_binary("ssh-add");
        let agent = find_binary("ssh-agent");

        let mut missing = Vec::new();
        if ssh.is_none() {
            missing.push("ssh".to_string());
        }
        if keygen.is_none() {
            missing.push("ssh-keygen".to_string());
        }
        if add.is_none() {
            missing.push("ssh-add".to_string());
        }
        if agent.is_none() {
            missing.push("ssh-agent".to_string());
        }

        let paths = match (&ssh, &keygen, &add, &agent) {
            (Some(ssh), Some(keygen), Some(add), Some(agent)) => Some(OpenSshPaths {
                ssh: ssh.clone(),
                keygen: keygen.clone(),
                add: add.clone(),
                agent: agent.clone(),
            }),
            _ => None,
        };

        let version = ssh.as_ref().and_then(|program| {
            let mut command = Command::new(program);
            command.arg("-V");
            let output = command.output().ok()?;
            first_non_empty(&combined_output(&output))
        });

        let install_hint = install_hint();
        let auto_install_supported = auto_install_supported();

        Self {
            info: OpenSshInfo {
                available: missing.is_empty(),
                version,
                missing,
                install_hint,
                auto_install_supported,
            },
            paths,
        }
    }

    pub fn require(&self) -> AppResult<&OpenSshPaths> {
        self.paths.as_ref().ok_or_else(|| {
            CommandError::new(
                codes::OPENSSH_MISSING,
                format!("未找到 OpenSSH 组件：{}。", self.info.missing.join(", ")),
            )
            .hint(self.info.install_hint.clone())
        })
    }

    /// Install OpenSSH using the platform package mechanism. Returns a bounded log.
    pub fn install(&self) -> AppResult<String> {
        if self.info.available {
            return Ok("OpenSSH 已经可用，无需安装。".to_string());
        }
        #[cfg(windows)]
        {
            install_windows()
        }
        #[cfg(target_os = "macos")]
        {
            install_macos()
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            install_linux()
        }
    }
}

fn find_binary(name: &str) -> Option<PathBuf> {
    let file_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    for directory in search_directories() {
        let candidate = directory.join(&file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn search_directories() -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            let root = PathBuf::from(system_root);
            directories.push(root.join("System32").join("OpenSSH"));
            directories.push(root.join("Sysnative").join("OpenSSH"));
        }
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            directories.push(PathBuf::from(program_files).join("OpenSSH"));
        }
    }
    #[cfg(not(windows))]
    {
        for directory in [
            "/usr/bin",
            "/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/opt/homebrew/opt/openssh/bin",
            "/usr/local/opt/openssh/bin",
            "/opt/local/bin",
        ] {
            directories.push(PathBuf::from(directory));
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        directories.extend(std::env::split_paths(&path));
    }
    let mut deduped = Vec::new();
    for directory in directories {
        if !deduped.contains(&directory) {
            deduped.push(directory);
        }
    }
    deduped
}

fn first_non_empty(value: &str) -> Option<String> {
    value
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn install_hint() -> String {
    #[cfg(windows)]
    {
        "在“设置 → 应用 → 可选功能”中添加 OpenSSH 客户端，或运行 `winget install Microsoft.OpenSSH.Beta`，随后重新检查环境。".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        "macOS 通常自带 OpenSSH。若被移除，请运行 `xcode-select --install`，或使用 Homebrew 安装：`brew install openssh`。".to_string()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        match linux_package_manager() {
            Some(manager) => format!(
                "请安装 OpenSSH 客户端，例如：`{}`，随后重新检查环境。",
                manager.install_command()
            ),
            None => "请使用系统包管理器安装 OpenSSH 客户端（openssh-client）。".to_string(),
        }
    }
}

#[cfg(windows)]
fn auto_install_supported() -> bool {
    powershell_path().is_some()
}

#[cfg(target_os = "macos")]
fn auto_install_supported() -> bool {
    find_binary("brew").is_some()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn auto_install_supported() -> bool {
    if linux_package_manager().is_none() {
        return false;
    }
    if is_root() {
        return true;
    }
    find_binary("pkexec").is_some()
}

#[cfg(windows)]
fn powershell_path() -> Option<PathBuf> {
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        let candidate = PathBuf::from(system_root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    find_binary("powershell")
}

#[cfg(windows)]
fn install_windows() -> AppResult<String> {
    let powershell = powershell_path().ok_or_else(|| {
        CommandError::new(
            codes::INSTALL_FAILED,
            "找不到 Windows PowerShell，无法自动安装 OpenSSH。",
        )
        .hint(install_hint())
    })?;

    // Add-WindowsCapability must run elevated. We start one elevated PowerShell
    // and pass the fixed installer command as an encoded command. No user input
    // is interpolated into this script.
    let script = r#"$ErrorActionPreference = 'Stop'
$inner = @'
$ProgressPreference = 'SilentlyContinue'
$ErrorActionPreference = 'Stop'
$capability = 'OpenSSH.Client~~~~0.0.1.0'
$state = (Get-WindowsCapability -Online -Name $capability).State
if ($state -ne 'Installed') {
  Add-WindowsCapability -Online -Name $capability | Out-Null
}
'@
$encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($inner))
$process = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-EncodedCommand',$encoded) -Verb RunAs -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "OpenSSH installation failed with exit code $($process.ExitCode)." }
Write-Output 'OpenSSH capability installation completed.'
"#;

    let mut command = Command::new(powershell);
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]);
    let output = run_output(&mut command, "无法启动 OpenSSH 安装程序。")?;
    let log = combined_output(&output);
    if !output.status.success() {
        return Err(CommandError::new(
            codes::INSTALL_FAILED,
            "OpenSSH 自动安装未完成。UAC 授权可能被取消，或 Windows 更新不可用。",
        )
        .hint(install_hint())
        .detail(if log.is_empty() {
            format!("exit code: {:?}", output.status.code())
        } else {
            log
        }));
    }
    Ok(if log.is_empty() {
        "OpenSSH 安装命令已完成。".to_string()
    } else {
        log
    })
}

#[cfg(target_os = "macos")]
fn install_macos() -> AppResult<String> {
    let brew = find_binary("brew").ok_or_else(|| {
        CommandError::new(
            codes::INSTALL_FAILED,
            "macOS 通常自带 OpenSSH；自动安装需要 Homebrew，但未找到 brew。",
        )
        .hint(install_hint())
    })?;
    let mut command = Command::new(brew);
    command
        .args(["install", "openssh"])
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("NONINTERACTIVE", "1");
    let output = run_output(&mut command, "无法启动 Homebrew。")?;
    let log = combined_output(&output);
    if !output.status.success() {
        return Err(
            CommandError::new(codes::INSTALL_FAILED, "Homebrew 安装 OpenSSH 失败。")
                .hint(install_hint())
                .detail(log),
        );
    }
    Ok(truncate(&log, 4000))
}

#[cfg(all(unix, not(target_os = "macos")))]
#[derive(Debug, Clone, Copy)]
enum LinuxPackageManager {
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

#[cfg(all(unix, not(target_os = "macos")))]
impl LinuxPackageManager {
    fn program(self) -> &'static str {
        match self {
            Self::Apt => "apt-get",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
        }
    }

    fn install_args(self) -> &'static [&'static str] {
        match self {
            Self::Apt => &["install", "-y", "openssh-client"],
            Self::Dnf => &["install", "-y", "openssh-clients"],
            Self::Pacman => &["-S", "--noconfirm", "openssh"],
            Self::Zypper => &["--non-interactive", "install", "-y", "openssh"],
        }
    }

    fn install_command(self) -> String {
        format!("sudo {} {}", self.program(), self.install_args().join(" "))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_package_manager() -> Option<LinuxPackageManager> {
    for (manager, binary) in [
        (LinuxPackageManager::Apt, "apt-get"),
        (LinuxPackageManager::Dnf, "dnf"),
        (LinuxPackageManager::Pacman, "pacman"),
        (LinuxPackageManager::Zypper, "zypper"),
    ] {
        if find_binary(binary).is_some() {
            return Some(manager);
        }
    }
    None
}

#[cfg(all(unix, not(target_os = "macos")))]
fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
        .unwrap_or(false)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn install_linux() -> AppResult<String> {
    let manager = linux_package_manager().ok_or_else(|| {
        CommandError::new(
            codes::INSTALL_FAILED,
            "未识别到支持的包管理器，无法自动安装 OpenSSH。",
        )
        .hint(install_hint())
    })?;

    let mut command = if is_root() {
        let mut command = Command::new(manager.program());
        command.args(manager.install_args());
        command
    } else if let Some(pkexec) = find_binary("pkexec") {
        let mut command = Command::new(pkexec);
        command.arg(manager.program());
        command.args(manager.install_args());
        command
    } else {
        return Err(CommandError::new(
            codes::INSTALL_FAILED,
            "自动安装需要图形化提权组件 pkexec，但当前系统未提供。",
        )
        .hint(install_hint()));
    };
    command.env("DEBIAN_FRONTEND", "noninteractive");

    let output = run_output(&mut command, "无法启动系统包管理器。")?;
    let log = combined_output(&output);
    if !output.status.success() {
        return Err(CommandError::new(
            codes::INSTALL_FAILED,
            "系统包管理器未能安装 OpenSSH。授权可能被取消，或软件源不可用。",
        )
        .hint(install_hint())
        .detail(log));
    }
    Ok(truncate(&log, 4000))
}
