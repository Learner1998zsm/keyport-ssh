use crate::error::{codes, AppResult, CommandError};
use crate::model::{ConnectionInput, KeyType};
use crate::util::shell_quote;

pub const DEFAULT_AUTHORIZED_KEYS_PATH: &str = "~/.ssh/authorized_keys";
pub const DROPBEAR_AUTHORIZED_KEYS_PATH: &str = "/etc/dropbear/authorized_keys";
const MAX_REMOTE_PATH_LEN: usize = 512;

/// A validated remote authorized_keys destination.
///
/// Paths are kept in two forms so the remote command can expand `~` on the
/// server instead of on this machine. Only a conservative ASCII allow-list is
/// accepted, and every path fragment is still passed through `shell_quote`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedKeysTarget {
    display: String,
    home_relative: Option<String>,
    absolute: Option<String>,
}

impl AuthorizedKeysTarget {
    pub fn parse(raw: Option<&str>) -> AppResult<Self> {
        let value = match raw {
            Some(value) => value.trim(),
            None => DEFAULT_AUTHORIZED_KEYS_PATH,
        };
        if value.is_empty() {
            return Err(validation_error("远端公钥路径不能为空。"));
        }
        if value.len() > MAX_REMOTE_PATH_LEN {
            return Err(validation_error(format!(
                "远端公钥路径不能超过 {MAX_REMOTE_PATH_LEN} 个字符。"
            )));
        }
        if !value.is_ascii() {
            return Err(validation_error("远端公钥路径只能使用 ASCII 字符。"));
        }

        let (home_relative, rest) = if let Some(rest) = value.strip_prefix("~/") {
            if rest.is_empty() {
                return Err(validation_error("~/ 后面还需要填写具体文件路径。"));
            }
            (Some(rest.to_string()), rest)
        } else if let Some(rest) = value.strip_prefix('/') {
            if rest.is_empty() {
                return Err(validation_error("远端公钥路径必须指向一个文件。"));
            }
            (None, rest)
        } else {
            return Err(validation_error(
                "远端公钥路径必须以 / 开头，或使用 ~/ 表示登录用户主目录。",
            ));
        };

        if !rest.bytes().all(is_allowed_path_byte) {
            return Err(validation_error(
                "远端公钥路径包含不支持的字符。只允许字母、数字、点、下划线、短横线、加号、@ 和 /。",
            ));
        }
        if rest.contains("//") || rest.starts_with('/') || rest.ends_with('/') {
            return Err(validation_error("远端公钥路径不能包含空路径段。"));
        }
        for segment in rest.split('/') {
            if segment.is_empty() {
                return Err(validation_error("远端公钥路径不能包含空路径段。"));
            }
            if segment == "." || segment == ".." {
                return Err(validation_error("远端公钥路径不能包含 . 或 .. 路径段。"));
            }
        }

        let display = if home_relative.is_some() {
            format!("~/{rest}")
        } else {
            format!("/{rest}")
        };
        let absolute = home_relative.is_none().then(|| display.clone());

        Ok(Self {
            display,
            home_relative,
            absolute,
        })
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn is_dropbear_default(&self) -> bool {
        self.absolute.as_deref() == Some(DROPBEAR_AUTHORIZED_KEYS_PATH)
    }

    pub fn is_under_etc(&self) -> bool {
        self.absolute
            .as_deref()
            .is_some_and(|path| path == "/etc" || path.starts_with("/etc/"))
    }

    /// Shell expression that expands to the target file on the remote host.
    pub fn target_shell_expr(&self) -> String {
        match &self.home_relative {
            Some(relative) => format!("\"$HOME\"/{}", shell_quote(relative)),
            None => shell_quote(self.absolute.as_deref().unwrap_or("/")),
        }
    }

    /// Shell expression for the parent directory, used only for `mkdir -p`.
    pub fn parent_shell_expr(&self) -> String {
        match &self.home_relative {
            Some(relative) => match relative.rsplit_once('/') {
                Some((parent, _)) if !parent.is_empty() => {
                    format!("\"$HOME\"/{}", shell_quote(parent))
                }
                _ => "\"$HOME\"".to_string(),
            },
            None => {
                let absolute = self.absolute.as_deref().unwrap_or("/");
                match absolute.rsplit_once('/') {
                    Some((parent, _)) if !parent.is_empty() => shell_quote(parent),
                    _ => shell_quote("/"),
                }
            }
        }
    }

    /// Only ~/.ssh is made private. Directories under /etc (notably
    /// /etc/dropbear) are intentionally left untouched.
    pub fn tighten_home_ssh_dir(&self) -> bool {
        matches!(
            self.home_relative
                .as_deref()
                .and_then(|path| path.rsplit_once('/')),
            Some((".ssh", _))
        )
    }
}

fn is_allowed_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-' | b'+' | b'@')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedConnection {
    pub alias: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub key_type: KeyType,
    pub key_filename: String,
    pub authorized_keys: AuthorizedKeysTarget,
}

