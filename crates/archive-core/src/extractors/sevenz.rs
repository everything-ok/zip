//! 7z 解压器（基于 `sevenz-rust2`，支持 AES-256 密码）。
//! 用 `extract_fn` 回调实现路径净化（防 Zip Slip）、覆盖策略、进度回报与取消。

use std::path::Path;

use crate::error::ArchiveError;
use crate::extractors::copy_stream;
use crate::safety::{ensure_safe_directory, prepare_output, sanitize_entry_path};
use crate::split_reader::SplitReader;
use crate::traits::{ArchiveExtractor, CancelToken, ExtractContext, ProgressSink};
use crate::types::{ArchiveEntry, ArchiveFormat, ExtractSummary, OverwritePolicy};

/// 组合 trait：Read + Seek，用于 trait object。
trait ReadSeek: std::io::Read + std::io::Seek {}
impl<T: std::io::Read + std::io::Seek> ReadSeek for T {}

pub struct SevenZExtractor;

impl ArchiveExtractor for SevenZExtractor {
    fn format_kind(&self) -> ArchiveFormat {
        ArchiveFormat::SevenZ
    }

    fn supports_password(&self) -> bool {
        true
    }

    fn list(&self, path: &Path, password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>> {
        let mut reader = self.open_reader(path)?;
        let sevenz_password = match password {
            Some(value) => sevenz_rust2::Password::from(value),
            None => sevenz_rust2::Password::empty(),
        };
        let archive =
            sevenz_rust2::Archive::read(&mut reader, &sevenz_password).map_err(|error| {
                if looks_like_password_error(&error.to_string()) {
                    ArchiveError::PasswordRequired
                } else {
                    ArchiveError::Corrupt(format!("读取 7z 头部失败: {error}"))
                }
            })?;
        // 从 blocks 检测真实加密标志：任一 block 含 AES256 编码器即视为加密。
        let actually_encrypted = archive.blocks.iter().any(|b| {
            b.coders.iter().any(|c| {
                c.encoder_method_id() == sevenz_rust2::EncoderMethod::ID_AES256_SHA256
            })
        });
        let mut entries = Vec::new();
        for file in &archive.files {
            entries.push(ArchiveEntry {
                path: file.name.clone(),
                size: file.size,
                compressed_size: file.compressed_size,
                is_dir: file.is_directory(),
                is_encrypted: actually_encrypted,
                modified: None,
            });
        }
        Ok(entries)
    }

    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        self.extract_filtered(ctx, &std::collections::HashSet::new())
    }

    fn supports_partial(&self) -> bool {
        true
    }

    fn extract_entries(
        &self,
        ctx: &ExtractContext,
        entries: &[String],
    ) -> anyhow::Result<ExtractSummary> {
        if entries.is_empty() {
            return self.extract(ctx);
        }
        let filter: std::collections::HashSet<String> =
            entries.iter().map(|s| s.replace('\\', "/")).collect();
        self.extract_filtered(ctx, &filter)
    }

    fn supports_test(&self) -> bool {
        true
    }

    fn test(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        // 7z 测试：用 decompress 回调把每个条目 reader 读入 sink，触发内部 CRC 校验，不写盘。
        ctx.progress.on_start(0, 0);
        let mut state = SzState {
            progress: ctx.progress,
            cancel: ctx.cancel,
            dest: ctx.dest,
            overwrite: crate::types::OverwritePolicy::Skip,
            idx: 0,
            summary: ExtractSummary::default(),
            cancelled: false,
            abort: None,
            bytes_done: 0,
            total_bytes: 0,
            limits: &ctx.options.limits,
            filter: &std::collections::HashSet::new(),
        };
        let reader = self.open_reader(ctx.source)?;
        let encrypted = ctx.options.password.is_some();
        let result = match ctx.options.password.as_deref() {
            Some(password) => {
                let sevenz_password = sevenz_rust2::Password::from(password);
                sevenz_rust2::decompress_with_extract_fn_and_password(
                    reader,
                    std::path::Path::new(""),
                    sevenz_password,
                    |entry, reader, _path| sz_test_callback(entry, reader, &mut state, encrypted),
                )
            }
            None => sevenz_rust2::decompress_with_extract_fn(reader, std::path::Path::new(""), |entry, reader, _path| {
                sz_test_callback(entry, reader, &mut state, encrypted)
            }),
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
                anyhow::bail!(crate::error::ArchiveError::WrongPassword);
            }
            return Err(error.into());
        }
        Ok(state.summary)
    }
}

