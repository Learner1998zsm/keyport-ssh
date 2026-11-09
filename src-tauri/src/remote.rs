use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use russh::client;
use russh::keys::known_hosts::{known_host_keys_path, learn_known_hosts_path};
use russh::keys::ssh_key::PublicKey;
use russh::keys::PublicKeyOrCertificate;
use russh::{ChannelMsg, Disconnect};
use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{codes, AppResult, CommandError};
use crate::model::{HostProbe, HostTrustStatus, TestResult};
use crate::openssh::OpenSsh;
use crate::paths::SshPaths;
use crate::util::{shell_quote, truncate};
use crate::validation::ValidatedConnection;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const AUTH_TIMEOUT: Duration = Duration::from_secs(35);
const REMOTE_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_CAPTURE_BYTES: usize = 64 * 1024;
const KEYPORT_OK_MARKER: &str = "KEYPORT_OK";

#[derive(Debug, Clone)]
struct CapturedKey {
    key: PublicKey,
    fingerprint: String,
    is_certificate: bool,
}

struct KeyCapture {
    captured: Arc<Mutex<Option<CapturedKey>>>,
    expected_fingerprint: Option<String>,
}

impl client::Handler for KeyCapture {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let is_certificate = server_public_key.certificate().is_some();
        let key = server_public_key.public_key();
        let fingerprint = key_fingerprint(&key);
        let fingerprint_matches = match &self.expected_fingerprint {
            Some(expected) => expected == &fingerprint,
            None => true,
        };

        if let Ok(mut captured) = self.captured.lock() {
            *captured = Some(CapturedKey {
                key,
                fingerprint,
                is_certificate,
            });
        }
        // Host certificates are not handled yet. Refusing before authentication
        // guarantees the password is never sent to a host we cannot pin in the
        // standard ~/.ssh/known_hosts format.
        Ok(fingerprint_matches && !is_certificate)
    }
}

fn key_fingerprint(key: &PublicKey) -> String {
    key.fingerprint(russh::keys::HashAlg::Sha256).to_string()
}

fn captured_snapshot(state: &Arc<Mutex<Option<CapturedKey>>>) -> Option<CapturedKey> {
    state.lock().ok().and_then(|guard| guard.clone())
}

async fn connect_capturing(
    host: &str,
    port: u16,
    expected_fingerprint: Option<String>,
) -> AppResult<(client::Handle<KeyCapture>, CapturedKey)> {
    let state = Arc::new(Mutex::new(None));
    let handler = KeyCapture {
        captured: state.clone(),
        expected_fingerprint: expected_fingerprint.clone(),
    };
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(25)),
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 2,
        nodelay: true,
        ..Default::default()
    });

    let result = timeout(
        CONNECT_TIMEOUT,
        client::connect(config, (host, port), handler),
    )
    .await;

    match result {
        Ok(Ok(session)) => {
            let captured = captured_snapshot(&state).ok_or_else(|| {
                CommandError::new(
                    codes::PROBE_FAILED,
                    "SSH 握手完成，但没有取得服务器主机密钥。",
                )
            })?;
            if captured.is_certificate {
                return Err(CommandError::new(
                    codes::HOST_KEY_UNSUPPORTED,
                    "该服务器使用 SSH 主机证书，当前版本暂不支持自动记录此类主机密钥。",
                )
                .hint("请先在终端使用 ssh 手动连接并接受主机证书，再使用 Keyport。"));
            }
            Ok((session, captured))
        }
        Ok(Err(error)) => {
            let captured = captured_snapshot(&state);
            Err(map_connect_error(
                captured,
                expected_fingerprint.as_deref(),
                error,
            ))
        }
        Err(_) => Err(
            CommandError::new(codes::PROBE_FAILED, "连接服务器超时（20 秒）。")
                .hint("请检查 IP、端口、网络连通性和防火墙设置。"),
        ),
    }
}

