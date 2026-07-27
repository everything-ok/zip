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

    /// 部分解压：只解压 `entries` 指定的归档内路径集合。
    /// 默认不支持（流式格式无法随机访问）。`entries` 为空时解压全部。
    fn extract_entries(
        &self,
        _ctx: &ExtractContext,
        _entries: &[String],
    ) -> anyhow::Result<ExtractSummary> {
        Err(crate::error::ArchiveError::Unsupported("该格式不支持部分解压".into()).into())
    }

    /// 是否支持部分解压（随机访问）。
    fn supports_partial(&self) -> bool {
        false
    }
}

/// 待加入归档的输入文件/目录条目。
#[derive(Debug, Clone)]
pub struct CreateSource {
    /// 文件系统上的实际路径（文件或目录）。
    pub fs_path: std::path::PathBuf,
    /// 归档内的相对路径（目录用 `dir/` 结尾可省略，创建器按是否目录处理）。
    pub archive_path: String,
}

/// 创建归档的选项。
#[derive(Debug, Clone, Default)]
pub struct CreateOptions {
    /// 加密密码（仅支持加密的格式生效）。
    pub password: Option<String>,
    /// 压缩级别（0=存储，1-9 递增；None 用各格式默认）。
    pub level: Option<i32>,
}

/// 创建归档上下文。
pub struct CreateContext<'a> {
    /// 输出归档路径。
    pub dest: &'a Path,
    /// 源条目列表。
    pub sources: &'a [CreateSource],
    pub options: &'a CreateOptions,
    pub progress: &'a dyn ProgressSink,
    pub cancel: &'a dyn CancelToken,
}

/// 创建归档的抽象。与 `ArchiveExtractor` 对偶。
/// 路径同样经 `safety::sanitize_entry_path` 校验，防注入 `..` 等。
pub trait ArchiveCreator: Send + Sync {
    fn format_kind(&self) -> ArchiveFormat;

    fn supports_password(&self) -> bool {
        false
    }

    /// 创建归档。返回写入字节数统计。
    fn create(&self, ctx: &CreateContext) -> anyhow::Result<ExtractSummary>;
}
