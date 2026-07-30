//! TAR 归档创建器（含 gzip/xz 压缩）。

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Context;

use crate::error::ArchiveError;
use crate::extractors::COPY_BUF;
use crate::traits::{ArchiveCreator, CreateContext, CreateSource};
use crate::types::{ArchiveFormat, ExtractSummary};

#[derive(Clone, Copy)]
enum TarCompression {
    None,
    Gzip,
    Xz,
}

pub struct TarCreator {
    compression: TarCompression,
}

impl TarCreator {
    pub fn plain() -> Self {
        Self {
            compression: TarCompression::None,
        }
    }
    pub fn gzip() -> Self {
        Self {
            compression: TarCompression::Gzip,
        }
    }
    pub fn xz() -> Self {
        Self {
            compression: TarCompression::Xz,
        }
    }
}

impl ArchiveCreator for TarCreator {
    fn format_kind(&self) -> ArchiveFormat {
        match self.compression {
            TarCompression::None => ArchiveFormat::Tar,
            TarCompression::Gzip => ArchiveFormat::TarGz,
            TarCompression::Xz => ArchiveFormat::TarXz,
        }
    }

    fn create(&self, ctx: &CreateContext) -> anyhow::Result<ExtractSummary> {
        let file = fs::File::create(ctx.dest)
            .with_context(|| format!("创建归档失败: {}", ctx.dest.display()))?;
        let level = ctx.options.level.unwrap_or(6).clamp(1, 9);

        // 用 boxed writer 适配三种压缩后端。
        let writer: Box<dyn Write> = match self.compression {
            TarCompression::None => Box::new(file),
            TarCompression::Gzip => Box::new(flate2::write::GzEncoder::new(
                file,
                flate2::Compression::new(level as u32),
            )),
            TarCompression::Xz => Box::new(xz2::write::XzEncoder::new(file, level as u32)),
        };
        let mut builder = tar::Builder::new(writer);
        // 安全：不跟随符号链接，防归档内植入指向外部的符号链接导致敏感文件被打包。
        builder.follow_symlinks(false);

        // 统计总字节用于进度。
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
            let safe = crate::safety::sanitize_entry_path(
                src.archive_path.trim_end_matches('/'),
                Path::new(""),
            )?;
            let archive_path = safe.to_string_lossy().to_string();

            if *is_dir || src.archive_path.ends_with('/') {
                builder.append_dir_all(&archive_path, &src.fs_path)?;
                summary.entries_extracted += 1;
                ctx.progress.on_entry_done(index, 0);
                continue;
            }

            let mut input = fs::File::open(&src.fs_path)
                .with_context(|| format!("打开源文件失败: {}", src.fs_path.display()))?;
            let size = fs::metadata(&src.fs_path).map(|m| m.len()).unwrap_or(0);
            ctx.progress.on_entry_start(
                index,
                paths.len(),
                &crate::types::ArchiveEntry {
                    path: archive_path.clone(),
                    size,
                    ..Default::default()
                },
            );

            // tar::Builder::append 流式写入 reader，无法插入取消检查；
            // 用 append_data + 自定义 reader 织入取消。
            let mut header = tar::Header::new_gnu();
            header.set_path(&archive_path)?;
            header.set_size(size);
            header.set_mode(0o644);
            // 保留源文件修改时间，避免解压后全部变成 epoch。
            if let Ok(meta) = fs::metadata(&src.fs_path) {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                        header.set_mtime(dur.as_secs());
                    }
                }
            }
            header.set_cksum();
            let cancel_reader = CancelReader::new(&mut input, ctx.cancel);
            builder.append(&header, cancel_reader)?;

            processed += size;
            summary.bytes_written += size;
            summary.entries_extracted += 1;
            ctx.progress.on_progress(processed, total_bytes);
            ctx.progress.on_entry_done(index, size);
        }

        builder.into_inner()?.flush()?;
        Ok(summary)
    }
}

/// 取消感知的 reader 包装。
struct CancelReader<'a, R: Read> {
    inner: &'a mut R,
    cancel: &'a dyn crate::traits::CancelToken,
}

impl<'a, R: Read> CancelReader<'a, R> {
    fn new(inner: &'a mut R, cancel: &'a dyn crate::traits::CancelToken) -> Self {
        Self { inner, cancel }
    }
}

impl<R: Read> Read for CancelReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                ArchiveError::Cancelled.to_string(),
            ));
        }
        self.inner.read(buf)
    }
}

#[allow(dead_code)]
fn _buf() -> [u8; COPY_BUF] {
    [0u8; COPY_BUF]
}