fn map_connect_error(
    captured: Option<CapturedKey>,
    expected_fingerprint: Option<&str>,
    error: russh::Error,
) -> CommandError {
    if let Some(captured) = captured {
        if captured.is_certificate {
            return CommandError::new(
                codes::HOST_KEY_UNSUPPORTED,
                "该服务器使用 SSH 主机证书，当前版本暂不支持自动记录此类主机密钥。",
            )
            .hint("请先在终端使用 ssh 手动连接并接受主机证书，再使用 Keyport。");
        }
        let expected = expected_fingerprint.unwrap_or_default();
        if !expected.is_empty() && captured.fingerprint != expected {
            return CommandError::new(
                codes::HOST_KEY_CHANGED,
                "服务器主机密钥与刚才探测到的指纹不一致，连接已中止。",
            )
            .hint("可能存在中间人攻击。请确认服务器指纹后重新开始配置。")
            .detail(format!(
                "expected: {expected}\nactual:   {}",
                captured.fingerprint
            ));
        }
    }
    CommandError::new(codes::PROBE_FAILED, "无法建立 SSH 连接。")
        .hint("请检查服务器地址、端口和网络连接。")
        .detail(error.to_string())
}

fn trust_status(
    host: &str,
    port: u16,
    key: &PublicKey,
    known_hosts_path: &std::path::Path,
) -> AppResult<(HostTrustStatus, Vec<String>)> {
    let recorded = known_host_keys_path(host, port, known_hosts_path).map_err(|error| {
        CommandError::new(codes::INTERNAL, "无法读取 known_hosts 文件。").detail(error.to_string())
    })?;
    if recorded.is_empty() {
        return Ok((HostTrustStatus::Unknown, Vec::new()));
    }
    let fingerprints = recorded
        .iter()
        .map(|(_, recorded_key)| key_fingerprint(recorded_key))
        .collect::<Vec<_>>();
    if recorded
        .iter()
        .any(|(_, recorded_key)| recorded_key.key_data() == key.key_data())
    {
        Ok((HostTrustStatus::Trusted, fingerprints))
    } else {
        Ok((HostTrustStatus::Changed, fingerprints))
    }
}

pub async fn probe_host(host: &str, port: u16, paths: &SshPaths) -> AppResult<HostProbe> {
    let (session, captured) = connect_capturing(host, port, None).await?;
    let _ = session
        .disconnect(Disconnect::ByApplication, "keyport probe complete", "en")
        .await;
    let (status, recorded_fingerprints) =
        trust_status(host, port, &captured.key, &paths.known_hosts_file)?;
    let host_token = if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    };
    Ok(HostProbe {
        host: host.to_string(),
        port,
        host_token,
        fingerprint: captured.fingerprint,
        algorithm: captured.key.algorithm().to_string(),
        status,
        recorded_fingerprints,
        known_hosts_path: paths.display_known_hosts(),
    })
}

pub struct RemoteInstallOutcome {
    pub host_key_learned: bool,
}

