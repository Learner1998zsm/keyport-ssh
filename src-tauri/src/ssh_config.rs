use std::fs;
use std::path::Path;

use crate::error::{codes, AppResult, CommandError};
use crate::paths::{atomic_write, display_path, SshPaths};
use crate::validation::ValidatedConnection;

#[derive(Debug, Clone)]
pub struct ConfigUpdate {
    pub block: String,
    pub path: String,
}

pub fn build_block(connection: &ValidatedConnection) -> String {
    let begin = format!(
        "# >>> keyport:{} managed by Keyport SSH >>>",
        connection.alias
    );
    let end = format!("# <<< keyport:{} <<<", connection.alias);
    let mut lines = vec![
        begin,
        format!("Host {}", connection.alias),
        format!("    HostName {}", connection.host),
        format!("    User {}", connection.user),
    ];
    if connection.port != 22 {
        lines.push(format!("    Port {}", connection.port));
    }
    lines.extend([
        format!("    IdentityFile {}", connection.identity_file()),
        "    IdentitiesOnly yes".to_string(),
        "    BatchMode yes".to_string(),
        "    ServerAliveInterval 30".to_string(),
        end,
    ]);
    lines.join("\n") + "\n"
}

pub fn upsert_config(
    paths: &SshPaths,
    connection: &ValidatedConnection,
) -> AppResult<ConfigUpdate> {
    paths.ensure_ssh_dir()?;
    let block = build_block(connection);
    let config_path = &paths.config_file;
    let existing = if config_path.exists() {
        fs::read_to_string(config_path).map_err(|err| {
            CommandError::new(codes::CONFIG_FAILED, "无法读取现有 SSH 配置。")
                .detail(format!("{}: {err}", display_path(config_path)))
        })?
    } else {
        String::new()
    };

    let updated = render_updated_config(&existing, &connection.alias, &block)?;

    if !existing.trim().is_empty() && !has_managed_block(&existing, &connection.alias) {
        let backup = config_path.with_extension("keyport.bak");
        if !backup.exists() {
            write_config_file(&backup, &existing)?;
        }
    }

    write_config_file(config_path, &updated)?;
    Ok(ConfigUpdate {
        block,
        path: display_path(config_path),
    })
}

/// Check for an unmanaged Host collision before generating any key material.
pub fn preflight_config(paths: &SshPaths, connection: &ValidatedConnection) -> AppResult<()> {
    if !paths.config_file.exists() {
        return Ok(());
    }
    let existing = fs::read_to_string(&paths.config_file).map_err(|err| {
        CommandError::new(codes::CONFIG_FAILED, "无法读取现有 SSH 配置。")
            .detail(format!("{}: {err}", display_path(&paths.config_file)))
    })?;
    let without_managed = remove_managed_blocks(&existing, &connection.alias)?;
    detect_host_conflict(&without_managed, &connection.alias)
}

fn write_config_file(path: &Path, contents: &str) -> AppResult<()> {
    #[cfg(unix)]
    let mode = Some(0o600);
    #[cfg(not(unix))]
    let mode = None;
    atomic_write(path, contents.as_bytes(), mode).map_err(|err| {
        CommandError::new(codes::CONFIG_FAILED, "无法写入 SSH 配置。").detail(format!(
            "{}: {}",
            display_path(path),
            err.message
        ))
    })
}

fn begin_marker(alias: &str) -> String {
    format!("# >>> keyport:{alias} managed by Keyport SSH >>>")
}

fn end_marker(alias: &str) -> String {
    format!("# <<< keyport:{alias} <<<")
}

fn has_managed_block(contents: &str, alias: &str) -> bool {
    let begin = begin_marker(alias);
    contents.lines().any(|line| line.trim() == begin)
}

/// Remove every managed block for this alias. A malformed block is an error,
/// because silently deleting the wrong lines would be worse than asking the
/// user to repair their SSH config.
fn remove_managed_blocks(contents: &str, alias: &str) -> AppResult<String> {
    let begin = begin_marker(alias);
    let end = end_marker(alias);
    let mut output = Vec::new();
    let mut inside = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if !inside && trimmed == begin {
            inside = true;
            continue;
        }
        if inside {
            if trimmed == end {
                inside = false;
            }
            continue;
        }
        output.push(line);
    }

    if inside {
        return Err(CommandError::new(
            codes::CONFIG_CONFLICT,
            format!("SSH 配置中的 Keyport 管理块（{alias}）缺少结束标记。"),
        )
        .hint("请手动检查或修复 ~/.ssh/config 中对应的 Keyport 注释块。"));
    }

    let mut rendered = output.join("\n");
    while rendered.contains("\n\n\n") {
        rendered = rendered.replace("\n\n\n", "\n\n");
    }
    Ok(rendered)
}

