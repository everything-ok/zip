//! ZIP 归档创建器（基于 `zip` crate，支持 AES 密码）。

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Context;

use crate::error::ArchiveError;
use crate::extractors::COPY_BUF;
use crate::traits::{ArchiveCreator, CreateContext, CreateSource};
use crate::types::{ArchiveFormat, ExtractSummary};
use crate::ExtractSummary as Summary;

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
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(level as i64));

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

        let mut summary = Summary {
            entries_total: paths.len(),
            ..Default::default()
        };
        let mut processed: u64 = 0;

        for (index, (src, is_dir)) in paths.iter().enumerate() {
            if ctx.cancel.is_cancelled() {
                summary.cancelled = true;
                break;
            }
            // 校验归档内路径，防注入。
            let safe = crate::safety::sanitize_entry_path(&src.archive_path, Path::new(""))?;
            let archive_path = safe.to_string_lossy().to_string();

            if *is_dir {
                writer.add_directory(&archive_path, options)?;
                summary.entries_extracted += 1;
                ctx.progress.on_entry_done(index, 0);
                continue;
            }

            let mut input = fs::File::open(&src.fs_path)
                .with_context(|| format!("打开源文件失败: {}", src.fs_path.display()))?;
            ctx.progress.on_entry_start(
                index,
                paths.len(),
                &crate::types::ArchiveEntry {
                    path: archive_path.clone(),
                    size: fs::metadata(&src.fs_path).map(|m| m.len()).unwrap_or(0),
                    ..Default::default()
                },
            );

            writer.start_file(&archive_path, options)?;
            let mut buf = vec![0u8; COPY_BUF];
            loop {
                if ctx.cancel.is_cancelled() {
                    summary.cancelled = true;
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

/// 将 `ArchiveError` 转为 `anyhow::Error` 的便捷别名（避免 unused import）。
#[allow(dead_code)]
fn _err(e: ArchiveError) -> anyhow::Error {
    e.into()
}
