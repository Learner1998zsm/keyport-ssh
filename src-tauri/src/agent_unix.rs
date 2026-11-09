use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::error::{codes, AppResult, CommandError};
use crate::model::AgentStatus;
use crate::openssh::{OpenSsh, OpenSshPaths};
use crate::paths::{atomic_write, display_path, SshPaths};
#[cfg(all(unix, not(target_os = "macos")))]
use crate::util::run_output;
use crate::util::{combined_output, shell_quote};

#[cfg(target_os = "macos")]
const LAUNCHD_LABEL: &str = "dev.keyport.ssh-agent";
#[cfg(all(unix, not(target_os = "macos")))]
const SYSTEMD_UNIT: &str = "keyport-ssh-agent.service";

pub fn status(openssh: &OpenSsh) -> AgentStatus {
    let paths = match SshPaths::discover() {
        Ok(paths) => paths,
        Err(error) => {
            return AgentStatus {
                ready: false,
                running: false,
                autostart: false,
                socket: None,
                manager: manager_name(),
                summary: "无法确定 ssh-agent 的 socket 路径。".to_string(),
                detail: Some(error.message),
            }
        }
    };
    let socket = managed_socket(&paths);
    let socket_exists = is_socket(&socket);
    #[cfg(target_os = "macos")]
    let autostart = launchd_plist_path(&paths).is_file();
    #[cfg(all(unix, not(target_os = "macos")))]
    let autostart = systemd_unit_enabled(&paths) || linux_desktop_autostart(&paths).is_file();
    let running = socket_exists && (openssh.paths.is_none() || agent_responds(openssh, &socket));

    let summary = match (running, autostart) {
        (true, true) => "ssh-agent 运行中，已设为登录自动启动。",
        (true, false) => "ssh-agent 运行中，但尚未设为自动启动。",
        (false, true) => "ssh-agent 已配置自动启动，但当前无法连接。",
        (false, false) => "ssh-agent 尚未运行或未配置自动启动。",
    };
    let detail = if running {
        None
    } else {
        Some(format!(
            "Socket: {}\n启动后会自动加入由 Keyport 生成的密钥。",
            display_path(&socket)
        ))
    };
    AgentStatus {
        ready: running && autostart,
        running,
        autostart,
        socket: Some(display_path(&socket)),
        manager: manager_name(),
        summary: summary.to_string(),
        detail,
    }
}

pub fn ensure(openssh: &OpenSsh) -> AppResult<AgentStatus> {
    let paths = SshPaths::discover()?;
    paths.ensure_ssh_dir()?;
    let socket = managed_socket(&paths);
    let provider = openssh.require()?;

    #[cfg(target_os = "macos")]
    ensure_launchd(&paths, provider, &socket)?;
    #[cfg(all(unix, not(target_os = "macos")))]
    ensure_linux(&paths, provider, &socket)?;

    write_shell_integration(&paths, &socket)?;

    let ready = wait_for_agent(openssh, &socket, 40);
    let status = status(openssh);
    if !ready {
        return Err(
            CommandError::new(codes::AGENT_FAILED, "ssh-agent 启动后仍无法连接。")
                .hint(format!(
                    "可手动检查 `ssh-add -l` 和 agent socket：{}。",
                    display_path(&socket)
                ))
                .detail(status.detail.clone().unwrap_or_default()),
        );
    }
    if !status.autostart {
        return Err(CommandError::new(
            codes::AGENT_FAILED,
            "ssh-agent 已运行，但无法确认自动启动配置。",
        )
        .hint("请重新尝试配置；Keyport 会在登录时再次启动 agent。")
        .detail(status.detail.clone().unwrap_or_default()));
    }
    Ok(status)
}

fn manager_name() -> String {
    #[cfg(target_os = "macos")]
    {
        "launchd · LaunchAgent".to_string()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if command_on_path("systemctl").is_some() {
            "systemd · user".to_string()
        } else {
            "XDG Autostart".to_string()
        }
    }
}

