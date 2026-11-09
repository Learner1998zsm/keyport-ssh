use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::error::{codes, AppResult, CommandError};
use crate::model::AgentStatus;
use crate::openssh::OpenSsh;
use crate::util::{combined_output, run_output};

const SERVICE: &str = "ssh-agent";

#[derive(Debug, Default)]
struct ServiceState {
    running: bool,
    autostart: bool,
    detail: Option<String>,
}

pub fn status(_openssh: &OpenSsh) -> AgentStatus {
    match read_service_state() {
        Ok(state) => build_status(state.running, state.autostart, state.detail, None),
        Err(error) => AgentStatus {
            ready: false,
            running: false,
            autostart: false,
            socket: None,
            manager: "Windows Service".to_string(),
            summary: "无法读取 OpenSSH Authentication Agent 服务状态。".to_string(),
            detail: error.detail.or(Some(error.message)),
        },
    }
}

pub fn ensure(_openssh: &OpenSsh) -> AppResult<AgentStatus> {
    let before = read_service_state()?;
    let mut diagnostics: Vec<String> = Vec::new();
    let mut elevation_needed = false;

    if !before.autostart {
        let mut command = sc_command();
        command.args(["config", SERVICE, "start=", "auto"]);
        let output = run_output(&mut command, "无法配置 ssh-agent 服务。")?;
        diagnostics.push(combined_output(&output));
        if !output.status.success() {
            elevation_needed = true;
        }
    }

    if !before.running {
        let mut command = sc_command();
        command.arg("start").arg(SERVICE);
        let output = run_output(&mut command, "无法启动 ssh-agent 服务。")?;
        diagnostics.push(combined_output(&output));
        if !output.status.success() {
            elevation_needed = true;
        }
    }

    if elevation_needed {
        diagnostics.push("需要提升权限，正在请求管理员授权…".to_string());
        elevate_service_setup()?;
    }

    let mut last_state = read_service_state()?;
    for _ in 0..20 {
        if last_state.running && last_state.autostart {
            break;
        }
        thread::sleep(Duration::from_millis(250));
        last_state = read_service_state()?;
    }

    let detail = diagnostics
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let status = build_status(
        last_state.running,
        last_state.autostart,
        last_state.detail,
        if detail.is_empty() {
            None
        } else {
            Some(detail)
        },
    );

    if !last_state.running || !last_state.autostart {
        return Err(CommandError::new(
            codes::AGENT_FAILED,
            "ssh-agent 服务尚未就绪。",
        )
        .hint("请确认 Windows 的 OpenSSH Authentication Agent 服务已安装，并在 UAC 提示中允许 Keyport 修改服务启动类型。")
        .detail(status.detail.clone().unwrap_or_default()));
    }
    Ok(status)
}

fn build_status(
    running: bool,
    autostart: bool,
    raw_detail: Option<String>,
    extra_detail: Option<String>,
) -> AgentStatus {
    let summary = match (running, autostart) {
        (true, true) => "ssh-agent 服务运行中，已设为开机自动启动。",
        (true, false) => "ssh-agent 服务运行中，但尚未设为自动启动。",
        (false, true) => "ssh-agent 服务已设为自动启动，但当前未运行。",
        (false, false) => "ssh-agent 服务当前未运行，也未设为自动启动。",
    };
    let detail = [raw_detail, extra_detail]
        .into_iter()
        .flatten()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    AgentStatus {
        ready: running && autostart,
        running,
        autostart,
        socket: None,
        manager: "Windows Service".to_string(),
        summary: summary.to_string(),
        detail: if detail.is_empty() {
            None
        } else {
            Some(detail)
        },
    }
}

fn read_service_state() -> AppResult<ServiceState> {
    let mut query = sc_command();
    query.args(["query", SERVICE]);
    let query_output = run_output(&mut query, "无法查询 ssh-agent 服务。")?;
    let query_text = combined_output(&query_output);
    if !query_output.status.success() {
        return Err(CommandError::new(
            codes::AGENT_FAILED,
            "未找到 Windows 的 OpenSSH Authentication Agent 服务。",
        )
        .hint("请确认已安装 OpenSSH 客户端（Windows 可选功能），然后重新检查环境。")
        .detail(query_text));
    }
    let running = query_text.to_ascii_uppercase().contains("RUNNING");

    let mut config = sc_command();
    config.args(["qc", SERVICE]);
    let config_output = run_output(&mut config, "无法读取 ssh-agent 服务启动类型。")?;
    let config_text = combined_output(&config_output);
    let autostart =
        config_output.status.success() && config_text.to_ascii_uppercase().contains("AUTO_START");

    let detail = if config_output.status.success() {
        None
    } else {
        Some(config_text)
    };
    Ok(ServiceState {
        running,
        autostart,
        detail,
    })
}

fn sc_command() -> Command {
    let path = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| root.join("System32").join("sc.exe"))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("sc.exe"));
    Command::new(path)
}

fn elevate_service_setup() -> AppResult<()> {
    let powershell = powershell_path().ok_or_else(|| {
        CommandError::new(
            codes::AGENT_FAILED,
            "找不到 Windows PowerShell，无法请求管理员权限。",
        )
    })?;

    let script = r#"$ErrorActionPreference = 'Stop'
$inner = @'
$ErrorActionPreference = 'Continue'
$sc = Join-Path $env:SystemRoot 'System32\sc.exe'
& $sc config ssh-agent start= auto | Out-Null
& $sc start ssh-agent | Out-Null
Start-Sleep -Milliseconds 700
'@
$encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($inner))
$process = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-EncodedCommand',$encoded) -Verb RunAs -Wait -PassThru
exit $process.ExitCode
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
    let output = run_output(&mut command, "无法发起管理员授权。")?;
    if !output.status.success() {
        return Err(CommandError::new(
            codes::AGENT_FAILED,
            "管理员授权被取消，ssh-agent 服务未能配置。",
        )
        .hint("再次点击“开始配置”即可重试；UAC 提示出现时请选择“是”。")
        .detail(combined_output(&output)));
    }
    Ok(())
}

fn powershell_path() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("SystemRoot") {
        let path = PathBuf::from(root)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if path.is_file() {
            return Some(path);
        }
    }
    Some(PathBuf::from("powershell.exe"))
}
