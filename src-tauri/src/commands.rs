use std::time::Instant;

use zeroize::Zeroizing;

use crate::agent;
use crate::error::{codes, AppResult, CommandError};
use crate::keygen;
use crate::model::{
    AgentStatus, ConnectionInput, EnvironmentReport, FinishInput, FinishResult, InstallResult,
    PrepareResult, ProbeInput, SetupStage, TestInput, TestResult,
};
use crate::openssh::OpenSsh;
use crate::paths::SshPaths;
use crate::remote;
use crate::ssh_config;
use crate::update::{self, UpdateInfo};
use crate::util::truncate;
use crate::validation::{
    normalize_host, normalize_name, validate_expected_fingerprint, ValidatedConnection,
};

const PLATFORM_LABEL: &str = if cfg!(windows) {
    "Windows"
} else if cfg!(target_os = "macos") {
    "macOS"
} else if cfg!(target_os = "linux") {
    "Linux"
} else {
    "Unix"
};

#[tauri::command]
pub async fn diagnose() -> AppResult<EnvironmentReport> {
    let openssh = OpenSsh::detect();
    environment_report(&openssh)
}

#[tauri::command]
pub async fn install_openssh() -> AppResult<InstallResult> {
    let before = OpenSsh::detect();
    if before.info.available {
        return Ok(InstallResult {
            installed: true,
            log: "OpenSSH 已经可用，无需安装。".to_string(),
            environment: environment_report(&before)?,
        });
    }

    let log = before.install()?;
    let after = OpenSsh::detect();
    if !after.info.available {
        return Err(CommandError::new(
            codes::INSTALL_FAILED,
            "安装流程结束，但仍未检测到完整的 OpenSSH 组件。",
        )
        .hint(after.info.install_hint.clone())
        .detail(truncate(&log, 4000)));
    }
    Ok(InstallResult {
        installed: true,
        log,
        environment: environment_report(&after)?,
    })
}

#[tauri::command]
pub async fn prepare_connection(input: ConnectionInput) -> AppResult<PrepareResult> {
    let connection = ValidatedConnection::from_input(&input)?;
    let paths = SshPaths::discover()?;
    paths.ensure_ssh_dir()?;
    ssh_config::preflight_config(&paths, &connection)?;

    let openssh = OpenSsh::detect();
    openssh.require()?;

    let generated = keygen::prepare_key(
        &openssh,
        &paths,
        &connection.alias,
        &connection.key_filename,
        connection.key_type,
        input.reuse_existing_key,
    )?;

    let agent = agent::ensure(&openssh, &generated.private_path, &generated.public_key)?;
    let config = ssh_config::upsert_config(&paths, &connection)?;

    let stages = vec![
        SetupStage {
            id: "key".to_string(),
            status: if generated.reused {
                "reused"
            } else {
                "created"
            }
            .to_string(),
            detail: format!(
                "{} · {}",
                crate::paths::display_path(&generated.private_path),
                generated.fingerprint
            ),
        },
        SetupStage {
            id: "agent".to_string(),
            status: "ready".to_string(),
            detail: agent.summary.clone(),
        },
        SetupStage {
            id: "config".to_string(),
            status: "updated".to_string(),
            detail: config.path.clone(),
        },
    ];

    Ok(PrepareResult {
        alias: connection.alias.clone(),
        host: connection.host.clone(),
        user: connection.user.clone(),
        port: connection.port,
        key_path: crate::paths::display_path(&generated.private_path),
        public_key: generated.public_key,
        fingerprint: generated.fingerprint,
        algorithm: generated.algorithm,
        key_type: connection.key_type,
        remote_authorized_keys_path: connection.authorized_keys.display().to_string(),
        config_path: config.path,
        known_hosts_path: paths.display_known_hosts(),
        reused_key: generated.reused,
        config_block: config.block,
        connect_command: connection.connect_command(),
        agent,
        stages,
    })
}

