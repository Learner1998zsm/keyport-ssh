use std::path::Path;
use std::process::Command;

use crate::error::{codes, AppResult, CommandError};
use crate::model::AgentStatus;
use crate::openssh::OpenSsh;
use crate::paths::display_path;
use crate::util::{combined_output, run_output};

#[cfg(windows)]
#[path = "agent_windows.rs"]
mod platform;
#[cfg(unix)]
#[path = "agent_unix.rs"]
mod platform;

pub fn status(openssh: &OpenSsh) -> AgentStatus {
    platform::status(openssh)
}

/// Ensure the platform agent is running and configured for automatic start, then
/// load the freshly generated identity and verify the agent can see it.
pub fn ensure(openssh: &OpenSsh, key_path: &Path, public_key: &str) -> AppResult<AgentStatus> {
    let status = platform::ensure(openssh)?;
    add_key(openssh, &status, key_path, public_key)?;
    Ok(status)
}

fn add_key(
    openssh: &OpenSsh,
    status: &AgentStatus,
    key_path: &Path,
    public_key: &str,
) -> AppResult<()> {
    let provider = openssh.require()?;
    let expected_blob = public_key.split_whitespace().nth(1).ok_or_else(|| {
        CommandError::new(codes::AGENT_FAILED, "公钥格式无效，无法加入 ssh-agent。")
    })?;

    let mut add = Command::new(&provider.add);
    add.arg(key_path);
    apply_agent_env(&mut add, status)?;
    let add_output = run_output(&mut add, "无法运行 ssh-add。")?;
    if !add_output.status.success() {
        return Err(CommandError::new(
            codes::AGENT_FAILED,
            "密钥已生成，但未能加入 ssh-agent。",
        )
        .hint("请检查 ssh-agent 是否正在运行；在 Windows 上还需要确认 OpenSSH Authentication Agent 服务可用。")
        .detail(combined_output(&add_output)));
    }

    let mut list = Command::new(&provider.add);
    list.arg("-L");
    apply_agent_env(&mut list, status)?;
    let list_output = run_output(&mut list, "无法查询 ssh-agent 中的密钥。")?;
    let listing = String::from_utf8_lossy(&list_output.stdout);
    if !list_output.status.success() || !listing.contains(expected_blob) {
        return Err(CommandError::new(
            codes::AGENT_FAILED,
            "ssh-add 已执行，但在 ssh-agent 中没有找到刚生成的密钥。",
        )
        .hint(format!(
            "请手动运行 `ssh-add {}` 并查看 agent 状态。",
            display_path(key_path)
        ))
        .detail(combined_output(&list_output)));
    }
    Ok(())
}

fn apply_agent_env(command: &mut Command, status: &AgentStatus) -> AppResult<()> {
    #[cfg(unix)]
    {
        let socket = status
            .socket
            .as_ref()
            .ok_or_else(|| CommandError::new(codes::AGENT_FAILED, "ssh-agent socket 不可用。"))?;
        command.env("SSH_AUTH_SOCK", socket);
    }
    #[cfg(windows)]
    {
        // On Windows, ssh-add talks to the OpenSSH Authentication Agent service.
        // An inherited SSH_AUTH_SOCK (for example from Git Bash) would point at
        // a different agent, so remove it for these commands.
        command.env_remove("SSH_AUTH_SOCK");
        let _ = status;
    }
    Ok(())
}
