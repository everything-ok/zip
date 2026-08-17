//! Extractr Tauri 适配层：把 archive-core 的同步接口桥接到 Tauri 命令 + Channel 进度。

pub mod adapter;
pub mod commands;
pub mod events;
pub mod state;

use std::sync::Mutex;

use tauri::{Emitter, Manager};

/// 从 argv 解析右键菜单动作与归档路径。
/// 支持 `--extract-here <path>` / `--extract-to-subdir <path>` / `--compress <path>` / 裸路径（打开预览）。
fn parse_open_action(args: &[String]) -> Option<OpenAction> {
    let mut iter = args.iter().skip(1);
    let archive_exts = [
        ".zip",
        ".7z",
        ".rar",
        ".tar",
        ".gz",
        ".gzip",
        ".bz2",
        ".xz",
        ".zst",
        ".zstd",
        ".tgz",
        ".tbz2",
        ".tbz",
        ".txz",
        ".tzst",
        ".tzs",
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
            "--compress" => {
                // 压缩目标通常是目录；校验路径存在，避免面板预填不存在的路径。
                if let Some(path) = iter.next() {
                    if std::path::Path::new(path).exists() {
                        return Some(OpenAction::Compress { path: path.clone() });
                    }
                }
            }
            _ => {
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
    Compress { path: String },
}

/// 缓存首启动作：文件关联/右键启动时 webview 可能尚未加载完成，
/// emit 会丢失；前端 ready 后会调用 `pop_pending_open` 取回。
static PENDING_OPEN: Mutex<Option<OpenAction>> = Mutex::new(None);

/// macOS 文件关联启动：Apple Events 传递 file:// URL，非 argv。
/// 将 URL 解析为本地路径后，按扩展名判断动作并缓存到 PENDING_OPEN。
#[cfg(target_os = "macos")]
fn handle_opened_urls(urls: &[url::Url]) {
    use std::path::Path;
    let archive_exts = [
        ".zip", ".7z", ".rar", ".tar", ".gz", ".gzip", ".bz2", ".xz",
        ".zst", ".zstd", ".tgz", ".tbz2", ".tbz", ".txz", ".tzst", ".tzs",
    ];
    let mut saved_action: Option<OpenAction> = None;
    for url in urls {
        if url.scheme() == "file" {
            if let Ok(path) = url.to_file_path() {
                let path_str = path.to_string_lossy().to_string();
                let lower = path_str.to_ascii_lowercase();
                if archive_exts.iter().any(|ext| lower.ends_with(ext)) && Path::new(&path).is_file() {
                    // 首个匹配文件作为 pending action 供前端预览。
                    if saved_action.is_none() {
                        saved_action = Some(OpenAction::Open { path: path_str });
                    }
                }
            }
        }
    }
    if let Some(action) = saved_action {
        if let Ok(mut slot) = PENDING_OPEN.lock() {
            *slot = Some(action);
        }
    }
}

/// 缓存首启动作并尝试 emit 到 webview。
fn cache_and_emit(app: &tauri::App, action: OpenAction) {
    if let Ok(mut slot) = PENDING_OPEN.lock() {
        *slot = Some(action.clone());
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("open-archive", action);
    }
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
            commands::pop_pending_open,
            commands::file_size,
            commands::delete_source,
            commands::open_default_apps_settings,
        ]);

    // 处理文件关联/右键菜单启动参数：解析动作后缓存到 PENDING_OPEN，
    // 并尝试立即 emit（若 webview 已就绪则直接收到，否则前端 ready 后 pop）。
    builder = builder.setup(|app| {
        // Windows / Linux：从 argv 解析启动动作
        #[cfg(not(target_os = "macos"))]
        {
            let args: Vec<String> = std::env::args().collect();
            if let Some(action) = parse_open_action(&args) {
                cache_and_emit(app, action);
            }
        }
        // macOS：argv 不含文件路径，Apple Events 通过 RunEvent::Opened 传递。
        // setup 阶段无法拿到，需在 run loop 中处理。
        // 首次启动时若通过 open -a 打开文件，Opened 事件会在 run loop 启动后触发。
        Ok(())
    });

    builder = builder.on_window_event(|_window, _event| {});

    // macOS 需用 build + run 模式以接收 RunEvent::Opened (Apple Events 文件关联)
    #[cfg(target_os = "macos")]
    {
        let app = builder.build(tauri::generate_context!()).expect("error while building Extractr");
        app.run(|app_handle, event| {
            if let tauri::RunEvent::Opened { urls } = event {
                handle_opened_urls(&urls);
                // 尝试 emit 已缓存的动作到 webview
                if let Ok(slot) = PENDING_OPEN.lock() {
                    if let Some(action) = slot.clone() {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.emit("open-archive", action);
                        }
                    }
                }
            }
        });
    }

    // Windows / Linux：直接 run
    #[cfg(not(target_os = "macos"))]
    {
        builder
            .run(tauri::generate_context!())
            .expect("error while running Extractr");
    }
}
