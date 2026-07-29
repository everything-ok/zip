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
            // ZIP 文件名可能非 UTF-8（GBK 中文 ZIP），用 name_raw 探测后回退 GBK。
            entries.push(ArchiveEntry {
                path: decode_zip_name(entry.name_raw()),
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                is_dir: entry.is_dir(),
                is_encrypted: entry.encrypted(),
                modified: None,
            });
        }
        Ok(entries)
    }

    fn supports_test(&self) -> bool {
        true
    }

    fn test(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        // ZIP 测试：遍历读取每个条目到 sink，CRC 不匹配会返回 InvalidCrc。
        let file = fs::File::open(ctx.source)?;
        let mut zip = ZipArchive::new(file)?;
        let password = ctx.options.password.as_deref();
        let total = zip.len();
        ctx.options.limits.check_entries(total)?;
        ctx.progress.on_start(total, 0);
        let mut summary = ExtractSummary {
            entries_total: total,
            ..Default::default()
        };
        for index in 0..total {
            if ctx.cancel.is_cancelled() {
                summary.cancelled = true;
                return Ok(summary);
            }
            let entry_result = match password {
                Some(p) => zip.by_index_decrypt(index, p.as_bytes()),
                None => zip.by_index(index),
            };
            let mut entry = match entry_result {
                Ok(e) => e,
                Err(zip::result::ZipError::InvalidPassword) => {
                    if password.is_some() {
                        anyhow::bail!(ArchiveError::WrongPassword);
                    } else {
                        anyhow::bail!(ArchiveError::PasswordRequired);
                    }
                }
                Err(e) => return Err(e.into()),
            };
            // 读入 sink 触发 CRC 校验。
            let name = decode_zip_name(entry.name_raw());
            ctx.progress.on_entry_start(index, total, &ArchiveEntry {
                path: name,
                size: entry.size(),
                ..Default::default()
            });
            let copied = std::io::copy(&mut entry, &mut std::io::sink())?;
            summary.entries_extracted += 1;
            summary.bytes_written += copied;
            ctx.progress.on_entry_done(index, copied);
        }
        Ok(summary)
    }

    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        self.extract_filtered(ctx, None)
    }

    fn supports_partial(&self) -> bool {
        true
    }

    fn extract_entries(
        &self,
        ctx: &ExtractContext,
        entries: &[String],
    ) -> anyhow::Result<ExtractSummary> {
        // 空集合视作解压全部。
        if entries.is_empty() {
            return self.extract(ctx);
        }
        self.extract_filtered(ctx, Some(entries))
    }
}

impl ZipExtractor {
    /// 解压实现，可选 `filter` 限定只解压匹配的归档内路径。
    /// `filter=None` 解压全部；匹配时目录前缀也一并解压以保留结构。
    fn extract_filtered(
        &self,
        ctx: &ExtractContext,
        filter: Option<&[String]>,
    ) -> anyhow::Result<ExtractSummary> {
        let file = fs::File::open(ctx.source)?;
        let mut zip = ZipArchive::new(file)?;
        let password = ctx.options.password.as_deref();
        let total = zip.len();

        // 规整过滤集合为带尾斜杠感知的匹配：条目路径相等，或为目录前缀。
        let filter_set: std::collections::HashSet<String> = filter
            .map(|slice| slice.iter().map(|s| normalize_filter(s)).collect())
            .unwrap_or_default();
        let want = |name: &str| -> bool {
            if filter_set.is_empty() {
                return true;
            }
            let norm = name.replace('\\', "/");
            if filter_set.contains(&norm) {
                return true;
            }
            // 目录前缀：若过滤项是目录，其下文件也应包含。
            filter_set.iter().any(|f| {
                norm.starts_with(f.trim_end_matches('/').to_string().as_str())
                    && (f.ends_with('/') || norm.as_bytes().get(f.len()) == Some(&b'/'))
            })
        };

        // ZIP 可随机访问，先计算所有非目录条目的未压缩总大小，用于整体进度。
        let mut total_bytes = 0_u64;
        let mut matched = 0usize;
        for index in 0..total {
            if let Ok(entry) = zip.by_index_raw(index) {
                if !entry.is_dir() && want(&decode_zip_name(entry.name_raw())) {
                    total_bytes += entry.size();
                    matched += 1;
                }
            }
        }
        // 条目数上限校验，防海量条目耗尽资源。
        ctx.options.limits.check_entries(total)?;
        ctx.progress.on_start(matched, total_bytes);

        let mut summary = ExtractSummary {
            entries_total: matched,
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
                    // 已提供密码却 InvalidPassword → 密码错误；未提供 → 需要密码。
                    if password.is_some() {
                        anyhow::bail!(ArchiveError::WrongPassword);
                    } else {
                        anyhow::bail!(ArchiveError::PasswordRequired);
                    }
                }
                Err(error) => {
                    if error.to_string().to_ascii_lowercase().contains("password") {
                        if password.is_some() {
                            anyhow::bail!(ArchiveError::WrongPassword);
                        } else {
                            anyhow::bail!(ArchiveError::PasswordRequired);
                        }
                    }
                    return Err(error.into());
                }
            };
            let name = decode_zip_name(entry.name_raw());
            if !want(&name) {
                continue;
            }
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
            ctx.progress.on_entry_start(index, matched, &metadata);

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
                &ctx.options.limits,
            ) {
                Ok(bytes) => {
                    output.commit()?;
                    summary.bytes_written += bytes;
                    summary.entries_extracted += 1;
                    ctx.progress.on_entry_done(index, bytes);
                    bytes_done += bytes;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
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

/// 规整过滤路径：统一正斜杠，去前导斜杠，目录补尾斜杠。
fn normalize_filter(s: &str) -> String {
    let mut norm = s.replace('\\', "/");
    while norm.starts_with('/') {
        norm.remove(0);
    }
    norm
}

/// 解码 ZIP 条目名：优先 UTF-8，失败回退 GBK（中文 ZIP 常见编码）。
/// zip crate 默认按 cp437 解码非 UTF-8 名，导致中文乱码。
fn decode_zip_name(raw: &[u8]) -> String {
    // 若字节是合法 UTF-8，直接用。
    if let Ok(s) = std::str::from_utf8(raw) {
        return s.to_string();
    }
    // 否则按 GBK 解码（encoding_rs 无 BOM 检测，直接 GBK）。
    let (cow, _, _) = encoding_rs::GBK.decode(raw);
    cow.into_owned()
}

#[cfg(test)]
mod tests {
    use super::decode_zip_name;

    #[test]
    fn utf8_name_unchanged() {
        assert_eq!(decode_zip_name("hello.txt".as_bytes()), "hello.txt");
        assert_eq!(decode_zip_name("测试.txt".as_bytes()), "测试.txt");
    }

    #[test]
    fn gbk_name_decoded() {
        // "测试.txt" GBK 字节。
        let gbk_bytes = b"\xb2\xe2\xca\xd4.txt";
        assert_eq!(decode_zip_name(gbk_bytes), "测试.txt");
    }
}