/// 测试模式回调：读入 sink 触发 CRC 校验，不写盘。
fn sz_test_callback(
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
    if let Err(error) = state.limits.check_entries(state.summary.entries_total) {
        state.abort = Some(error.into());
        return Ok(false);
    }
    let name = entry.name().to_string();
    let is_directory = entry.is_directory();
    let size = entry.size;
    state.progress.on_entry_start(
        state.idx,
        0,
        &crate::types::ArchiveEntry {
            path: name,
            size,
            compressed_size: entry.compressed_size,
            is_dir: is_directory,
            is_encrypted: encrypted,
            modified: None,
        },
    );
    if !is_directory {
        match std::io::copy(reader, &mut std::io::sink()) {
            Ok(bytes) => {
                state.summary.bytes_written += bytes;
                state.summary.entries_extracted += 1;
                state.progress.on_entry_done(state.idx, bytes);
            }
            Err(error) => {
                state.abort = Some(error.into());
                return Ok(false);
            }
        }
    } else {
        state.summary.entries_extracted += 1;
        state.progress.on_entry_done(state.idx, 0);
    }
    state.idx += 1;
    Ok(true)
}

impl SevenZExtractor {
    /// 打开 7z 文件读取器。若检测到分卷（.7z.001），用 SplitReader 串联；否则普通 File。
    fn open_reader(&self, path: &Path) -> anyhow::Result<Box<dyn ReadSeek>> {
        let lower = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        // 检测 .7z.001 分卷
        if lower.contains(".7z.") {
            if let Some(idx) = lower.find(".7z.") {
                let after = &lower[idx + 4..];
                if after.chars().all(|c| c.is_ascii_digit()) && !after.is_empty() {
                    if let Ok(sr) = SplitReader::from_first_volume(path) {
                        return Ok(Box::new(sr));
                    }
                }
            }
        }
        Ok(Box::new(std::fs::File::open(path)?))
    }

