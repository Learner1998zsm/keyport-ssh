use crate::error::{codes, AppResult, CommandError};
use crate::model::ConnectionInput;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedConnection {
    pub alias: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub key_filename: String,
}

impl ValidatedConnection {
    pub fn from_input(input: &ConnectionInput) -> AppResult<Self> {
        let alias = normalize_name(&input.name)?;
        let host = normalize_host(&input.host)?;
        let user = normalize_user(&input.user)?;
        if input.port == 0 {
            return Err(validation_error("端口必须在 1–65535 之间。"));
        }
        let key_filename = format!("id_ed25519_{alias}");

        Ok(Self {
            alias,
            host,
            user,
            port: input.port,
            key_filename,
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
        };
        let valid = ValidatedConnection::from_input(&input).unwrap();
        assert_eq!(valid.alias, "home");
        assert_eq!(valid.key_filename, "id_ed25519_home");
        assert_eq!(valid.identity_file(), "~/.ssh/id_ed25519_home");
        assert_eq!(valid.connect_command(), "ssh home");
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
