mod agent;
mod commands;
mod error;
mod keygen;
mod model;
mod openssh;
mod paths;
mod remote;
mod ssh_config;
mod util;
mod validation;

use commands::{
    diagnose, finish_setup, install_openssh, open_ssh_folder, prepare_connection, probe_host,
    test_connection,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            diagnose,
            install_openssh,
            prepare_connection,
            probe_host,
            finish_setup,
            test_connection,
            open_ssh_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running Keyport SSH");
}