fn managed_socket(paths: &SshPaths) -> PathBuf {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
            let runtime = PathBuf::from(runtime);
            if !runtime.as_os_str().is_empty() {
                return runtime.join("keyport-ssh-agent.sock");
            }
        }
    }
    paths.ssh_dir.join("keyport-agent.sock")
}

fn is_socket(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.file_type().is_socket())
        .unwrap_or(false)
}

fn agent_responds(openssh: &OpenSsh, socket: &Path) -> bool {
    let Some(provider) = openssh.paths.as_ref() else {
        return false;
    };
    if !is_socket(socket) {
        return false;
    }
    let mut command = Command::new(&provider.add);
    command.arg("-l").env("SSH_AUTH_SOCK", socket);
    match command.output() {
        Ok(output) => matches!(output.status.code(), Some(0) | Some(1)),
        Err(_) => false,
    }
}

fn wait_for_agent(openssh: &OpenSsh, socket: &Path, attempts: usize) -> bool {
    for _ in 0..attempts {
        if agent_responds(openssh, socket) {
            return true;
        }
        thread::sleep(Duration::from_millis(200));
    }
    agent_responds(openssh, socket)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn command_on_path(name: &str) -> Option<PathBuf> {
    let mut directories: Vec<PathBuf> = ["/usr/bin", "/bin", "/usr/local/bin", "/opt/homebrew/bin"]
        .into_iter()
        .map(PathBuf::from)
        .collect();
    if let Some(path) = std::env::var_os("PATH") {
        directories.extend(std::env::split_paths(&path));
    }
    directories
        .into_iter()
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn run_succeeded(program: &str, args: &[&str], context: &str) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(args);
    match command.output() {
        Ok(output) => {
            let text = combined_output(&output);
            if output.status.success() {
                Ok(text)
            } else {
                Err(format!("{context}: {text}"))
            }
        }
        Err(error) => Err(format!("{context}: {error}")),
    }
}

// ---------------------------------------------------------------------------
// macOS / launchd
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn launchd_plist_path(paths: &SshPaths) -> PathBuf {
    paths
        .home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LAUNCHD_LABEL}.plist"))
}

#[cfg(target_os = "macos")]
fn ensure_launchd(paths: &SshPaths, provider: &OpenSshPaths, socket: &Path) -> AppResult<()> {
    let plist_path = launchd_plist_path(paths);
    let launch_agents = plist_path
        .parent()
        .ok_or_else(|| CommandError::new(codes::AGENT_FAILED, "无法确定 LaunchAgents 目录。"))?;
    fs::create_dir_all(launch_agents).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法创建 LaunchAgents 目录。")
            .detail(error.to_string())
    })?;
    let logs_dir = paths.home.join("Library").join("Logs").join("Keyport");
    fs::create_dir_all(&logs_dir).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法创建 Keyport 日志目录。")
            .detail(error.to_string())
    })?;
    let stdout = logs_dir.join("ssh-agent.log");
    let stderr = logs_dir.join("ssh-agent.err.log");

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{agent}</string>
        <string>-D</string>
        <string>-a</string>
        <string>{socket}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ProcessType</key>
    <string>Background</string>
    <key>StandardOutPath</key>
    <string>{stdout}</string>
    <key>StandardErrorPath</key>
    <string>{stderr}</string>
