use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{codes, AppResult, CommandError, IoResultExt};

#[derive(Debug, Clone)]
pub struct SshPaths {
    pub home: PathBuf,
    pub ssh_dir: PathBuf,
    pub config_file: PathBuf,
    pub known_hosts_file: PathBuf,
}

impl SshPaths {
    pub fn discover() -> AppResult<Self> {
        let home = dirs::home_dir()
            .filter(|path| !path.as_os_str().is_empty())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
            .ok_or_else(|| CommandError::new(codes::INTERNAL, "无法确定当前用户的主目录。"))?;
        let ssh_dir = home.join(".ssh");
        Ok(Self {
            config_file: ssh_dir.join("config"),
            known_hosts_file: ssh_dir.join("known_hosts"),
            home,
            ssh_dir,
        })
    }

    pub fn ensure_ssh_dir(&self) -> AppResult<()> {
        make_private_dir(&self.ssh_dir)
    }

    pub fn key_path(&self, filename: &str) -> PathBuf {
        self.ssh_dir.join(filename)
    }

    pub fn public_key_path(&self, filename: &str) -> PathBuf {
        self.ssh_dir.join(format!("{filename}.pub"))
    }

    pub fn display_ssh_dir(&self) -> String {
        display_path(&self.ssh_dir)
    }

    pub fn display_config(&self) -> String {
        display_path(&self.config_file)
    }

    pub fn display_known_hosts(&self) -> String {
        display_path(&self.known_hosts_file)
    }
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn make_private_dir(path: &Path) -> AppResult<()> {
    fs::create_dir_all(path).io_context(format!("无法创建目录 {}", display_path(path)))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .io_context(format!("无法设置目录权限 {}", display_path(path)))?;
    }
    Ok(())
}

/// Write a file atomically in the same directory, optionally enforcing Unix mode.
pub fn atomic_write(path: &Path, contents: &[u8], unix_mode: Option<u32>) -> AppResult<()> {
    #[cfg(not(unix))]
    let _ = unix_mode;
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::new(codes::INTERNAL, "目标文件没有父目录。"))?;
    fs::create_dir_all(parent).io_context(format!("无法创建目录 {}", display_path(parent)))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("keyport-file");
    let temp_path = parent.join(format!(
        ".{file_name}.keyport-tmp-{}-{}",
        std::process::id(),
        unique_suffix()
    ));

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(unix_mode.unwrap_or(0o600));
    }
    let write_result = (|| -> AppResult<()> {
        let mut file = options
            .open(&temp_path)
            .io_context(format!("无法创建临时文件 {}", display_path(&temp_path)))?;
        file.write_all(contents).io_context("写入临时文件失败。")?;
        file.sync_all().io_context("同步临时文件失败。")?;
        drop(file);
        #[cfg(unix)]
        if let Some(mode) = unix_mode {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp_path, fs::Permissions::from_mode(mode))
                .io_context("无法设置文件权限。")?;
        }
        fs::rename(&temp_path, path)
            .io_context(format!("无法替换目标文件 {}", display_path(path)))?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_file_and_replaces() {
        let dir = std::env::temp_dir().join(format!(
            "keyport-paths-test-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        atomic_write(&path, b"first\n", None).unwrap();
        atomic_write(&path, b"second\n", None).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
        fs::remove_dir_all(&dir).unwrap();
    }
}
