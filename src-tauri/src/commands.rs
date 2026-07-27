//! Tauri 命令：前端通过 `invoke` 调用，进度经 `Channel` 回流。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Deserialize;

use tauri::async_runtime;
use tauri::ipc::Channel;
use tauri::State;

use archive_core::creators;
use archive_core::traits::{CancelToken, CreateContext, CreateOptions, CreateSource, ExtractContext};
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
                compressed_size: entry.compressed_size,
                is_dir: entry.is_dir,
                is_encrypted: entry.is_encrypted,
                modified: entry.modified,
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
    // 拒绝重复 task_id 与并发超限，避免旧任务令牌被覆盖或资源耗尽。
    match state.register_task(&req.task_id, cancel.clone()).await {
        Ok(_) => {}
        Err(crate::state::RegisterError::Duplicate(_)) => {
            let error = ArchiveErrorDto::new("duplicate_task", "任务 ID 已存在");
            let _ = on_progress.send(ProgressEvent::Error {
                error: error.clone(),
            });
            return Err(error);
        }
        Err(crate::state::RegisterError::TooMany(current)) => {
            let error = ArchiveErrorDto::new(
                "too_many_tasks",
                format!("并发任务已达上限 {}（当前 {}）", crate::state::MAX_CONCURRENT_TASKS, current),
            );
            let _ = on_progress.send(ProgressEvent::Error {
                error: error.clone(),
            });
            return Err(error);
        }
    }

    let source = PathBuf::from(&req.source);
    let dest = PathBuf::from(&req.dest);
    let options = parse_options(&req);
    let progress = ChannelSink::new(on_progress.clone());
    let cancel_tok = ArcCancel(cancel.clone());
    let entries = req.entries;

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
        match &entries {
            Some(list) if !list.is_empty() => extractor.extract_entries(&context, list),
            _ => extractor.extract(&context),
        }
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

#[derive(Deserialize)]
pub struct CreateRequest {
    pub task_id: String,
    pub dest: String,
    pub sources: Vec<CreateSourceDto>,
    pub password: Option<String>,
    pub level: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateSourceDto {
    pub fs_path: String,
    pub archive_path: String,
}

/// 创建归档（当前支持 .zip）。进度经 `on_progress` Channel 流式推送。
#[tauri::command]
pub async fn create_archive(
    req: CreateRequest,
    on_progress: Channel<ProgressEvent>,
    state: State<'_, AppState>,
) -> Result<SummaryDto, ArchiveErrorDto> {
    let cancel = Arc::new(AtomicBool::new(false));
    match state.register_task(&req.task_id, cancel.clone()).await {
        Ok(_) => {}
        Err(crate::state::RegisterError::Duplicate(_)) => {
            let error = ArchiveErrorDto::new("duplicate_task", "任务 ID 已存在");
            let _ = on_progress.send(ProgressEvent::Error {
                error: error.clone(),
            });
            return Err(error);
        }
        Err(crate::state::RegisterError::TooMany(current)) => {
            let error = ArchiveErrorDto::new(
                "too_many_tasks",
                format!(
                    "并发任务已达上限 {}（当前 {}）",
                    crate::state::MAX_CONCURRENT_TASKS,
                    current
                ),
            );
            let _ = on_progress.send(ProgressEvent::Error {
                error: error.clone(),
            });
            return Err(error);
        }
    }

    let dest = PathBuf::from(&req.dest);
    let sources: Vec<CreateSource> = req
        .sources
        .into_iter()
        .map(|s| CreateSource {
            fs_path: PathBuf::from(&s.fs_path),
            archive_path: s.archive_path,
        })
        .collect();
    let options = CreateOptions {
        password: req.password,
        level: req.level,
    };
    let progress = ChannelSink::new(on_progress.clone());
    let cancel_tok = ArcCancel(cancel.clone());

    let join_result = async_runtime::spawn_blocking(move || {
        let creator = creators::creator_for_path(&dest)?;
        let ctx = CreateContext {
            dest: &dest,
            sources: &sources,
            options: &options,
            progress: &progress,
            cancel: &cancel_tok,
        };
        creator.create(&ctx)
    })
    .await;

    state.drop_task(&req.task_id, &cancel).await;

    let create_result = match join_result {
        Ok(inner) => inner,
        Err(error) => {
            let dto = ArchiveErrorDto::new("io", format!("创建任务异常: {error}"));
            let _ = on_progress.send(ProgressEvent::Error {
                error: dto.clone(),
            });
            return Err(dto);
        }
    };

    match create_result {
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
            let _ = on_progress.send(ProgressEvent::Error {
                error: dto.clone(),
            });
            Err(dto)
        }
    }
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