</dict>
</plist>
"#,
        label = xml_escape(LAUNCHD_LABEL),
        agent = xml_escape(&display_path(&provider.agent)),
        socket = xml_escape(&display_path(socket)),
        stdout = xml_escape(&display_path(&stdout)),
        stderr = xml_escape(&display_path(&stderr)),
    );

    atomic_write(&plist_path, plist.as_bytes(), Some(0o644)).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法写入 ssh-agent LaunchAgent 配置。")
            .detail(error.message)
    })?;

    let uid = current_uid()?;
    let domain = format!("gui/{uid}");
    let service = format!("{domain}/{LAUNCHD_LABEL}");
    let plist_arg = display_path(&plist_path);

    if !agent_responds_raw(provider, socket) {
        remove_stale_socket_raw(provider, socket);
        let mut diagnostics = Vec::new();
        let _ = run_succeeded("launchctl", &["bootout", &service], "unload old agent");
        match run_succeeded(
            "launchctl",
            &["bootstrap", &domain, &plist_arg],
            "launchctl bootstrap",
        ) {
            Ok(text) => {
                if !text.trim().is_empty() {
                    diagnostics.push(text);
                }
            }
            Err(error) => {
                diagnostics.push(error);
                if let Err(legacy) =
                    run_succeeded("launchctl", &["load", "-w", &plist_arg], "launchctl load")
                {
                    diagnostics.push(legacy);
                }
            }
        }
        if let Err(error) = run_succeeded(
            "launchctl",
            &["kickstart", "-k", &service],
            "launchctl kickstart",
        ) {
            diagnostics.push(error);
        }
        if !wait_for_provider(provider, socket, 30) {
            return Err(CommandError::new(
                codes::AGENT_FAILED,
                "launchd 未能启动 Keyport ssh-agent。",
            )
            .hint("请检查 macOS 的 LaunchAgents 权限，或重新登录后重试。")
            .detail(diagnostics.join("\n")));
        }
    } else {
        let _ = run_succeeded(
            "launchctl",
            &["kickstart", "-k", &service],
            "launchctl kickstart",
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn current_uid() -> AppResult<String> {
    let output = Command::new("id").arg("-u").output().map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法获取当前用户 UID。").detail(error.to_string())
    })?;
    if !output.status.success() {
        return Err(
            CommandError::new(codes::AGENT_FAILED, "无法获取当前用户 UID。")
                .detail(combined_output(&output)),
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ---------------------------------------------------------------------------
// Linux / systemd user unit with XDG autostart fallback
// ---------------------------------------------------------------------------

#[cfg(all(unix, not(target_os = "macos")))]
fn systemd_unit_path(paths: &SshPaths) -> PathBuf {
    paths
        .home
        .join(".config")
        .join("systemd")
        .join("user")
        .join(SYSTEMD_UNIT)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_agent_helper(paths: &SshPaths) -> PathBuf {
    paths
        .home
        .join(".local")
        .join("bin")
        .join("keyport-ssh-agent")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_desktop_autostart(paths: &SshPaths) -> PathBuf {
    paths
        .home
        .join(".config")
        .join("autostart")
        .join("dev.keyport.ssh-agent.desktop")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn systemd_unit_enabled(paths: &SshPaths) -> bool {
    if !systemd_unit_path(paths).is_file() || command_on_path("systemctl").is_none() {
        return false;
    }
    Command::new("systemctl")
        .args(["--user", "is-enabled", SYSTEMD_UNIT])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ensure_linux(paths: &SshPaths, provider: &OpenSshPaths, socket: &Path) -> AppResult<()> {
    let unit_path = systemd_unit_path(paths);
    let unit_dir = unit_path
        .parent()
        .ok_or_else(|| CommandError::new(codes::AGENT_FAILED, "无法确定 systemd user 目录。"))?;
    fs::create_dir_all(unit_dir).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法创建 systemd user 目录。")
            .detail(error.to_string())
    })?;

    let unit = format!(
        "[Unit]\nDescription=Keyport SSH agent\n\n[Service]\nType=simple\nExecStart={agent} -D -a {socket}\nRestart=on-failure\nRestartSec=2\n\n[Install]\nWantedBy=default.target\n",
        agent = systemd_quote(&display_path(&provider.agent)),
        socket = systemd_quote(&display_path(socket)),
    );
    atomic_write(&unit_path, unit.as_bytes(), Some(0o644)).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法写入 ssh-agent systemd 单元。")
            .detail(error.message)
    })?;

    let mut diagnostics = Vec::new();
    let mut systemd_ready = false;

    if let Some(systemctl) = command_on_path("systemctl") {
        remove_stale_socket_raw(provider, socket);
        let systemctl = display_path(&systemctl);
        let reload = run_succeeded(&systemctl, &["--user", "daemon-reload"], "daemon-reload");
        let enable = run_succeeded(
            &systemctl,
            &["--user", "enable", SYSTEMD_UNIT],
            "enable agent",
        );
        let restart = run_succeeded(
            &systemctl,
            &["--user", "restart", SYSTEMD_UNIT],
            "restart agent",
        );
        for error in [reload.err(), enable.err(), restart.err()]
            .into_iter()
            .flatten()
        {
            diagnostics.push(error);
        }
        if diagnostics.is_empty() {
            systemd_ready = wait_for_provider(provider, socket, 25);
            if !systemd_ready {
                diagnostics.push("systemd 服务已启动，但 agent socket 仍未就绪。".to_string());
            }
        }
    } else {
        diagnostics.push("systemctl 不可用，使用 XDG Autostart。".to_string());
    }

    if !systemd_ready {
        ensure_linux_fallback(paths, provider, socket, &mut diagnostics)?;
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ensure_linux_fallback(
    paths: &SshPaths,
    provider: &OpenSshPaths,
    socket: &Path,
    diagnostics: &mut Vec<String>,
) -> AppResult<()> {
    let helper_path = linux_agent_helper(paths);
    if let Some(parent) = helper_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::new(codes::AGENT_FAILED, "无法创建 ~/.local/bin。")
                .detail(error.to_string())
        })?;
    }
    let helper = format!(
        "#!/bin/sh\nSOCK={socket}\nAGENT={agent}\nADD={add}\nif [ -S \"$SOCK\" ]; then\n  SSH_AUTH_SOCK=\"$SOCK\" \"$ADD\" -l >/dev/null 2>&1\n  status=$?\n  if [ \"$status\" -ne 2 ]; then\n    exit 0\n  fi\n  rm -f \"$SOCK\"\nfi\n\"$AGENT\" -a \"$SOCK\" >/dev/null 2>&1\n",
        socket = shell_quote(&display_path(socket)),
        agent = shell_quote(&display_path(&provider.agent)),
        add = shell_quote(&display_path(&provider.add)),
    );
    atomic_write(&helper_path, helper.as_bytes(), Some(0o755)).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法写入 ssh-agent 启动脚本。")
            .detail(error.message)
    })?;

    let desktop_path = linux_desktop_autostart(paths);
    if let Some(parent) = desktop_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::new(codes::AGENT_FAILED, "无法创建 XDG Autostart 目录。")
                .detail(error.to_string())
        })?;
    }
    let desktop = format!(
        "[Desktop Entry]\nType=Application\nName=Keyport SSH Agent\nComment=Start the SSH agent used by Keyport SSH\nExec={exec}\nTerminal=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
        exec = desktop_exec_quote(&display_path(&helper_path)),
    );
    atomic_write(&desktop_path, desktop.as_bytes(), Some(0o644)).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法写入 XDG Autostart 配置。")
            .detail(error.message)
    })?;

    let mut helper_command = Command::new(&helper_path);
    let output = run_output(&mut helper_command, "无法启动 ssh-agent 启动脚本。")?;
    if !output.status.success() {
        diagnostics.push(combined_output(&output));
    }
    if !wait_for_provider(provider, socket, 30) {
        return Err(
            CommandError::new(codes::AGENT_FAILED, "ssh-agent 未能在当前会话中启动。")
                .hint("请检查 OpenSSH 客户端是否完整，以及 ~/.ssh 是否可写。")
                .detail(diagnostics.join("\n")),
        );
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn systemd_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
    )
}

