use serde::{Deserialize, Serialize};

fn default_port() -> u16 {
    22
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    pub name: String,
    pub host: String,
    pub user: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub reuse_existing_key: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeInput {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestInput {
    pub name: String,
}

/// Finish input intentionally does not implement `Debug`, `Clone` or `Serialize`.
/// The password is consumed once by the command and zeroized after use.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishInput {
    pub connection: ConnectionInput,
    pub password: String,
    pub expected_fingerprint: String,
    #[serde(default)]
    pub trust_host: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSshInfo {
    pub available: bool,
    pub version: Option<String>,
    pub missing: Vec<String>,
    pub install_hint: String,
    pub auto_install_supported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub ready: bool,
    pub running: bool,
    pub autostart: bool,
    pub socket: Option<String>,
    pub manager: String,
    pub summary: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub platform: String,
    pub arch: String,
    pub platform_label: String,
    pub home: String,
    pub ssh_dir: String,
    pub config_path: String,
    pub known_hosts_path: String,
    pub openssh: OpenSshInfo,
    pub agent: AgentStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub installed: bool,
    pub log: String,
    pub environment: EnvironmentReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStage {
    pub id: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareResult {
    pub alias: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub key_path: String,
    pub public_key: String,
    pub fingerprint: String,
    pub algorithm: String,
    pub config_path: String,
    pub known_hosts_path: String,
    pub reused_key: bool,
    pub config_block: String,
    pub connect_command: String,
    pub agent: AgentStatus,
    pub stages: Vec<SetupStage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HostTrustStatus {
    Trusted,
    Unknown,
    Changed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostProbe {
    pub host: String,
    pub port: u16,
    pub host_token: String,
    pub fingerprint: String,
    pub algorithm: String,
    pub status: HostTrustStatus,
    pub recorded_fingerprints: Vec<String>,
    pub known_hosts_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishResult {
    pub copied: bool,
    pub host_key_learned: bool,
    pub agent: AgentStatus,
    pub test: TestResult,
    pub alias: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub connect_command: String,
    pub key_path: String,
    pub config_path: String,
    pub known_hosts_path: String,
    pub fingerprint: String,
    pub elapsed_ms: u64,
}
