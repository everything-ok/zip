//! Extractr Tauri 适配层：把 archive-core 的同步接口桥接到 Tauri 命令 + Channel 进度。

pub mod adapter;
pub mod commands;
pub mod events;
pub mod state;

use tauri::{Emitter, Manager};

/// 从 argv 解析右键菜单动作与归档路径。
/// 支持 `--extract-here <path>` / `--extract-to-subdir <path>` / 裸路径（打开预览）。
fn parse_open_action(args: &[String]) -> Option<OpenAction> {
    let mut iter = args.iter().skip(1);
    let archive_exts = [
        ".zip", ".7z", ".rar", ".tar", ".gz", ".gzip", ".bz2", ".xz", ".zst", ".zstd", ".tgz",
        ".tbz2", "tbz", ".txz", ".tzst", ".tzs",
    ];
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--extract-here" => {
                if let Some(path) = iter.next() {
                    return Some(OpenAction::ExtractHere { path: path.clone() });
                }
            }
            "--extract-to-subdir" => {
                if let Some(path) = iter.next() {
                    return Some(OpenAction::ExtractToSubdir { path: path.clone() });
                }
            }
            _ => {
                // 裸路径：归档文件则打开预览。
                let lower = arg.to_ascii_lowercase();
                if archive_exts.iter().any(|ext| lower.ends_with(ext))
                    && std::path::Path::new(arg).is_file()
                {
                    return Some(OpenAction::Open { path: arg.clone() });
                }
            }
        }
    }
    None
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "action")]
enum OpenAction {
    Open { path: String },
    ExtractHere { path: String },
    ExtractToSubdir { path: String },
}

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
            commands::convert_archive,
            commands::test_archive,
            commands::check_update,
        ]);

    // 处理文件关联/右键菜单启动参数，解析动作并 emit open-archive 事件。
    builder = builder.setup(|app| {
        let args: Vec<String> = std::env::args().collect();
        if let Some(action) = parse_open_action(&args) {
            let window = app.get_webview_window("main");
            if let Some(window) = window {
                let _ = window.emit("open-archive", action);
            }
        }
        Ok(())
    });

    builder = builder.on_window_event(|_window, _event| {});

    builder
        .run(tauri::generate_context!())
        .expect("error while running Extractr");
}