pub async fn install_public_key(
    connection: &ValidatedConnection,
    expected_fingerprint: &str,
    trust_host: bool,
    password: &str,
    public_key: &str,
    paths: &SshPaths,
) -> AppResult<RemoteInstallOutcome> {
    if password.is_empty() {
        return Err(CommandError::new(
            codes::AUTH_FAILED,
            "服务器密码不能为空。",
        ));
    }

    let (mut session, captured) = connect_capturing(
        &connection.host,
        connection.port,
        Some(expected_fingerprint.to_string()),
    )
    .await?;

    // Verify known_hosts and trust expectations before authentication. This is
    // the last point where we can avoid sending the one-time password.
    let (status, recorded_fingerprints) = match trust_status(
        &connection.host,
        connection.port,
        &captured.key,
        &paths.known_hosts_file,
    ) {
        Ok(value) => value,
        Err(error) => {
            let _ = session
                .disconnect(Disconnect::ByApplication, "keyport aborted", "en")
                .await;
            return Err(error);
        }
    };

    if status == HostTrustStatus::Changed {
        let _ = session
            .disconnect(Disconnect::ByApplication, "keyport host key changed", "en")
            .await;
        return Err(CommandError::new(
            codes::HOST_KEY_CHANGED,
            "known_hosts 中保存的主机密钥与当前服务器不一致，已中止。",
        )
        .hint("这可能是服务器重装，也可能是中间人攻击。确认安全后可用 ssh-keygen -R 移除旧记录并重新配置。")
        .detail(format!(
            "current:  {}\nrecorded: {}",
            captured.fingerprint,
            recorded_fingerprints.join(", ")
        )));
    }
    if status == HostTrustStatus::Unknown && !trust_host {
        let _ = session
            .disconnect(Disconnect::ByApplication, "keyport untrusted host", "en")
            .await;
        return Err(CommandError::new(
            codes::HOST_KEY_UNTRUSTED,
            "尚未确认服务器主机指纹，连接已中止。",
        )
        .hint("请重新打开密码窗口，并勾选确认主机指纹。"));
    }

    if let Err(error) = authenticate_password(&mut session, &connection.user, password).await {
        let _ = session
            .disconnect(Disconnect::ByApplication, "keyport auth failed", "en")
            .await;
        return Err(error);
    }

    let command = authorized_keys_command(public_key);
    let remote = timeout(REMOTE_TIMEOUT, run_remote_command(&session, &command)).await;
    let remote_result = match remote {
        Ok(result) => result,
        Err(_) => {
            let _ = session
                .disconnect(Disconnect::ByApplication, "keyport timeout", "en")
                .await;
            return Err(CommandError::new(
                codes::REMOTE_FAILED,
                "服务器在 45 秒内没有完成公钥安装命令。",
            )
            .hint("请检查服务器磁盘权限与 shell 配置。"));
        }
    };
    match remote_result {
        Ok(result) if result.exit_code == Some(0) => {}
        Ok(result) => {
            let _ = session
                .disconnect(Disconnect::ByApplication, "keyport remote failed", "en")
                .await;
            return Err(CommandError::new(
                codes::REMOTE_FAILED,
                "公钥安装命令在服务器上执行失败。",
            )
            .hint("请确认登录用户拥有 ~/.ssh/authorized_keys 的写权限，且服务器使用 POSIX shell。")
            .detail(format!(
                "exit code: {:?}\nstdout:\n{}\nstderr:\n{}",
                result.exit_code,
                truncate(&result.stdout, 3000),
                truncate(&result.stderr, 3000)
            )));
        }
        Err(error) => {
            let _ = session
                .disconnect(Disconnect::ByApplication, "keyport remote error", "en")
                .await;
            return Err(error);
        }
    }

    let host_key_learned = status == HostTrustStatus::Unknown;
    if host_key_learned {
        learn_host_key(&connection.host, connection.port, &captured.key, paths)?;
    }

    let _ = session
        .disconnect(Disconnect::ByApplication, "keyport finished", "en")
        .await;
    Ok(RemoteInstallOutcome { host_key_learned })
}

fn learn_host_key(host: &str, port: u16, key: &PublicKey, paths: &SshPaths) -> AppResult<()> {
    learn_known_hosts_path(host, port, key, &paths.known_hosts_file).map_err(|error| {
        CommandError::new(
            codes::KNOWN_HOSTS_FAILED,
            "公钥已送达服务器，但无法把主机密钥写入 known_hosts。",
        )
        .hint("请检查 ~/.ssh/known_hosts 的文件权限后重试。")
        .detail(error.to_string())
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            &paths.known_hosts_file,
            std::fs::Permissions::from_mode(0o644),
        );
    }
    Ok(())
}