fn render_updated_config(contents: &str, alias: &str, block: &str) -> AppResult<String> {
    let without_managed = remove_managed_blocks(contents, alias)?;
    detect_host_conflict(&without_managed, alias)?;

    let body = without_managed.trim_matches('\n');
    let mut next = String::new();
    next.push_str(block);
    if !body.trim().is_empty() {
        next.push('\n');
        next.push_str(body);
        if !next.ends_with('\n') {
            next.push('\n');
        }
    }
    Ok(next)
}

fn detect_host_conflict(contents: &str, alias: &str) -> AppResult<()> {
    for (index, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut tokens = trimmed.split_whitespace();
        let Some(keyword) = tokens.next() else {
            continue;
        };
        let (keyword, first_pattern) = if let Some((key, rest)) = keyword.split_once('=') {
            (key, Some(rest))
        } else {
            (keyword, None)
        };
        if !keyword.eq_ignore_ascii_case("host") {
            continue;
        }

        let mut patterns = Vec::new();
        if let Some(first) = first_pattern {
            if !first.is_empty() {
                patterns.push(first);
            }
        }
        patterns.extend(tokens.take_while(|token| !token.starts_with('#')));

        for pattern in patterns {
            if pattern.eq_ignore_ascii_case(alias) {
                return Err(CommandError::new(
                    codes::CONFIG_CONFLICT,
                    format!(
                        "SSH 配置第 {} 行已经存在名为 “{}” 的 Host。",
                        index + 1,
                        alias
                    ),
                )
                .hint("请换一个连接名称，或先手动移走原有的 Host 配置（Keyport 不会覆盖非托管配置）。")
                .detail("~/.ssh/config"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ConnectionInput;

    fn connection(name: &str, host: &str, port: u16) -> ValidatedConnection {
        ValidatedConnection::from_input(&ConnectionInput {
            name: name.into(),
            host: host.into(),
            user: "deploy".into(),
            port,
            reuse_existing_key: false,
        })
        .unwrap()
    }

    #[test]
    fn block_contains_expected_directives() {
        let block = build_block(&connection("home", "10.126.126.3", 22));
        assert!(block.contains("Host home"));
        assert!(block.contains("HostName 10.126.126.3"));
        assert!(block.contains("User deploy"));
        assert!(block.contains("IdentityFile ~/.ssh/id_ed25519_home"));
        assert!(block.contains("IdentitiesOnly yes"));
        assert!(block.contains("BatchMode yes"));
        assert!(block.contains("ServerAliveInterval 30"));
        assert!(!block.contains("    Port 22\n"));
    }

    #[test]
    fn nonstandard_port_is_written() {
        let block = build_block(&connection("pi", "192.168.1.8", 2222));
        assert!(block.contains("    Port 2222"));
    }

    #[test]
    fn inserts_managed_block_before_existing_config() {
        let existing = "Host *\n    ServerAliveInterval 10\n";
        let conn = connection("home", "10.0.0.2", 22);
        let block = build_block(&conn);
        let updated = render_updated_config(existing, &conn.alias, &block).unwrap();
        assert!(updated.starts_with("# >>> keyport:home"));
        assert!(updated.ends_with("Host *\n    ServerAliveInterval 10\n"));
    }

    #[test]
    fn updates_existing_managed_block_without_duplication() {
        let conn = connection("home", "10.0.0.2", 22);
        let first = render_updated_config("", &conn.alias, &build_block(&conn)).unwrap();
        let conn2 = connection("home", "10.0.0.3", 22);
        let second = render_updated_config(&first, &conn2.alias, &build_block(&conn2)).unwrap();
        assert_eq!(second.matches("Host home").count(), 1);
        assert!(second.contains("HostName 10.0.0.3"));
        assert!(!second.contains("HostName 10.0.0.2"));
    }

    #[test]
    fn refuses_to_modify_unmanaged_same_alias() {
        let existing = "Host home\n    HostName legacy.example.com\n";
        let conn = connection("home", "10.0.0.2", 22);
        let error = render_updated_config(existing, &conn.alias, &build_block(&conn)).unwrap_err();
        assert_eq!(error.code, codes::CONFIG_CONFLICT);
    }

    #[test]
    fn allows_unrelated_and_wildcard_hosts() {
        let existing = "Host *\n    User default\n\nHost work\n    HostName work.example.com\n";
        let conn = connection("home", "10.0.0.2", 22);
        assert!(render_updated_config(existing, &conn.alias, &build_block(&conn)).is_ok());
    }

    #[test]
    fn broken_managed_block_is_reported() {
        let existing = "# >>> keyport:home managed by Keyport SSH >>>\nHost home\n";
        let conn = connection("home", "10.0.0.2", 22);
        let error = render_updated_config(existing, &conn.alias, &build_block(&conn)).unwrap_err();
        assert_eq!(error.code, codes::CONFIG_CONFLICT);
    }

    #[test]
    fn equal_syntax_host_is_detected() {
        let existing = "Host=home\n    HostName legacy\n";
        let conn = connection("home", "10.0.0.2", 22);
        assert!(render_updated_config(existing, &conn.alias, &build_block(&conn)).is_err());
    }
}
