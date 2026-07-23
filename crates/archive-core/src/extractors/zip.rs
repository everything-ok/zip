//! ZIP 格式解压器（基于 `zip` crate，支持 AES/ZipCrypto 密码）。

use std::fs;
use std::path::Path;

use anyhow::Context;
use zip::ZipArchive;

use crate::error::ArchiveError;
use crate::extractors::copy_stream;
use crate::safety::{ensure_safe_directory, prepare_output, sanitize_entry_path};
use crate::traits::{ArchiveExtractor, ExtractContext};
use crate::types::{ArchiveEntry, ArchiveFormat, ExtractSummary};

pub struct ZipExtractor;

impl ArchiveExtractor for ZipExtractor {
    fn format_kind(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }

    fn supports_password(&self) -> bool {
        true
    }

    fn list(&self, path: &Path, _password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>> {
        let file = fs::File::open(path).with_context(|| format!("打开失败: {}", path.display()))?;
        // by_index_raw：不读载荷，无需密码即可列出加密归档条目。
        let mut zip = ZipArchive::new(file)?;
        let mut entries = Vec::with_capacity(zip.len());
        for index in 0..zip.len() {
            let entry = zip.by_index_raw(index)?;
            entries.push(ArchiveEntry {
                path: entry.name().to_string(),
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                is_dir: entry.is_dir(),
                is_encrypted: entry.encrypted(),
                modified: None,
            });
        }
        Ok(entries)
    }

    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        let file = fs::File::open(ctx.source)?;
        let mut zip = ZipArchive::new(file)?;
        let password = ctx.options.password.as_deref();
        let total = zip.len();

        // ZIP 可随机访问，先计算所有非目录条目的未压缩总大小，用于整体进度。
        let mut total_bytes = 0_u64;
        for index in 0..total {
            if let Ok(entry) = zip.by_index_raw(index) {
                if !entry.is_dir() {
                    total_bytes += entry.size();
                }
            }
        }
        ctx.progress.on_start(total, total_bytes);

        let mut summary = ExtractSummary {
            entries_total: total,
            ..Default::default()
        };
        let mut bytes_done = 0_u64;

        for index in 0..total {
            if ctx.cancel.is_cancelled() {
                summary.cancelled = true;
                return Ok(summary);
            }
            let entry_result = match password {
                Some(password) => zip.by_index_decrypt(index, password.as_bytes()),
                None => zip.by_index(index),
            };
            let mut entry = match entry_result {
                Ok(entry) => entry,
                Err(zip::result::ZipError::InvalidPassword) => {
                    anyhow::bail!(ArchiveError::PasswordRequired)
                }
                Err(error) => {
                    if error.to_string().to_ascii_lowercase().contains("password") {
                        anyhow::bail!(ArchiveError::PasswordRequired);
                    }
                    return Err(error.into());
                }
            };
            let name = entry.name().to_string();
            let size = entry.size();
            let is_directory = entry.is_dir();
            let metadata = ArchiveEntry {
                path: name.clone(),
                size,
                compressed_size: entry.compressed_size(),
                is_dir: is_directory,
                is_encrypted: entry.encrypted(),
                modified: None,
            };
            ctx.progress.on_entry_start(index, total, &metadata);

            if is_directory {
                let relative = sanitize_entry_path(&name, ctx.dest)?;
                ensure_safe_directory(ctx.dest, &relative)?;
                summary.entries_extracted += 1;
                ctx.progress.on_entry_done(index, 0);
                continue;
            }

            let Some(mut output) = prepare_output(ctx.dest, &name, ctx.options.overwrite)? else {
                summary.entries_skipped += 1;
                ctx.progress.on_entry_done(index, 0);
                bytes_done += size;
                ctx.progress.on_progress(bytes_done, total_bytes);
                continue;
            };
            match copy_stream(
                &mut entry,
                output.file_mut(),
                ctx.progress,
                ctx.cancel,
                bytes_done,
                size,
                total_bytes,
            ) {
                Ok(bytes) => {
                    output.commit()?;
                    summary.bytes_written += bytes;
                    summary.entries_extracted += 1;
                    ctx.progress.on_entry_done(index, bytes);
                    bytes_done += bytes;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                    // Drop 清理临时文件；既有目标文件没有被打开或截断。
                    summary.cancelled = true;
                    return Ok(summary);
                }
                Err(error) => {
                    if error.to_string().to_ascii_lowercase().contains("password") {
                        anyhow::bail!(ArchiveError::WrongPassword);
                    }
                    return Err(error.into());
                }
            }
        }
        Ok(summary)
    }
}