impl ValidatedConnection {
    pub fn from_input(input: &ConnectionInput) -> AppResult<Self> {
        let alias = normalize_name(&input.name)?;
        let host = normalize_host(&input.host)?;
        let user = normalize_user(&input.user)?;
        if input.port == 0 {
            return Err(validation_error("端口必须在 1–65535 之间。"));
        }
        let key_filename = format!("{}_{alias}", input.key_type.filename_prefix());
        let authorized_keys =
            AuthorizedKeysTarget::parse(input.remote_authorized_keys_path.as_deref())?;

        Ok(Self {
            alias,
            host,
            user,
            port: input.port,
            key_type: input.key_type,
            key_filename,
            authorized_keys,
        })
    }

    pub fn connect_command(&self) -> String {
        format!("ssh {}", self.alias)
    }

    pub fn identity_file(&self) -> String {
        format!("~/.ssh/{}", self.key_filename)
    }
}

fn validation_error(message: impl Into<String>) -> CommandError {
    CommandError::new(codes::VALIDATION, message)
}

pub fn normalize_name(raw: &str) -> AppResult<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(validation_error("请填写连接名称。"));
    }
    if name.len() > 64 {
        return Err(validation_error("连接名称不能超过 64 个字符。"));
    }
    if !name.is_ascii() {
        return Err(validation_error(
            "连接名称请使用 ASCII 字母、数字、点、下划线或短横线。",
        ));
    }
    let first = name.chars().next().unwrap_or_default();
    if !first.is_ascii_alphanumeric() {
        return Err(validation_error("连接名称必须以字母或数字开头。"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(validation_error(
            "连接名称只能包含字母、数字、点、下划线和短横线。",
        ));
    }
    Ok(name.to_string())
}

pub fn normalize_host(raw: &str) -> AppResult<String> {
    let mut host = raw.trim();
    if host.starts_with('[') && host.ends_with(']') && host.len() > 2 {
        host = &host[1..host.len() - 1];
    }
    if host.is_empty() {
        return Err(validation_error("请填写服务器地址。"));
    }
    if host.len() > 253 {
        return Err(validation_error("服务器地址过长。"));
    }
    if host.starts_with('-') {
        return Err(validation_error("服务器地址不能以短横线开头。"));
    }
    if !host.is_ascii() {
        return Err(validation_error(
            "服务器地址请使用 IPv4、IPv6 或 ASCII 主机名。",
        ));
    }
    if host
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '_')))
    {
        return Err(validation_error(
            "服务器地址包含不支持的字符。请只填写 IP 或主机名，不要包含用户名、斜杠或空格。",
        ));
    }
    if !host.chars().any(|c| c.is_ascii_alphanumeric()) {
        return Err(validation_error("服务器地址无效。"));
    }
    if host.contains("..") && !host.contains(':') {
        return Err(validation_error("服务器地址不能包含连续的 ..。"));
    }
    Ok(host.to_string())
}

pub fn normalize_user(raw: &str) -> AppResult<String> {
    let user = raw.trim();
    if user.is_empty() {
        return Err(validation_error("请填写登录用户。"));
    }
    if user.len() > 64 {
        return Err(validation_error("登录用户不能超过 64 个字符。"));
    }
    if user.starts_with('-') {
        return Err(validation_error("登录用户不能以短横线开头。"));
    }
    if !user
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@' | '+'))
    {
        return Err(validation_error(
            "登录用户只能包含字母、数字、点、下划线、短横线、@ 或 +。",
        ));
    }
    Ok(user.to_string())
}