#[tauri::command]
pub async fn probe_host(input: ProbeInput) -> AppResult<crate::model::HostProbe> {
    let host = normalize_host(&input.host)?;
    if input.port == 0 {
        return Err(CommandError::new(
            codes::VALIDATION,
            "端口必须在 1–65535 之间。",
        ));
    }
    let paths = SshPaths::discover()?;
    remote::probe_host(&host, input.port, &paths).await
}

#[tauri::command]
pub async fn finish_setup(input: FinishInput) -> AppResult<FinishResult> {
    let started = Instant::now();
    let FinishInput {
        connection: raw_connection,
        password,
        expected_fingerprint,
        trust_host,
    } = input;
    let password = Zeroizing::new(password);

    let connection = ValidatedConnection::from_input(&raw_connection)?;
    let expected_fingerprint = validate_expected_fingerprint(&expected_fingerprint)?;
    let openssh = OpenSsh::detect();
    openssh.require()?;
    let paths = SshPaths::discover()?;
    paths.ensure_ssh_dir()?;

    let generated = keygen::load_existing_key(
        &openssh,
        &paths,
        &connection.alias,
        &connection.key_filename,
        connection.key_type,
    )?;

    // Re-apply the managed config block, so a manually edited config can never
    // cause the final test to use different connection details.
    let config = ssh_config::upsert_config(&paths, &connection)?;
    let agent = agent::ensure(&openssh, &generated.private_path, &generated.public_key)?;

    let outcome = remote::install_public_key(
        &connection,
        &expected_fingerprint,
        trust_host,
        password.as_str(),
        &generated.public_key,
        &paths,
    )
    .await?;

    let test = remote::test_connection(&openssh, &connection.alias).await?;

    Ok(FinishResult {
        copied: true,
        host_key_learned: outcome.host_key_learned,
        agent,
        test,
        alias: connection.alias.clone(),
        host: connection.host.clone(),
        user: connection.user.clone(),
        port: connection.port,
        connect_command: connection.connect_command(),
        key_path: crate::paths::display_path(&generated.private_path),
        key_type: connection.key_type,
        remote_authorized_keys_path: connection.authorized_keys.display().to_string(),
        config_path: config.path,
        known_hosts_path: paths.display_known_hosts(),
        fingerprint: generated.fingerprint,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn test_connection(input: TestInput) -> AppResult<TestResult> {
    let alias = normalize_name(&input.name)?;
    let openssh = OpenSsh::detect();
    openssh.require()?;
    remote::test_connection(&openssh, &alias).await
}

#[tauri::command]
pub fn open_ssh_folder() -> AppResult<()> {
    let paths = SshPaths::discover()?;
    paths.ensure_ssh_dir()?;
    open::that_detached(&paths.ssh_dir).map_err(|error| {
        CommandError::new(codes::INTERNAL, "无法打开 SSH 目录。").detail(error.to_string())
    })?;
    Ok(())
}

#[tauri::command]
pub async fn check_update() -> AppResult<UpdateInfo> {
    // ureq is blocking; run it off the Tauri runtime so the window stays
    // responsive while GitHub is queried.
    tauri::async_runtime::spawn_blocking(update::check_for_update)
        .await
        .map_err(|error| {
            CommandError::new(codes::INTERNAL, "检查更新任务没有完成。").detail(error.to_string())
        })?
}

#[tauri::command]
pub fn open_release_page(url: String) -> AppResult<()> {
    update::open_release_page(&url)
}

fn environment_report(openssh: &OpenSsh) -> AppResult<EnvironmentReport> {
    let paths = SshPaths::discover()?;
    let agent: AgentStatus = agent::status(openssh);
    Ok(EnvironmentReport {
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        platform_label: PLATFORM_LABEL.to_string(),
        home: crate::paths::display_path(&paths.home),
        ssh_dir: paths.display_ssh_dir(),
        config_path: paths.display_config(),
        known_hosts_path: paths.display_known_hosts(),
        openssh: openssh.info.clone(),
        agent,
    })
}
