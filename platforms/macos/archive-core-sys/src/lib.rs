//! archive-core C ABI 导出层
//!
//! 将 archive-core 的 Rust API 导出为 C 函数，供 macOS Swift FFI 调用。
//! 编译产物为 libarchive_core.dylib。

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

use archive_core::{
    dispatcher, detect, ArchiveEntry, ArchiveExtractor, CancelToken, CreateContext, CreateOptions,
    CreateSource, ExtractContext, ExtractOptions, ExtractSummary, OverwritePolicy, ProgressSink,
};

// ─── 导出函数 ───────────────────────────────────────────────

/// 探测归档格式。返回堆分配 C 字符串，需调用方 `archive_free_string` 释放。
/// 失败返回 NULL。
#[no_mangle]
pub extern "C" fn archive_detect_format(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match detect::detect_format(&PathBuf::from(path_str)) {
        Ok(fmt) => CString::new(fmt.to_string()).unwrap_or_default().into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// 释放 `archive_detect_format` 返回的字符串。
#[no_mangle]
pub extern "C" fn archive_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

// TODO: archive_list, archive_extract, archive_create 等
// 完整实现需在 Swift 侧 UI 就绪后逐步补全

/// 空的进度 sink（占位）
struct NullSink;
impl ProgressSink for NullSink {
    fn on_entry_start(&self, _index: usize, _total: usize, _path: &str, _size: u64) {}
    fn on_entry_done(&self, _index: usize) {}
    fn on_bytes(&self, _processed: u64, _total: u64, _speed: u64, _eta: Option<u64>) {}
    fn on_finished(&self, _summary: &ExtractSummary) {}
}

/// 空的取消令牌（占位）
struct NoCancel;
impl CancelToken for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}