pub fn validate_expected_fingerprint(raw: &str) -> AppResult<String> {
    let fp = raw.trim();
    if !fp.starts_with("SHA256:") || fp.len() < 20 || fp.len() > 160 {
        return Err(validation_error("主机指纹格式无效，请重新探测主机。"));
    }
    if fp
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || matches!(c, ':' | '+' | '/' | '=')))
    {
        return Err(validation_error("主机指纹格式无效，请重新探测主机。"));
    }
    Ok(fp.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_connection() {
        let input = ConnectionInput {
            name: "home".into(),
            host: "10.126.126.3".into(),
            user: "z".into(),
            port: 22,
            reuse_existing_key: false,
            key_type: KeyType::Ed25519,
            remote_authorized_keys_path: Some(DEFAULT_AUTHORIZED_KEYS_PATH.into()),
        };
        let valid = ValidatedConnection::from_input(&input).unwrap();
        assert_eq!(valid.alias, "home");
        assert_eq!(valid.key_filename, "id_ed25519_home");
        assert_eq!(valid.identity_file(), "~/.ssh/id_ed25519_home");
        assert_eq!(valid.connect_command(), "ssh home");
        assert_eq!(valid.authorized_keys.display(), "~/.ssh/authorized_keys");
        assert_eq!(
            valid.authorized_keys.target_shell_expr(),
            "\"$HOME\"/'.ssh/authorized_keys'"
        );
        assert_eq!(
            valid.authorized_keys.parent_shell_expr(),
            "\"$HOME\"/'.ssh'"
        );
        assert!(valid.authorized_keys.tighten_home_ssh_dir());
    }

    #[test]
    fn rsa_connection_uses_rsa_filename() {
        let input = ConnectionInput {
            name: "router".into(),
            host: "192.168.1.1".into(),
            user: "root".into(),
            port: 22,
            reuse_existing_key: false,
            key_type: KeyType::Rsa,
            remote_authorized_keys_path: Some(DROPBEAR_AUTHORIZED_KEYS_PATH.into()),
        };
        let valid = ValidatedConnection::from_input(&input).unwrap();
        assert_eq!(valid.key_filename, "id_rsa_router");
        assert_eq!(valid.identity_file(), "~/.ssh/id_rsa_router");
        assert_eq!(
            valid.authorized_keys.target_shell_expr(),
            "'/etc/dropbear/authorized_keys'"
        );
        assert_eq!(valid.authorized_keys.parent_shell_expr(), "'/etc/dropbear'");
        assert!(!valid.authorized_keys.tighten_home_ssh_dir());
        assert!(valid.authorized_keys.is_dropbear_default());
        assert!(valid.authorized_keys.is_under_etc());
        assert!(
            !AuthorizedKeysTarget::parse(Some("/data/ssh/authorized_keys"))
                .unwrap()
                .is_under_etc()
        );
    }

    #[test]
    fn remote_authorized_keys_paths_are_strict() {
        assert_eq!(
            AuthorizedKeysTarget::parse(None).unwrap().display(),
            DEFAULT_AUTHORIZED_KEYS_PATH
        );
        for path in [
            "~/.ssh/authorized_keys",
            "/etc/dropbear/authorized_keys",
            "~/keys/authorized_keys",
            "/data/ssh/authorized_keys",
            "~/.ssh/authorized_keys-2",
        ] {
            assert!(
                AuthorizedKeysTarget::parse(Some(path)).is_ok(),
                "should accept {path:?}"
            );
        }

        for path in [
            "",
            "~",
            "relative/authorized_keys",
            "/etc/../authorized_keys",
            "/etc//authorized_keys",
            "~//authorized_keys",
            "/etc/dropbear/",
            "/etc/dropbear/authorized keys",
            "/etc/dropbear/authorized_keys;reboot",
            "/etc/dropbear/authorized_keys\nrm -rf /",
            "/etc/$HOME/authorized_keys",
            "/etc/dropbear/authorized`id`",
        ] {
            assert!(
                AuthorizedKeysTarget::parse(Some(path)).is_err(),
                "should reject {path:?}"
            );
        }
    }

    #[test]
    fn name_rejects_path_and_spaces() {
        for value in ["../home", "my home", "-home", "café", ""] {
            assert!(normalize_name(value).is_err(), "should reject {value:?}");
        }
        assert_eq!(normalize_name(" Home.01 ").unwrap(), "Home.01");
    }

    #[test]
    fn host_accepts_ipv4_ipv6_and_dns_but_strips_brackets() {
        assert_eq!(normalize_host(" 10.0.0.1 ").unwrap(), "10.0.0.1");
        assert_eq!(normalize_host("[2001:db8::1]").unwrap(), "2001:db8::1");
        assert_eq!(
            normalize_host("server.example.com").unwrap(),
            "server.example.com"
        );
        assert!(normalize_host("z@10.0.0.1").is_err());
        assert!(normalize_host("-oProxyCommand=bad").is_err());
        assert!(normalize_host("host name").is_err());
    }

    #[test]
    fn user_is_defensive() {
        assert_eq!(normalize_user("deploy").unwrap(), "deploy");
        assert_eq!(normalize_user("user@example").unwrap(), "user@example");
        assert!(normalize_user("bad user").is_err());
        assert!(normalize_user("-root").is_err());
    }

    #[test]
    fn fingerprint_shape_is_checked() {
        assert!(validate_expected_fingerprint("SHA256:AbCdEf0123456789abcd").is_ok());
        assert!(validate_expected_fingerprint("MD5:aa:bb").is_err());
        assert!(validate_expected_fingerprint("SHA256:bad space here xx").is_err());
    }
}
