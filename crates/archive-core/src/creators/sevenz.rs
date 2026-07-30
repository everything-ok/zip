//! 7z 归档创建器（基于 `sevenz-rust2` 的 `ArchiveWriter`，支持 AES-256 加密）。

use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::Context;

use crate::error::ArchiveError;
use crate::extractors::COPY_BUF;
use crate::traits::{ArchiveCreator, CreateContext, CreateSource};
use crate::types::{ArchiveFormat, ExtractSummary};

pub struct SevenZCreator;

impl ArchiveCreator for SevenZCreator {
    fn format_kind(&self) -> ArchiveFormat {
        ArchiveFormat::SevenZ
    }

    fn supports_password(&self) -> bool {
        true
    }

    fn create(&self, ctx: &CreateContext) -> anyhow::Result<ExtractSummary> {
        // ArchiveWriter::create 接受路径，内部创建文件。
        let mut writer = sevenz_rust2::ArchiveWriter::create(ctx.dest)
            .map_err(|e| ArchiveError::Corrupt(format!("创建 7z 失败: {e}")))?;

        // 加密：加密头部 + 内容用 AES256_SHA256 + LZMA2 链式编码。
        let password = ctx.options.password.as_deref();
        let has_password = password.is_some();
        let level = ctx.options.level.unwrap_or(6).clamp(0, 9) as u32;
        let lzma2_cfg = sevenz_rust2::EncoderConfiguration::new(
            sevenz_rust2::EncoderMethod::LZMA2,
        )
        .with_options(sevenz_rust2::encoder_options::EncoderOptions::Lzma2(
            sevenz_rust2::encoder_options::Lzma2Options::from_level(level),
        ));
        if has_password {
            writer.set_encrypt_header(true);
            // 内容方法：LZMA2 压缩 + AES256 加密。
            let pw = sevenz_rust2::Password::from(password.unwrap());
            let methods: Vec<sevenz_rust2::EncoderConfiguration> =
                vec![lzma2_cfg, sevenz_rust2::encoder_options::AesEncoderOptions::new(pw).into()];
            writer.set_content_methods(methods);
        } else {
            writer.set_encrypt_header(false);
            writer.set_content_methods(vec![lzma2_cfg]);
        }

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
            // 校验归档内路径，防注入。
            let safe = crate::safety::sanitize_entry_path(&src.archive_path, Path::new(""))?;
            let archive_path = safe.to_string_lossy().to_string();

            if *is_dir {
                let entry = sevenz_rust2::ArchiveEntry::new_directory(&archive_path);
                writer
                    .push_archive_entry::<&[u8]>(entry, None)
                    .map_err(|e| ArchiveError::Corrupt(format!("写入目录失败: {e}")))?;
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

            // push_archive_entry 内部用 4KB 缓冲读取；为支持取消，先用自定义 reader
            // 包装，在 read 时检查取消。
            let entry = sevenz_rust2::ArchiveEntry::new_file(&archive_path);
            let cancel_reader = CancelReader::new(&mut input, ctx.cancel);
            writer
                .push_archive_entry(entry, Some(cancel_reader))
                .map_err(|e| ArchiveError::Corrupt(format!("写入条目失败: {e}")))?;
            processed += size;
            summary.bytes_written += size;
            summary.entries_extracted += 1;
            ctx.progress.on_progress(processed, total_bytes);
            ctx.progress.on_entry_done(index, size);
        }

        writer
            .finish()
            .map_err(|e| ArchiveError::Corrupt(format!("完成 7z 失败: {e}")))?;
        Ok(summary)
    }
}

/// 取消感知的 reader 包装：读取时检查取消令牌。
/// sevenz-rust2 的 push_archive_entry 接受 `impl Read`，这里把取消检查织入 read。
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

/// 占位：COPY_BUF 在 7z 创建中未直接使用（库内部缓冲），保留导入避免警告。
#[allow(dead_code)]
fn _buf() -> [u8; COPY_BUF] {
    [0u8; COPY_BUF]
}
