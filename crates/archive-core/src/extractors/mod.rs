//! 各格式 extractor 实现 + 共享 IO 工具。

pub mod gz;
#[cfg(feature = "rar")]
pub mod rar;
pub mod sevenz;
pub mod tar;
pub mod zip;

#[cfg(not(feature = "rar"))]
pub mod unsupported;

use std::io::{Read, Write};

use crate::error::ArchiveError;
use crate::traits::{CancelToken, ProgressSink};
use crate::types::ExtractLimits;

/// 流式拷贝缓冲大小：256KB。内存占用与文件大小无关。
pub const COPY_BUF: usize = 256 * 1024;

/// 流式拷贝：固定缓冲，每轮回调任务累计进度、检查取消，并校验解压炸弹上限。
///
/// - `bytes_before`：本条目开始前任务已累计的字节数。
/// - `entry_total`：本条目未压缩大小（可能为 0，未知）。
/// - `task_total`：任务总量，用于整体进度。
/// - `limits`：解压上限。每轮检查累计字节是否超 `max_total_bytes`；
///   若 `entry_total` 已知且超 `max_file_bytes`，则在拷贝前就拒绝。
///
/// 超限返回封装为 `io::Error` 的 `ArchiveError::BombDetected` / `FileTooLarge`，
/// 调用方据此中止并清理临时文件，不会损坏既有目标。
#[allow(clippy::too_many_arguments)]
pub fn copy_stream<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    sink: &dyn ProgressSink,
    cancel: &dyn CancelToken,
    bytes_before: u64,
    entry_total: u64,
    task_total: u64,
    limits: &ExtractLimits,
) -> std::io::Result<u64> {
    // 已知单文件大小时先校验，避免无谓拷贝。
    if entry_total > 0 {
        limits
            .check_file_size(entry_total)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    }
    let mut buf = vec![0u8; COPY_BUF];
    let mut written = 0u64;
    loop {
        if cancel.is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                ArchiveError::Cancelled.to_string(),
            ));
        }
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        written += read as u64;
        // 累计字节与单文件实际大小双重校验。
        if written > limits.max_file_bytes {
            return Err(std::io::Error::other(
                ArchiveError::FileTooLarge {
                    actual: written,
                    max: limits.max_file_bytes,
                }
                .to_string(),
            ));
        }
        if bytes_before + written > limits.max_total_bytes {
            return Err(std::io::Error::other(
                ArchiveError::BombDetected {
                    current: bytes_before + written,
                    max: limits.max_total_bytes,
                }
                .to_string(),
            ));
        }
        writer.write_all(&buf[..read])?;
        sink.on_progress(bytes_before + written, task_total);
    }
    writer.flush()?;
    Ok(written)
}
