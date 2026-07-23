//! 解压核心的 trait 抽象与调用上下文。

use std::path::Path;

use crate::types::{ArchiveEntry, ArchiveFormat, ExtractOptions, ExtractSummary};

/// 进度回调接口。核心不认识任何异步运行时 / Tauri 类型，由上层实现。
/// 所有方法提供默认空实现，便于上层只覆写关心的回调。
///
/// 字节进度语义为“任务累计”：`on_progress` 的 `processed` 是整个任务到目前为止
/// 已解压的字节数，`total` 是整个任务未压缩总字节（未知为 0）。
/// 这样前端可显示整体百分比，而非单个条目从 0 重新开始。
pub trait ProgressSink: Send + Sync {
    /// 任务开始。`total_bytes` 为 0 表示未知总量（如 TAR 流式格式）。
    fn on_start(&self, _total_entries: usize, _total_bytes: u64) {}

    /// 任务累计字节进度。`total=0` 表示未知总量；上层应显示不确定进度。
    /// 节流由实现负责（避免在紧密循环里压垮通道）。
    fn on_progress(&self, _processed: u64, _total: u64) {}

    fn on_entry_start(&self, _index: usize, _total: usize, _entry: &ArchiveEntry) {}
    fn on_entry_done(&self, _index: usize, _bytes_written: u64) {}
    fn on_message(&self, _msg: &str) {}
}

/// 取消令牌接口。
pub trait CancelToken: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

/// 一次解压调用的上下文。
pub struct ExtractContext<'a> {
    pub source: &'a Path,
    pub dest: &'a Path,
    pub options: &'a ExtractOptions,
    pub progress: &'a dyn ProgressSink,
    pub cancel: &'a dyn CancelToken,
}

/// 统一解压器抽象（同步）。由调用方包到 `spawn_blocking` 里执行。
pub trait ArchiveExtractor: Send + Sync {
    fn format_kind(&self) -> ArchiveFormat;

    /// 是否支持密码解压。
    fn supports_password(&self) -> bool {
        false
    }

    /// 仅读取元数据，不解压载荷。
    fn list(&self, path: &Path, password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>>;

    /// 执行解压。通过 `ctx` 拿进度 / 取消。
    fn extract(&self, ctx: &ExtractContext) -> anyhow::Result<ExtractSummary>;
}
