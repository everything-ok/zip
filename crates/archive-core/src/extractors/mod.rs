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

use crate::traits::{CancelToken, ProgressSink};

/// 流式拷贝缓冲大小：256KB。内存占用与文件大小无关。
pub const COPY_BUF: usize = 256 * 1024;

/// 流式拷贝：固定缓冲，每轮回调任务累计进度与检查取消。
///
/// `bytes_before` 是本条目开始前任务已累计的字节数；`entry_total` 是本条目
/// 未压缩大小（可能为 0）。回报的 `processed = bytes_before + written`，
/// `total` 由调用方传入的任务总量提供，保证前端看到的是整体进度。
pub fn copy_stream<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    sink: &dyn ProgressSink,
    cancel: &dyn CancelToken,
    bytes_before: u64,
    _entry_total: u64,
    task_total: u64,
) -> std::io::Result<u64> {
    let mut buf = vec![0u8; COPY_BUF];
    let mut written = 0u64;
    loop {
        if cancel.is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                crate::error::ArchiveError::Cancelled.to_string(),
            ));
        }
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        writer.write_all(&buf[..read])?;
        written += read as u64;
        sink.on_progress(bytes_before + written, task_total);
    }
    writer.flush()?;
    Ok(written)
}
