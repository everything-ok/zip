//! Extractr Tauri 适配层：把 archive-core 的同步接口桥接到 Tauri 命令 + Channel 进度。

pub mod adapter;
pub mod commands;
pub mod events;
pub mod state;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::detect_format,
            commands::list_archive,
            commands::extract_archive,
            commands::cancel_extraction,
            commands::create_archive,
        ]);

    // 处理文件关联/右键菜单启动参数：当通过"用 Extractr 打开"或双击归档启动时，
    // argv 中带文件路径，前端可通过 open-file 事件接收并自动预览。
    builder = builder.setup(|app| {
        // 1. 启动时的命令行参数（Windows 双击关联文件会传入路径）。
        let args: Vec<String> = std::env::args().collect();
        let candidates: Vec<String> = args
            .into_iter()
            .skip(1)
            .filter(|a| {
                let lower = a.to_ascii_lowercase();
                let is_archive_ext = [
                    ".zip", ".7z", ".rar", ".tar", ".gz", ".gzip", ".bz2", ".xz", ".zst",
                    ".zstd", ".tgz", ".tbz2", "tbz", ".txz", ".tzst", ".tzs",
                ]
                .iter()
                .any(|ext| lower.ends_with(ext));
                is_archive_ext && std::path::Path::new(a).is_file()
            })
            .collect();
        if !candidates.is_empty() {
            let window = app.get_webview_window("main");
            if let Some(window) = window {
                let _ = window.emit("open-file", candidates[0].clone());
            }
        }
        Ok(())
    });

    // 2. 运行中再次通过文件关联打开（单实例已在后续扩展），监听 deep-link/file。
    builder = builder.on_window_event(|_window, _event| {});

    builder
        .run(tauri::generate_context!())
        .expect("error while running Extractr");
}
