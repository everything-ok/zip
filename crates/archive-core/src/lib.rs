//! archive-core: 跨平台、无 UI / 无运行时依赖的归档解压核心库。
//!
//! 设计目标：
//! - 全部解压逻辑集中于此，不依赖 tauri / tokio，可纯 `#[test]` 单测；
//! - 通过 `ArchiveExtractor` trait + `ProgressSink` / `CancelToken` 依赖倒置，
//!   让上层（Tauri、未来鸿蒙 NAPI 等）以同步阻塞方式调用并桥接进度/取消；
//! - 安全：统一 `safety::sanitize_entry_path` 防 Zip Slip，流式 256KB 缓冲防 OOM。

pub mod creators;
pub mod detect;
pub mod dispatcher;
pub mod error;
pub mod extractors;
pub mod progress;
pub mod safety;
pub mod traits;
pub mod types;

// `unrar_sys` 静态编译 RARLab C++ 源码时没有主动声明这两个 Windows
// 系统库；RAR 的注册表、令牌与加密 API 分别来自 Advapi32/Crypt32。
// 显式链接保证启用 `rar` feature 的测试与最终 Tauri 产物都能正常链接。
#[cfg(all(windows, feature = "rar"))]
#[link(name = "advapi32")]
unsafe extern "system" {}

#[cfg(all(windows, feature = "rar"))]
#[link(name = "crypt32")]
unsafe extern "system" {}

pub use error::ArchiveError;
pub use traits::{
    ArchiveCreator, ArchiveExtractor, CancelToken, CreateContext, CreateOptions, CreateSource,
    ExtractContext, ProgressSink,
};
pub use types::{
    ArchiveEntry, ArchiveFormat, ExtractLimits, ExtractOptions, ExtractSummary, OverwritePolicy,
};
