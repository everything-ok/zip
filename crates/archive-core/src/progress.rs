//! 进度/取消的默认实现与辅助工具。

use std::sync::atomic::{AtomicBool, Ordering};

use crate::traits::{CancelToken, ProgressSink};
use crate::types::ArchiveEntry;

/// 空实现，什么都不做。用于不需要进度的场景（如测试）。
pub struct NoopSink;
impl ProgressSink for NoopSink {}

/// 永不取消的令牌。
pub struct NoopCancel;
impl CancelToken for NoopCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// 基于 `AtomicBool` 的取消令牌，可跨线程设置。
#[derive(Default)]
pub struct AtomicCancel {
    flag: AtomicBool,
}

impl AtomicCancel {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
}

impl CancelToken for AtomicCancel {
    fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// 检查取消，若已取消则返回 `Err(Cancelled)`。
pub fn check_cancel(cancel: &dyn CancelToken) -> anyhow::Result<()> {
    if cancel.is_cancelled() {
        anyhow::bail!(crate::error::ArchiveError::Cancelled);
    }
    Ok(())
}

/// 累计式字节进度 sink：每达到阈值或时间间隔才回调一次。
/// 这里提供一个简单的"每 entry 回报"策略供纯 Rust 测试用；
/// 真正的节流由 Tauri 层 `ChannelSink` 负责。
pub struct CountingSink {
    pub bytes: std::sync::atomic::AtomicU64,
}

impl CountingSink {
    pub fn new() -> Self {
        Self {
            bytes: std::sync::atomic::AtomicU64::new(0),
        }
    }
    pub fn total(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }
}

impl Default for CountingSink {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressSink for CountingSink {
    fn on_entry_done(&self, _index: usize, bytes_written: u64) {
        self.bytes.fetch_add(bytes_written, Ordering::Relaxed);
    }
    fn on_progress(&self, processed: u64, _total: u64) {
        // 直接用 processed 覆盖式记录（更准确）
        self.bytes.store(processed, Ordering::Relaxed);
    }
}

/// 让 `&ArchiveEntry` 也能被 sink 当作"开始一个条目"用，保持 trait 简洁。
#[allow(dead_code)]
fn _entry_type_check(_e: &ArchiveEntry) {}
