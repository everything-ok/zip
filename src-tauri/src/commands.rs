//! Tauri 命令：前端通过 `invoke` 调用，进度经 `Channel` 回流。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::async_runtime;
use tauri::ipc::Channel;
use tauri::State;

use archive_core::traits::{CancelToken, ExtractContext};
use archive_core::{dispatcher, ExtractOptions, OverwritePolicy};

use crate::adapter::ChannelSink;
use crate::events::{
    ArchiveErrorDto, EntryDto, ExtractRequest, ListRequest, ProgressEvent, SummaryDto,
};
use crate::state::AppState;

/// 包装 `Arc<AtomicBool>` 实现 `CancelToken`，跨 spawn_blocking 共享。
struct ArcCancel(Arc<AtomicBool>);
impl CancelToken for ArcCancel {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// 探测归档格式。
#[tauri::command]
pub async fn detect_format(path: String) -> Result<String, String> {
    async_runtime::spawn_blocking(move || {
        archive_core::detect::detect_format(&PathBuf::from(&path))
            .map(|format| format.to_string())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// 列出归档条目。返回结构化错误，便于前端区分“需要密码”与其他失败。
#[tauri::command]
pub async fn list_archive(req: ListRequest) -> Result<Vec<EntryDto>, ArchiveErrorDto> {
    async_runtime::spawn_blocking(move || {
        let extractor = dispatcher::open(&PathBuf::from(&req.path))
            .map_err(|error| ArchiveErrorDto::from_anyhow(&error))?;
        let entries = extractor
            .list(&PathBuf::from(&req.path), req.password.as_deref())
            .map_err(|error| ArchiveErrorDto::from_anyhow(&error))?;
        Ok(entries
            .into_iter()
            .map(|entry| EntryDto {
                path: entry.path,
                size: entry.size,
                is_dir: entry.is_dir,
                is_encrypted: entry.is_encrypted,
            })
            .collect())
    })
    .await
    .map_err(|error| ArchiveErrorDto::new("io", error.to_string()))?
}

/// 解压（核心命令）。进度经 `on_progress` Channel 流式推送。
#[tauri::command]
pub async fn extract_archive(
    req: ExtractRequest,
    on_progress: Channel<ProgressEvent>,
    state: State<'_, AppState>,
) -> Result<SummaryDto, ArchiveErrorDto> {
    let cancel = Arc::new(AtomicBool::new(false));
    // 拒绝重复 task_id，避免旧任务令牌被覆盖而无法取消。
    if state
        .register_task(&req.task_id, cancel.clone())
        .await
        .is_err()
    {
        let error = ArchiveErrorDto::new("duplicate_task", "任务 ID 已存在");
        let _ = on_progress.send(ProgressEvent::Error {
            error: error.clone(),
        });
        return Err(error);
    }

    let source = PathBuf::from(&req.source);
    let dest = PathBuf::from(&req.dest);
    let options = parse_options(&req);
    let progress = ChannelSink::new(on_progress.clone());
    let cancel_tok = ArcCancel(cancel.clone());

    // spawn_blocking 的 join 结果先保存，任何路径都先清理任务表再返回。
    let join_result = async_runtime::spawn_blocking(move || {
        let extractor = dispatcher::open(&source)?;
        let context = ExtractContext {
            source: &source,
            dest: &dest,
            options: &options,
            progress: &progress,
            cancel: &cancel_tok,
        };
        extractor.extract(&context)
    })
    .await;

    // 统一清理：无论成功/失败/取消/panic 都按令牌归属移除当前任务。
    state.drop_task(&req.task_id, &cancel).await;

    // JoinError（panic / 运行时取消）必须先清理任务表再返回。
    let extract_result = match join_result {
        Ok(inner) => inner,
        Err(error) => {
            let dto = ArchiveErrorDto::new("io", format!("解压任务异常: {error}"));
            let _ = on_progress.send(ProgressEvent::Error { error: dto.clone() });
            return Err(dto);
        }
    };

    match extract_result {
        Ok(summary) if summary.cancelled => {
            let _ = on_progress.send(ProgressEvent::Cancelled);
            Err(ArchiveErrorDto::new("cancelled", "操作已取消"))
        }
        Ok(summary) => {
            let dto = SummaryDto {
                entries_extracted: summary.entries_extracted,
                entries_skipped: summary.entries_skipped,
                bytes_written: summary.bytes_written,
                cancelled: summary.cancelled,
            };
            let _ = on_progress.send(ProgressEvent::Finished {
                summary: dto.clone(),
            });
            Ok(dto)
        }
        Err(error) => {
            let dto = ArchiveErrorDto::from_anyhow(&error);
            let _ = on_progress.send(ProgressEvent::Error { error: dto.clone() });
            Err(dto)
        }
    }
}

/// 取消进行中的任务。
#[tauri::command]
pub async fn cancel_extraction(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_task(&task_id).await;
    Ok(())
}

fn parse_options(req: &ExtractRequest) -> ExtractOptions {
    let overwrite = match req.overwrite.as_str() {
        "overwrite" => OverwritePolicy::Overwrite,
        "rename" => OverwritePolicy::Rename,
        "error" => OverwritePolicy::Error,
        _ => OverwritePolicy::Skip,
    };
    ExtractOptions {
        password: req.password.clone(),
        overwrite,
        ..Default::default()
    }
}
