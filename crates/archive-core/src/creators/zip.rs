//! ZIP 归档创建器（基于 `zip` crate，支持 AES-256 密码）。

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Context;

use crate::extractors::COPY_BUF;
use crate::traits::{ArchiveCreator, CreateContext, CreateSource};
use crate::types::{ArchiveFormat, ExtractSummary};

pub struct ZipCreator;

impl ArchiveCreator for ZipCreator {
    fn format_kind(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }

    fn supports_password(&self) -> bool {
        true
    }

    fn create(&self, ctx: &CreateContext) -> anyhow::Result<ExtractSummary> {
        let file = fs::File::create(ctx.dest)
            .with_context(|| format!("创建归档失败: {}", ctx.dest.display()))?;
        let mut writer = zip::ZipWriter::new(file);
        // 压缩级别：0=存储，1-9；默认 6。
        let level = ctx.options.level.unwrap_or(6).clamp(0, 9);
        let mut options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(level as i64));
        // 加密：优先 AES-256（zip crate aes-crypto feature 已启用）。
        if let Some(password) = ctx.options.password.as_deref() {
            options = options.with_aes_encryption(zip::AesMode::Aes256, password);
        }

        // 先统计总字节用于进度。
        let mut total_bytes = 0u64;
        let mut paths: Vec<(&CreateSource, bool)> = Vec::new();
        for src in ctx.sources {
            let is_dir = src.fs_path.is_dir();
            paths.push((src, is_dir));
            if !is_dir {
                total_bytes += fs::metadata(&src.fs_path).map(|m| m.len()).unwrap_or(0);
            }
        }
        ctx.progress.on_start(paths.len(), total_bytes);

        let mut summary = ExtractSummary {
            entries_total: paths.len(),
            ..Default::default()
        };
        let mut processed: u64 = 0;

        for (index, (src, is_dir)) in paths.iter().enumerate() {
            if ctx.cancel.is_cancelled() {
                summary.cancelled = true;
                break;
            }
            // 校验归档内路径，防注入。末尾 '/' 标记空目录，去掉后创建目录条目。
            let clean_path = src.archive_path.trim_end_matches('/');
            let safe = crate::safety::sanitize_entry_path(clean_path, Path::new(""))?;
            let archive_path = safe.to_string_lossy().to_string();

            if *is_dir || src.archive_path.ends_with('/') {
                writer.add_directory(&archive_path, options)?;
                summary.entries_extracted += 1;
                ctx.progress.on_entry_done(index, 0);
                continue;
            }

            let mut input = fs::File::open(&src.fs_path)
                .with_context(|| format!("打开源文件失败: {}", src.fs_path.display()))?;
            // 条目级时间：从源文件元数据取修改时间填入 ZIP options。
            let entry_options = {
                let mut o = options;
                if let Ok(meta) = fs::metadata(&src.fs_path) {
                    if let Ok(mtime) = meta.modified() {
                        if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                            let secs = dur.as_secs() as i64;
                            // 把 unix 秒转年月日时分秒（zip crate DateTime 无 SystemTime 直转）。
                            let days = secs / 86400;
                            let rem = secs % 86400;
                            let hour = (rem / 3600) as u32;
                            let min = ((rem % 3600) / 60) as u32;
                            let sec = (rem % 60) as u32;
                            // 1970-01-01 起算的天数转年月日（民用算法）。
                            let (y, mo, d) = days_to_ymd(days);
                            if let Ok(dt) = zip::DateTime::from_date_and_time(
                                y as u16,
                                mo as u8,
                                d as u8,
                                hour as u8,
                                min as u8,
                                sec as u8,
                            ) {
                                o = o.last_modified_time(dt);
                            }
                        }
                    }
                }
                o
            };
            ctx.progress.on_entry_start(
                index,
                paths.len(),
                &crate::types::ArchiveEntry {
                    path: archive_path.clone(),
                    size: fs::metadata(&src.fs_path).map(|m| m.len()).unwrap_or(0),
                    ..Default::default()
                },
            );

            writer.start_file(&archive_path, entry_options)?;
            let mut buf = vec![0u8; COPY_BUF];
            loop {
                if ctx.cancel.is_cancelled() {
                    summary.cancelled = true;
                    // 取消时目标文件无中央目录已损坏，删除避免残留无法打开的半成品。
                    let _ = writer.finish(); // 尽量 flush，但文件仍可能不完整
                    let _ = fs::remove_file(ctx.dest);
                    return Ok(summary);
                }
                let n = input.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                writer.write_all(&buf[..n])?;
                processed += n as u64;
                summary.bytes_written += n as u64;
                ctx.progress.on_progress(processed, total_bytes);
            }
            summary.entries_extracted += 1;
            ctx.progress.on_entry_done(index, 0);
        }

        writer.finish()?;
        Ok(summary)
    }
}

/// 把 1970-01-01 起的天数转为 (年, 月, 日)。民用公历算法，1900+ 适用。
fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let days = days + 719162; // 0000-03-01 为基准
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}
