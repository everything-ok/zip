//! Extractr Tauri 适配层：把 archive-core 的同步接口桥接到 Tauri 命令 + Channel 进度。

pub mod adapter;
pub mod commands;
pub mod events;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::detect_format,
            commands::list_archive,
            commands::extract_archive,
            commands::cancel_extraction,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Extractr");
}
