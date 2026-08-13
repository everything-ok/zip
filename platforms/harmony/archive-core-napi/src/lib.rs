//! archive-core NAPI 导出层
//!
//! 将 archive-core 的 Rust API 导出为 NAPI 函数，供鸿蒙 ArkTS 调用。
//! 编译产物为 libarchive_core_napi.so。

use napi::bindgen_prelude::*;
use napi_derive::napi;

use std::path::PathBuf;

use archive_core::{
    detect, dispatcher, ArchiveExtractor, CancelToken, CreateContext, CreateOptions, CreateSource,
    ExtractContext, ExtractOptions, ExtractSummary, OverwritePolicy, ProgressSink,
};

// ─── NAPI 导出函数 ──────────────────────────────────────────

/// 探测归档格式
#[napi]
pub fn detect_format(path: String) -> Result<String> {
    detect::detect_format(&PathBuf::from(&path))
        .map(|fmt| fmt.to_string())
        .map_err(|e| Error::from_reason(format!("探测格式失败: {e}")))
}

// TODO: list_archive, extract_archive, create_archive, convert_archive, test_archive
// 完整实现需在 ArkTS 侧 UI 就绪后逐步补全

// ─── 内部辅助 ──────────────────────────────────────────────

struct NullSink;
impl ProgressSink for NullSink {
    fn on_entry_start(&self, _index: usize, _total: usize, _path: &str, _size: u64) {}
    fn on_entry_done(&self, _index: usize) {}
    fn on_bytes(&self, _processed: u64, _total: u64, _speed: u64, _eta: Option<u64>) {}
    fn on_finished(&self, _summary: &ExtractSummary) {}
}

struct NoCancel;
impl CancelToken for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}
