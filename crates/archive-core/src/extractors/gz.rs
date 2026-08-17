//! 单文件压缩流解压器（gzip / bzip2 / xz / zstd）。
//! 解压成去掉压缩扩展名后的单文件。

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::extractors::copy_stream;
use crate::safety::prepare_output;
use crate::traits::{ArchiveExtractor, ExtractContext};
use crate::types::{ArchiveEntry, ArchiveFormat, ExtractSummary};

pub enum GzExtractor {
    Gzip,
    Bzip2,
    Xz,
    Zstd,
}

impl GzExtractor {
    fn kind(&self) -> ArchiveFormat {
        match self {
            GzExtractor::Gzip => ArchiveFormat::Gzip,
            GzExtractor::Bzip2 => ArchiveFormat::Bzip2,
            GzExtractor::Xz => ArchiveFormat::Xz,
            GzExtractor::Zstd => ArchiveFormat::Zstd,
        }
    }
}

impl ArchiveExtractor for GzExtractor {
    fn format_kind(&self) -> ArchiveFormat {
        self.kind()
    }

    fn list(&self, path: &Path, _password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>> {
        let output_name = strip_compound_ext(path);
        let compressed_size = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        Ok(vec![ArchiveEntry {
            path: output_name,
            size: 0,
            compressed_size,
            is_dir: false,
            is_encrypted: false,
            modified: None,
        }])
    }

    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        let output_name = strip_compound_ext(ctx.source);
        let Some(mut output) = prepare_output(ctx.dest, &output_name, ctx.options.overwrite)?
        else {
            return Ok(ExtractSummary {
                entries_total: 1,
                entries_skipped: 1,
                ..Default::default()
            });
        };

        // 单文件压缩流通常无法预知解压后大小，total_bytes=0 表示不确定。
        ctx.progress.on_start(1, 0);
        ctx.progress.on_entry_start(
            0,
            1,
            &ArchiveEntry {
                path: output_name,
                ..Default::default()
            },
        );

        let input = fs::File::open(ctx.source)?;
        // 压缩后大小用于压缩比校验（解压炸弹防护：流式格式无法预知总量，用 ratio 兜底）。
        let compressed_size = fs::metadata(ctx.source).map(|m| m.len()).unwrap_or(0);
        let decoder: Box<dyn Read> = match self {
            GzExtractor::Gzip => Box::new(flate2::read::GzDecoder::new(input)),
            GzExtractor::Bzip2 => Box::new(bzip2::read::BzDecoder::new(input)),
            GzExtractor::Xz => Box::new(xz2::read::XzDecoder::new(input)),
            GzExtractor::Zstd => Box::new(zstd::stream::Decoder::new(input)?),
        };
        match copy_stream(
            decoder,
            output.file_mut(),
            ctx.progress,
            ctx.cancel,
            0,
            0,
            0,
            &ctx.options.limits,
        ) {
            Ok(bytes) => {
                // 压缩比校验：解压后 / 压缩前 超 max_ratio 视为解压炸弹。
                if let Some(ratio) = bytes.checked_div(compressed_size) {
                    if ratio > ctx.options.limits.max_ratio {
                        // 提交前已超 ratio，拒绝提交，Drop 清理临时文件。
                        return Err(crate::error::ArchiveError::BombDetected {
                            current: bytes,
                            max: ctx.options.limits.max_total_bytes,
                        }
                        .into());
                    }
                }
                output.commit()?;
                ctx.progress.on_entry_done(0, bytes);
                Ok(ExtractSummary {
                    entries_total: 1,
                    entries_extracted: 1,
                    bytes_written: bytes,
                    ..Default::default()
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                // Drop 清理临时文件，既有目标不被截断或删除。
                Ok(ExtractSummary {
                    entries_total: 1,
                    cancelled: true,
                    ..Default::default()
                })
            }
            Err(error) => Err(error.into()),
        }
    }
}

/// 去掉压缩扩展名得到解压后文件名。
fn strip_compound_ext(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let lowercase = name.to_ascii_lowercase();
    for extension in [
        ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".gzip", ".zstd", ".gz", ".bz2", ".xz",
        ".zst",
    ] {
        if lowercase.ends_with(extension) {
            return name[..name.len() - extension.len()].to_string();
        }
    }
    format!("{name}.out")
}
