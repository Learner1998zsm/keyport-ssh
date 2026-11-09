use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use russh::keys::ssh_key::PublicKey;
use russh::keys::HashAlg;

use crate::error::{codes, AppResult, CommandError};
use crate::openssh::OpenSsh;
use crate::paths::{display_path, SshPaths};
use crate::util::{combined_output, run_output};

#[derive(Debug, Clone)]
pub struct GeneratedKey {
    pub private_path: PathBuf,
    pub public_key: String,
    pub fingerprint: String,
    pub algorithm: String,
    pub reused: bool,
}

pub fn prepare_key(
    openssh: &OpenSsh,
    paths: &SshPaths,
    alias: &str,
    key_filename: &str,
    reuse_existing: bool,
) -> AppResult<GeneratedKey> {
    paths.ensure_ssh_dir()?;
    let private_path = paths.key_path(key_filename);
    let public_path = paths.public_key_path(key_filename);

    let private_exists = private_path.exists();
    let public_exists = public_path.exists();

    if !private_exists && public_exists {
        return Err(
            CommandError::new(codes::KEY_EXISTS, "发现孤立公钥文件，但对应的私钥不存在。").hint(
                format!(
                    "请检查或移走 {}，或者换一个连接名称。",
                    display_path(&public_path)
                ),
            ),
        );
    }

    let reused = private_exists;
    if reused && !reuse_existing {
        return Err(CommandError::new(
            codes::KEY_EXISTS,
            "同名连接密钥已经存在，Keyport 不会覆盖它。",
        )
        .hint("选择“复用现有密钥”继续，或换一个连接名称重新生成。")
        .detail(display_path(&private_path)));
    }

    if !reused {
        let provider = openssh.require()?;
        let comment = format!("keyport:{alias}");
        let private_path_arg = display_path(&private_path);
        let mut command = Command::new(&provider.keygen);
        command.args([
            "-q",
            "-t",
            "ed25519",
            "-f",
            &private_path_arg,
            "-N",
            "",
            "-C",
            &comment,
        ]);
        let output = run_output(&mut command, "无法启动 ssh-keygen。")?;
        if !output.status.success() {
            return Err(
                CommandError::new(codes::KEY_FAILED, "ssh-keygen 未能生成密钥。")
                    .detail(combined_output(&output)),
            );
        }
    }

    if !public_path.is_file() {
        derive_public_key(openssh, &private_path, &public_path, alias)?;
    }

    apply_key_permissions(&private_path, &public_path)?;
    let public_key = read_public_key(&public_path)?;
    if !public_key.starts_with("ssh-ed25519 ") {
        return Err(CommandError::new(
            codes::KEY_FAILED,
            "该连接名称对应的现有密钥不是 Ed25519 密钥。",
        )
        .hint("请换一个连接名称，让 Keyport 生成新的 Ed25519 密钥。"));
    }

    let parsed = PublicKey::from_openssh(&public_key).map_err(|err| {
        CommandError::new(codes::KEY_FAILED, "无法解析生成的公钥。").detail(err.to_string())
    })?;
    let fingerprint = parsed.fingerprint(HashAlg::Sha256).to_string();
    let public_key = parsed.to_openssh().map_err(|err| {
        CommandError::new(codes::KEY_FAILED, "无法规范化生成的公钥。").detail(err.to_string())
    })?;

    Ok(GeneratedKey {
        private_path,
        public_key,
        fingerprint,
        algorithm: parsed.algorithm().to_string(),
        reused,
    })
}

/// Load an existing Keyport key without ever generating a new one. Used by the
/// finish/install step so the backend never trusts a key path supplied by the UI.
pub fn load_existing_key(
    openssh: &OpenSsh,
    paths: &SshPaths,
    alias: &str,
    key_filename: &str,
) -> AppResult<GeneratedKey> {
    let private_path = paths.key_path(key_filename);
    if !private_path.is_file() {
        return Err(CommandError::new(
            codes::KEY_FAILED,
            "找不到准备步骤生成的私钥。请重新开始配置。",
        )
        .detail(display_path(&private_path)));
    }
    prepare_key(openssh, paths, alias, key_filename, true)
}

fn derive_public_key(
    openssh: &OpenSsh,
    private_path: &Path,
    public_path: &Path,
    alias: &str,
) -> AppResult<()> {
    let provider = openssh.require()?;
    let private_path_arg = display_path(private_path);
    let mut command = Command::new(&provider.keygen);
    command.args(["-y", "-f", &private_path_arg]);
    let output = run_output(&mut command, "无法读取现有私钥。")?;
    if !output.status.success() {
        return Err(CommandError::new(
            codes::KEY_FAILED,
            "现有私钥没有对应的公钥，且无法从私钥推导公钥。密钥可能设置了口令。",
        )
        .hint("请换一个连接名称，或手动运行 ssh-keygen -y -f <私钥路径>。")
        .detail(combined_output(&output)));
    }
    let derived = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if derived.is_empty() {
        return Err(CommandError::new(
            codes::KEY_FAILED,
            "ssh-keygen 未能推导公钥。",
        ));
    }
    fs::write(public_path, format!("{derived} keyport:{alias}\n")).map_err(|err| {
        CommandError::new(codes::KEY_FAILED, "无法写入公钥文件。").detail(err.to_string())
    })
}

fn read_public_key(path: &Path) -> AppResult<String> {
    let contents = fs::read_to_string(path).map_err(|err| {
        CommandError::new(codes::KEY_FAILED, "无法读取生成的公钥。")
            .detail(format!("{}: {err}", display_path(path)))
    })?;
    let mut lines = contents.lines().filter(|line| !line.trim().is_empty());
    let line = lines
        .next()
        .ok_or_else(|| CommandError::new(codes::KEY_FAILED, "生成的公钥文件为空。"))?;
    if lines.next().is_some() {
        return Err(CommandError::new(
            codes::KEY_FAILED,
            "公钥文件包含多行内容，Keyport 不会继续处理。",
        ));
    }
    Ok(line.trim().to_string())
}

fn apply_key_permissions(private_path: &Path, public_path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(private_path, fs::Permissions::from_mode(0o600)).map_err(|err| {
            CommandError::new(codes::KEY_FAILED, "无法收紧私钥权限。").detail(err.to_string())
        })?;
        fs::set_permissions(public_path, fs::Permissions::from_mode(0o644)).map_err(|err| {
            CommandError::new(codes::KEY_FAILED, "无法设置公钥权限。").detail(err.to_string())
        })?;
    }
    #[cfg(not(unix))]
    {
        let _ = (private_path, public_path);
    }
    Ok(())
}