async fn authenticate_password(
    session: &mut client::Handle<KeyCapture>,
    user: &str,
    password: &str,
) -> AppResult<()> {
    let password_result =
        timeout(AUTH_TIMEOUT, session.authenticate_password(user, password)).await;

    let password_succeeded = match password_result {
        Ok(Ok(result)) => result.success(),
        Ok(Err(_)) => false,
        Err(_) => {
            return Err(CommandError::new(
                codes::AUTH_FAILED,
                "服务器密码认证超时（35 秒）。",
            ))
        }
    };
    if password_succeeded {
        return Ok(());
    }

    // Many Linux distributions expose password login through keyboard-interactive
    // (PAM) rather than the SSH "password" method. Try one fallback round before
    // declaring failure. Responses are never logged.
    let start = timeout(
        AUTH_TIMEOUT,
        session.authenticate_keyboard_interactive_start(user, None::<String>),
    )
    .await
    .map_err(|_| CommandError::new(codes::AUTH_FAILED, "服务器交互式认证超时（35 秒）。"))?
    .map_err(|error| {
        CommandError::new(codes::AUTH_FAILED, "服务器拒绝了密码认证。").detail(error.to_string())
    })?;

    let mut response = start;
    for _ in 0..4 {
        match response {
            russh::client::KeyboardInteractiveAuthResponse::Success => return Ok(()),
            russh::client::KeyboardInteractiveAuthResponse::Failure { .. } => {
                return Err(auth_error());
            }
            russh::client::KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let responses = prompts
                    .iter()
                    .map(|prompt| {
                        if prompt.echo {
                            String::new()
                        } else {
                            password.to_string()
                        }
                    })
                    .collect();
                response = timeout(
                    AUTH_TIMEOUT,
                    session.authenticate_keyboard_interactive_respond(responses),
                )
                .await
                .map_err(|_| CommandError::new(codes::AUTH_FAILED, "服务器交互式认证超时。"))?
                .map_err(|error| {
                    CommandError::new(codes::AUTH_FAILED, "服务器拒绝了密码认证。")
                        .detail(error.to_string())
                })?;
            }
        }
    }
    Err(auth_error())
}

fn auth_error() -> CommandError {
    CommandError::new(codes::AUTH_FAILED, "服务器密码认证失败。")
        .hint("请确认登录用户和密码；服务器也可能关闭了密码登录。密码不会被保存，可直接重试。")
}

struct RemoteCommandResult {
    stdout: String,
    stderr: String,
    exit_code: Option<u32>,
}

async fn run_remote_command(
    session: &client::Handle<KeyCapture>,
    command: &str,
) -> AppResult<RemoteCommandResult> {
    let mut channel = session.channel_open_session().await.map_err(|error| {
        CommandError::new(codes::REMOTE_FAILED, "无法打开 SSH 会话通道。").detail(error.to_string())
    })?;
    channel
        .exec(true, command.as_bytes().to_vec())
        .await
        .map_err(|error| {
            CommandError::new(codes::REMOTE_FAILED, "无法在服务器上执行公钥安装命令。")
                .detail(error.to_string())
        })?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_code = None;
    loop {
        match channel.wait().await {
            Some(ChannelMsg::Data { data }) => append_capped(&mut stdout, &data),
            Some(ChannelMsg::ExtendedData { data, .. }) => append_capped(&mut stderr, &data),
            Some(ChannelMsg::ExitStatus { exit_status }) => exit_code = Some(exit_status),
            Some(ChannelMsg::Close) | None => break,
            _ => {}
        }
    }

    Ok(RemoteCommandResult {
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
        exit_code,
    })
}

fn append_capped(buffer: &mut Vec<u8>, data: &[u8]) {
    let remaining = MAX_CAPTURE_BYTES.saturating_sub(buffer.len());
    if remaining > 0 {
        buffer.extend_from_slice(&data[..data.len().min(remaining)]);
    }
}

