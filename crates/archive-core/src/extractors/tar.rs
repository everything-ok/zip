//! TAR 及压缩 TAR（.tar.gz/.bz2/.xz/.zst）解压器。

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::extractors::copy_stream;
use crate::safety::{ensure_safe_directory, prepare_output, sanitize_entry_path};
use crate::traits::{ArchiveExtractor, ExtractContext};
use crate::types::{ArchiveEntry, ArchiveFormat, ExtractSummary};

#[derive(Clone, Copy)]
enum Compression {
    None,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
}

pub struct TarExtractor {
    compression: Compression,
}

impl TarExtractor {
    pub fn plain() -> Self {
        Self {
            compression: Compression::None,
        }
    }

    pub fn gzip() -> Self {
        Self {
            compression: Compression::Gzip,
        }
    }

    pub fn bzip2() -> Self {
        Self {
            compression: Compression::Bzip2,
        }
    }

    pub fn xz() -> Self {
        Self {
            compression: Compression::Xz,
        }
    }

    pub fn zstd() -> Self {
        Self {
            compression: Compression::Zstd,
        }
    }

    fn wrap_reader<R: Read + 'static>(&self, reader: R) -> anyhow::Result<Box<dyn Read>> {
        Ok(match self.compression {
            Compression::None => Box::new(reader),
            Compression::Gzip => Box::new(flate2::read::GzDecoder::new(reader)),
            Compression::Bzip2 => Box::new(bzip2::read::BzDecoder::new(reader)),
            Compression::Xz => Box::new(xz2::read::XzDecoder::new(reader)),
            Compression::Zstd => Box::new(zstd::stream::Decoder::new(reader)?),
        })
    }
}

impl ArchiveExtractor for TarExtractor {
    fn format_kind(&self) -> ArchiveFormat {
        match self.compression {
            Compression::None => ArchiveFormat::Tar,
            Compression::Gzip => ArchiveFormat::TarGz,
            Compression::Bzip2 => ArchiveFormat::TarBz2,
            Compression::Xz => ArchiveFormat::TarXz,
            Compression::Zstd => ArchiveFormat::TarZst,
        }
    }

    fn list(&self, path: &Path, _password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>> {
        let file = fs::File::open(path)?;
        let mut archive = tar::Archive::new(self.wrap_reader(file)?);
        let mut entries = Vec::new();
        for entry in archive.entries()? {
            let entry = entry?;
            let header = entry.header();
            entries.push(ArchiveEntry {
                path: entry.path()?.to_string_lossy().to_string(),
                size: entry.size(),
                compressed_size: 0,
                is_dir: header.entry_type().is_dir(),
                is_encrypted: false,
                modified: header.mtime().ok(),
            });
        }
        Ok(entries)
    }

    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        let file = fs::File::open(ctx.source)?;
        let mut archive = tar::Archive::new(self.wrap_reader(file)?);
        // TAR 是流格式，开始时无法可靠得知条目/字节总数；total_bytes=0 表示不确定。
        ctx.progress.on_start(0, 0);

        let mut summary = ExtractSummary::default();
        let mut bytes_done = 0_u64;
        let mut index = 0_usize;
        for entry in archive.entries()? {
            if ctx.cancel.is_cancelled() {
                summary.cancelled = true;
                return Ok(summary);
            }
            let mut entry = entry?;
            let header = entry.header().clone();
            let entry_type = header.entry_type();
            let path = entry.path()?.to_string_lossy().to_string();
            let size = entry.size();
            let metadata = ArchiveEntry {
                path: path.clone(),
                size,
                compressed_size: 0,
                is_dir: entry_type.is_dir(),
                is_encrypted: false,
                modified: header.mtime().ok(),
            };
            ctx.progress.on_entry_start(index, 0, &metadata);
            summary.entries_total += 1;

            if entry_type.is_dir() {
                let relative = sanitize_entry_path(&path, ctx.dest)?;
                ensure_safe_directory(ctx.dest, &relative)?;
                summary.entries_extracted += 1;
                ctx.progress.on_entry_done(index, 0);
                index += 1;
                continue;
            }

            // 安全白名单：只允许正常文件。符号链接、硬链接、FIFO 与设备文件
            // 默认全部跳过，避免路径逃逸和跨平台语义错误。
            if !entry_type.is_file() {
                // `skip_symlinks` 关闭时也不创建链接：当前安全模型只允许目录和
                // 普通文件，避免链接、FIFO、设备文件改变目标目录语义。
                let _is_symlink = entry_type.is_symlink();
                summary.entries_skipped += 1;
                ctx.progress.on_entry_done(index, 0);
                index += 1;
                continue;
            }

            let Some(mut output) = prepare_output(ctx.dest, &path, ctx.options.overwrite)? else {
                summary.entries_skipped += 1;
                ctx.progress.on_entry_done(index, 0);
                index += 1;
                continue;
            };
            // TAR 无总量，task_total=0；回报累计 processed，前端走不确定进度。
            match copy_stream(
                &mut entry,
                output.file_mut(),
                ctx.progress,
                ctx.cancel,
                bytes_done,
                size,
                0,
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
                Err(error) => return Err(error.into()),
            }
            index += 1;
        }
        Ok(summary)
    }
}
