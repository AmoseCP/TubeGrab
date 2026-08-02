mod download;
mod engine;
mod error;
mod parser;
mod settings;
mod updater;

use std::sync::{Arc, Mutex};

use download::{DownloadState, SharedState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage::<SharedState>(Arc::new(Mutex::new(DownloadState::default())))
        .setup(|app| {
            let handle = app.handle().clone();
            // 恢复持久化队列并自动续传
            download::load_persisted(&handle);
            download::pump(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine::get_engine_version,
            engine::update_engine,
            parser::parse_url,
            parser::parse_playlist,
            download::add_tasks,
            download::get_tasks,
            download::cancel_task,
            download::retry_task,
            download::remove_task,
            download::open_in_folder,
            settings::get_settings,
            settings::save_settings,
            updater::check_app_update,
            updater::install_app_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