#[cfg(all(unix, not(target_os = "macos")))]
fn desktop_exec_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
    )
}

// ---------------------------------------------------------------------------
// Shared Unix helpers
// ---------------------------------------------------------------------------

fn write_shell_integration(paths: &SshPaths, socket: &Path) -> AppResult<()> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("sh")
        .to_ascii_lowercase();

    if shell_name == "fish" {
        let fish_path = paths
            .home
            .join(".config")
            .join("fish")
            .join("conf.d")
            .join("keyport-ssh-agent.fish");
        if let Some(parent) = fish_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                CommandError::new(codes::AGENT_FAILED, "无法创建 fish 配置目录。")
                    .detail(error.to_string())
            })?;
        }
        let block = format!(
            "# >>> keyport ssh-agent >>>\nif not set -q SSH_AUTH_SOCK\n    if test -S '{socket}'\n        set -gx SSH_AUTH_SOCK '{socket}'\n    end\nend\n# <<< keyport ssh-agent <<<\n",
            socket = fish_escape(&display_path(socket)),
        );
        upsert_shell_block(&fish_path, &block)?;
        return Ok(());
    }

    let rc_path = match shell_name.as_str() {
        "zsh" => paths.home.join(".zshrc"),
        "bash" => paths.home.join(".bashrc"),
        _ => paths.home.join(".profile"),
    };
    let block = format!(
        "# >>> keyport ssh-agent >>>\nif [ -S {socket} ] && [ -z \"${{SSH_AUTH_SOCK:-}}\" ]; then\n    export SSH_AUTH_SOCK={socket}\nfi\n# <<< keyport ssh-agent <<<\n",
        socket = shell_quote(&display_path(socket)),
    );
    upsert_shell_block(&rc_path, &block)?;
    Ok(())
}