pub fn authorized_keys_command(public_key: &str) -> String {
    let quoted_key = shell_quote(public_key.trim());
    format!(
        "set -e; umask 077; mkdir -p \"$HOME/.ssh\"; touch \"$HOME/.ssh/authorized_keys\"; chmod 700 \"$HOME/.ssh\"; chmod 600 \"$HOME/.ssh/authorized_keys\"; key={quoted_key}; if ! grep -qxF \"$key\" \"$HOME/.ssh/authorized_keys\"; then printf '%s\\n' \"$key\" >> \"$HOME/.ssh/authorized_keys\"; fi; printf '{marker}\\n'",
        marker = KEYPORT_OK_MARKER
    )
}

pub async fn test_connection(openssh: &OpenSsh, alias: &str) -> AppResult<TestResult> {
    let provider = openssh.require()?;
    let start = Instant::now();
    let mut command = Command::new(&provider.ssh);
    command.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=15",
        "-o",
        "NumberOfPasswordPrompts=0",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "LogLevel=ERROR",
        alias,
        "true",
    ]);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env_remove("SSH_ASKPASS")
        .env("SSH_ASKPASS_REQUIRE", "never");

    let child = command.spawn().map_err(|error| {
        CommandError::new(codes::TEST_FAILED, "无法启动 ssh 连接测试。").detail(error.to_string())
    })?;

    let output = timeout(Duration::from_secs(40), child.wait_with_output()).await;
    let duration_ms = start.elapsed().as_millis() as u64;
    let output = match output {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => {
            return Ok(TestResult {
                ok: false,
                message: "ssh 测试进程运行失败。".to_string(),
                stdout: String::new(),
                stderr: truncate(&error.to_string(), 3000),
                exit_code: None,
                duration_ms,
            });
        }
        Err(_) => {
            return Ok(TestResult {
                ok: false,
                message: "连接测试超时（40 秒）。".to_string(),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms,
            });
        }
    };

    let stdout = truncate(&String::from_utf8_lossy(&output.stdout), 3000);
    let stderr = truncate(&String::from_utf8_lossy(&output.stderr), 3000);
    let ok = output.status.success();
    let message = if ok {
        "使用新密钥成功登录服务器，连接测试通过。".to_string()
    } else {
        classify_ssh_failure(&stdout, &stderr)
    };
    Ok(TestResult {
        ok,
        message,
        stdout,
        stderr,
        exit_code: output.status.code(),
        duration_ms,
    })
}

fn classify_ssh_failure(stdout: &str, stderr: &str) -> String {
    let combined = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if combined.contains("permission denied") || combined.contains("authentication failed") {
        "认证失败：服务器拒绝了刚安装的公钥。".to_string()
    } else if combined.contains("host key verification failed") {
        "主机密钥校验失败。请重新探测服务器并确认指纹。".to_string()
    } else if combined.contains("connection timed out")
        || combined.contains("timed out")
        || combined.contains("connection refused")
        || combined.contains("could not resolve")
    {
        "网络连接失败。请检查服务器地址、端口和网络。".to_string()
    } else if combined.contains("bad configuration option") || combined.contains("config") {
        "SSH 配置或 known_hosts 校验失败。".to_string()
    } else {
        "SSH 连接测试失败。".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_command_quotes_the_public_key() {
        let key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExample keyport:test's";
        let command = authorized_keys_command(key);
        assert!(command.contains("set -e"));
        assert!(command.contains("chmod 600"));
        assert!(command.contains("grep -qxF"));
        assert!(command.contains("KEYPORT_OK"));
        assert!(
            command.contains("'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExample keyport:test'\\''s'")
        );
    }

    #[test]
    fn failure_classifier_is_actionable() {
        assert!(classify_ssh_failure("", "Permission denied (publickey).").contains("认证失败"));
        assert!(classify_ssh_failure("", "Host key verification failed.").contains("主机密钥"));
    }
}