    fn extract_filtered(
        &self,
        ctx: &ExtractContext,
        filter: &std::collections::HashSet<String>,
    ) -> anyhow::Result<ExtractSummary> {
        std::fs::create_dir_all(ctx.dest)?;

        // 7z 头部含每个条目的未压缩大小，先读取计算任务总量，用于整体进度。
        let total_bytes = {
            let mut header_reader = self.open_reader(ctx.source)?;
            let sevenz_password = match ctx.options.password.as_deref() {
                Some(value) => sevenz_rust2::Password::from(value),
                None => sevenz_rust2::Password::empty(),
            };
            match sevenz_rust2::Archive::read(&mut header_reader, &sevenz_password) {
                Ok(archive) => archive
                    .files
                    .iter()
                    .filter(|f| !f.is_directory())
                    .filter(|f| filter.is_empty() || wants(filter, f.name()))
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
            limits: &ctx.options.limits,
            filter,
        };
        let reader = self.open_reader(ctx.source)?;
        let encrypted = ctx.options.password.is_some();
        let result = match ctx.options.password.as_deref() {
            Some(password) => {
                let sevenz_password = sevenz_rust2::Password::from(password);
                sevenz_rust2::decompress_with_extract_fn_and_password(
                    reader,
                    ctx.dest,
                    sevenz_password,
                    |entry, reader, _path| sz_callback(entry, reader, &mut state, encrypted),
                )
            }
            None => {
                sevenz_rust2::decompress_with_extract_fn(reader, ctx.dest, |entry, reader, _path| {
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
    limits: &'a crate::types::ExtractLimits,
    /// 部分解压过滤集合（空表示全部）。
    filter: &'a std::collections::HashSet<String>,
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
    // 部分解压：非目标条目（含目录前缀）直接消费跳过，不写入目标。
    if !state.filter.is_empty() && !wants(state.filter, &name) {
        let _ = std::io::copy(reader, &mut std::io::sink());
        state.idx += 1;
        return Ok(true);
    }
    let is_directory = entry.is_directory();
    let size = entry.size;
    // 条目数上限校验，防海量条目耗尽资源。
    if let Err(error) = state.limits.check_entries(state.summary.entries_total) {
        state.abort = Some(error.into());
        return Ok(false);
    }
    // 安全白名单：7z 归档可能含符号链接/反条目，当前安全模型只允许目录与普通文件。
    // has_stream=false 且非目录：可能是空文件、链接、或反条目。
    // - 反条目（is_anti_item）跳过；
    // - size==0 的空普通文件创建空文件（与 list 一致）；
    // - 其余（链接等无流条目）跳过防逃逸。
    if !is_directory && !entry.has_stream {
        if entry.is_anti_item() {
            drain_reader(reader, state, size);
            state.idx += 1;
            return Ok(true);
        }
        if size == 0 {
            // 空普通文件：创建空文件。
            match prepare_output(state.dest, &name, state.overwrite) {
                Ok(Some(output)) => {
                    if let Err(error) = output.commit() {
                        state.abort = Some(error);
                        return Ok(false);
                    }
                    state.summary.entries_extracted += 1;
                    state.progress.on_entry_done(state.idx, 0);
                }
                Ok(None) => {
                    state.summary.entries_skipped += 1;
                    state.progress.on_entry_done(state.idx, 0);
                }
                Err(error) => {
                    drain_reader(reader, state, size);
                    state.abort = Some(error);
                    return Ok(false);
                }
            }
            state.idx += 1;
            return Ok(true);
        }
        // 无流非空非目录非反条目：链接/设备，跳过防逃逸。
        drain_reader(reader, state, size);
        state.idx += 1;
        return Ok(true);
    }
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
                state.limits,
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
    // 分块消费数据流，每轮检查取消与上限，防恶意大条目耗尽 CPU/磁盘。
    let mut buf = vec![0u8; crate::extractors::COPY_BUF];
    let mut consumed = 0u64;
    loop {
        if state.cancel.is_cancelled() {
            state.cancelled = true;
            return;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                consumed += n as u64;
                if consumed > state.limits.max_file_bytes {
                    state.abort = Some(
                        crate::error::ArchiveError::FileTooLarge {
                            actual: consumed,
                            max: state.limits.max_file_bytes,
                        }
                        .into(),
                    );
                    return;
                }
                if state.bytes_done + consumed > state.limits.max_total_bytes {
                    state.abort = Some(
                        crate::error::ArchiveError::BombDetected {
                            current: state.bytes_done + consumed,
                            max: state.limits.max_total_bytes,
                        }
                        .into(),
                    );
                    return;
                }
            }
            Err(_) => break,
        }
    }
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

/// 判断归档内路径是否在部分解压的目标集合中。
/// 支持目录前缀：过滤项 `dir/` 时，其下文件 `dir/a.txt` 也匹配。
fn wants(filter: &std::collections::HashSet<String>, name: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let norm = name.replace('\\', "/");
    if filter.contains(&norm) {
        return true;
    }
    filter.iter().any(|f| {
        let base = f.trim_end_matches('/');
        norm.starts_with(base) && norm.as_bytes().get(base.len()) == Some(&b'/')
    })
}