fn upsert_shell_block(path: &Path, block: &str) -> AppResult<()> {
    let begin = "# >>> keyport ssh-agent >>>";
    let end = "# <<< keyport ssh-agent <<<";
    let existing = if path.exists() {
        fs::read_to_string(path).map_err(|error| {
            CommandError::new(codes::AGENT_FAILED, "无法读取 shell 配置文件。")
                .detail(format!("{}: {error}", display_path(path)))
        })?
    } else {
        String::new()
    };

    let lines: Vec<&str> = existing.lines().collect();
    let begin_index = lines.iter().position(|line| line.trim() == begin);
    let end_index = lines.iter().position(|line| line.trim() == end);
    let updated = match (begin_index, end_index) {
        (Some(start), Some(finish)) if finish >= start => {
            let mut output = Vec::new();
            output.extend_from_slice(&lines[..start]);
            output.push(block.trim_end_matches('\n'));
            output.extend_from_slice(&lines[finish + 1..]);
            let mut text = output.join("\n");
            text.push('\n');
            text
        }
        (None, None) => {
            let mut text = existing;
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(block);
            text
        }
        _ => {
            return Err(CommandError::new(
                codes::AGENT_FAILED,
                "shell 配置中的 Keyport 标记不完整，未自动修改。",
            )
            .hint(format!(
                "请检查 {} 中的 keyport ssh-agent 标记。",
                display_path(path)
            )));
        }
    };

    let mode = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions().mode() & 0o777)
        .unwrap_or(0o644);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CommandError::new(codes::AGENT_FAILED, "无法创建 shell 配置目录。")
                .detail(error.to_string())
        })?;
    }
    atomic_write(path, updated.as_bytes(), Some(mode)).map_err(|error| {
        CommandError::new(codes::AGENT_FAILED, "无法写入 shell 配置。").detail(error.message)
    })
}

fn fish_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn agent_responds_raw(provider: &OpenSshPaths, socket: &Path) -> bool {
    if !is_socket(socket) {
        return false;
    }
    Command::new(&provider.add)
        .arg("-l")
        .env("SSH_AUTH_SOCK", socket)
        .output()
        .map(|output| matches!(output.status.code(), Some(0) | Some(1)))
        .unwrap_or(false)
}

fn remove_stale_socket_raw(provider: &OpenSshPaths, socket: &Path) {
    if is_socket(socket) && !agent_responds_raw(provider, socket) {
        let _ = fs::remove_file(socket);
    }
}

fn wait_for_provider(provider: &OpenSshPaths, socket: &Path, attempts: usize) -> bool {
    for _ in 0..attempts {
        if agent_responds_raw(provider, socket) {
            return true;
        }
        thread::sleep(Duration::from_millis(200));
    }
    agent_responds_raw(provider, socket)
}
