//! 7z 解压器（基于 `sevenz-rust2`，支持 AES-256 密码）。
//! 用 `extract_fn` 回调实现路径净化（防 Zip Slip）、覆盖策略、进度回报与取消。

use std::path::Path;

use crate::error::ArchiveError;
use crate::extractors::copy_stream;
use crate::safety::{ensure_safe_directory, prepare_output, sanitize_entry_path};
use crate::traits::{ArchiveExtractor, CancelToken, ExtractContext, ProgressSink};
use crate::types::{ArchiveEntry, ArchiveFormat, ExtractSummary, OverwritePolicy};

pub struct SevenZExtractor;

impl ArchiveExtractor for SevenZExtractor {
    fn format_kind(&self) -> ArchiveFormat {
        ArchiveFormat::SevenZ
    }

    fn supports_password(&self) -> bool {
        true
    }

    fn list(&self, path: &Path, password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>> {
        let mut file = std::fs::File::open(path)?;
        let sevenz_password = match password {
            Some(value) => sevenz_rust2::Password::from(value),
            None => sevenz_rust2::Password::empty(),
        };
        let archive =
            sevenz_rust2::Archive::read(&mut file, &sevenz_password).map_err(|error| {
                if looks_like_password_error(&error.to_string()) {
                    ArchiveError::PasswordRequired
                } else {
                    ArchiveError::Corrupt(format!("读取 7z 头部失败: {error}"))
                }
            })?;
        let mut entries = Vec::new();
        for file in &archive.files {
            entries.push(ArchiveEntry {
                path: file.name.clone(),
                size: file.size,
                compressed_size: file.compressed_size,
                is_dir: file.is_directory(),
                is_encrypted: password.is_some(),
                modified: None,
            });
        }
        Ok(entries)
    }

    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        std::fs::create_dir_all(ctx.dest)?;

        // 7z 头部含每个条目的未压缩大小，先读取计算任务总量，用于整体进度。
        let total_bytes = {
            let mut header_file = std::fs::File::open(ctx.source)?;
            let sevenz_password = match ctx.options.password.as_deref() {
                Some(value) => sevenz_rust2::Password::from(value),
                None => sevenz_rust2::Password::empty(),
            };
            match sevenz_rust2::Archive::read(&mut header_file, &sevenz_password) {
                Ok(archive) => archive
                    .files
                    .iter()
                    .filter(|f| !f.is_directory())
                    .map(|f| f.size)
                    .sum(),
                Err(_) => 0,
            }
        };
        ctx.progress.on_start(0, total_bytes);

        let mut state = SzState {
            progress: ctx.progress,
            cancel: ctx.cancel,
            dest: ctx.dest,
            overwrite: ctx.options.overwrite,
            idx: 0,
            summary: ExtractSummary::default(),
            cancelled: false,
            abort: None,
            bytes_done: 0,
            total_bytes,
        };
        let file = std::fs::File::open(ctx.source)?;
        let encrypted = ctx.options.password.is_some();
        let result = match ctx.options.password.as_deref() {
            Some(password) => {
                let sevenz_password = sevenz_rust2::Password::from(password);
                sevenz_rust2::decompress_with_extract_fn_and_password(
                    file,
                    ctx.dest,
                    sevenz_password,
                    |entry, reader, _path| sz_callback(entry, reader, &mut state, encrypted),
                )
            }
            None => {
                sevenz_rust2::decompress_with_extract_fn(file, ctx.dest, |entry, reader, _path| {
                    sz_callback(entry, reader, &mut state, encrypted)
                })
            }
        };

        if let Some(abort) = state.abort.take() {
            return Err(abort);
        }
        if state.cancelled {
            state.summary.cancelled = true;
            return Ok(state.summary);
        }
        if let Err(error) = result {
            if looks_like_password_error(&error.to_string()) {
                anyhow::bail!(ArchiveError::WrongPassword);
            }
            return Err(error.into());
        }
        Ok(state.summary)
    }
}

struct SzState<'a> {
    progress: &'a dyn ProgressSink,
    cancel: &'a dyn CancelToken,
    dest: &'a Path,
    overwrite: OverwritePolicy,
    idx: usize,
    summary: ExtractSummary,
    cancelled: bool,
    abort: Option<anyhow::Error>,
    bytes_done: u64,
    total_bytes: u64,
}

/// 单条目回调：净化路径、应用覆盖策略、回报进度、检查取消。
/// 返回 `Ok(false)` 中止解压；中止原因写入 `state.abort`，与用户取消分开。
fn sz_callback(
    entry: &sevenz_rust2::ArchiveEntry,
    reader: &mut dyn std::io::Read,
    state: &mut SzState,
    encrypted: bool,
) -> Result<bool, sevenz_rust2::Error> {
    if state.cancel.is_cancelled() {
        state.cancelled = true;
        return Ok(false);
    }
    state.summary.entries_total += 1;

    let name = entry.name().to_string();
    let is_directory = entry.is_directory();
    let size = entry.size;
    let metadata = ArchiveEntry {
        path: name.clone(),
        size,
        compressed_size: entry.compressed_size,
        is_dir: is_directory,
        is_encrypted: encrypted,
        modified: None,
    };
    state.progress.on_entry_start(state.idx, 0, &metadata);

    // 先做纯路径校验；非法路径必须先消费数据流以保持库的流对齐，再跳过。
    if sanitize_entry_path(&name, state.dest).is_err() {
        drain_reader(reader, state, size);
        state.idx += 1;
        return Ok(true);
    }

    if is_directory {
        let relative = sanitize_entry_path(&name, state.dest).expect("已校验");
        if let Err(error) = ensure_safe_directory(state.dest, &relative) {
            state.abort = Some(error);
            return Ok(false);
        }
        state.summary.entries_extracted += 1;
        state.progress.on_entry_done(state.idx, 0);
        state.idx += 1;
        return Ok(true);
    }

    match prepare_output(state.dest, &name, state.overwrite) {
        Ok(Some(mut output)) => {
            match copy_stream(
                reader,
                output.file_mut(),
                state.progress,
                state.cancel,
                state.bytes_done,
                size,
                state.total_bytes,
            ) {
                Ok(bytes) => {
                    if let Err(error) = output.commit() {
                        state.abort = Some(error);
                        return Ok(false);
                    }
                    state.summary.bytes_written += bytes;
                    state.summary.entries_extracted += 1;
                    state.progress.on_entry_done(state.idx, bytes);
                    state.bytes_done += bytes;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                    // Drop 清理临时文件，既有目标保持不变。
                    state.cancelled = true;
                    return Ok(false);
                }
                Err(error) => {
                    state.abort = Some(error.into());
                    return Ok(false);
                }
            }
        }
        Ok(None) => {
            drain_reader(reader, state, size);
        }
        Err(error) => {
            // Skip/Overwrite/Rename 决策失败（如 Error 策略命中冲突）视为中止错误，
            // 不能静默覆盖或伪装成取消。先消费该条目数据流再中止。
            drain_reader(reader, state, size);
            state.abort = Some(error);
            return Ok(false);
        }
    }
    state.idx += 1;
    Ok(true)
}

fn drain_reader(reader: &mut dyn std::io::Read, state: &mut SzState, size: u64) {
    let _ = std::io::copy(reader, &mut std::io::sink());
    state.summary.entries_skipped += 1;
    state.progress.on_entry_done(state.idx, 0);
    // 跳过的条目仍计入累计进度，保证整体百分比连续。
    state.bytes_done += size;
    state
        .progress
        .on_progress(state.bytes_done, state.total_bytes);
}

fn looks_like_password_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("password") || lower.contains("encrypt")
}
