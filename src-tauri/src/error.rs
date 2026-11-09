use serde::Serialize;

pub type AppResult<T> = Result<T, CommandError>;

/// Serializable error returned to the webview.
///
/// Passwords and other credentials must never be placed in any field of this
/// type. `detail` is intended for bounded command stderr / technical context.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hint: None,
            detail: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        if !detail.trim().is_empty() {
            self.detail = Some(detail);
        }
        self
    }
}

pub mod codes {
    pub const VALIDATION: &str = "VALIDATION";
    pub const OPENSSH_MISSING: &str = "OPENSSH_MISSING";
    pub const INSTALL_FAILED: &str = "INSTALL_FAILED";
    pub const KEY_EXISTS: &str = "KEY_EXISTS";
    pub const KEY_FAILED: &str = "KEY_FAILED";
    pub const CONFIG_CONFLICT: &str = "CONFIG_CONFLICT";
    pub const CONFIG_FAILED: &str = "CONFIG_FAILED";
    pub const AGENT_FAILED: &str = "AGENT_FAILED";
    pub const PROBE_FAILED: &str = "PROBE_FAILED";
    pub const HOST_KEY_CHANGED: &str = "HOST_KEY_CHANGED";
    pub const HOST_KEY_UNTRUSTED: &str = "HOST_KEY_UNTRUSTED";
    pub const HOST_KEY_UNSUPPORTED: &str = "HOST_KEY_UNSUPPORTED";
    pub const AUTH_FAILED: &str = "AUTH_FAILED";
    pub const REMOTE_FAILED: &str = "REMOTE_FAILED";
    pub const KNOWN_HOSTS_FAILED: &str = "KNOWN_HOSTS_FAILED";
    pub const TEST_FAILED: &str = "TEST_FAILED";
    pub const INTERNAL: &str = "INTERNAL";
}

pub trait IoResultExt<T> {
    fn io_context(self, context: impl Into<String>) -> AppResult<T>;
}

impl<T> IoResultExt<T> for std::io::Result<T> {
    fn io_context(self, context: impl Into<String>) -> AppResult<T> {
        self.map_err(|err| {
            CommandError::new(codes::INTERNAL, context.into()).detail(err.to_string())
        })
    }
}
