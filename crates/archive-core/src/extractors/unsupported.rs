//! 未启用 `rar` feature 时对 RAR 的占位实现（返回明确错误而非编译失败）。

use std::path::Path;

use crate::error::ArchiveError;
use crate::traits::{ArchiveExtractor, ExtractContext};
use crate::types::{ArchiveEntry, ArchiveFormat, ExtractSummary};

pub struct UnsupportedExtractor(pub ArchiveFormat);

impl ArchiveExtractor for UnsupportedExtractor {
    fn format_kind(&self) -> ArchiveFormat {
        self.0
    }

    fn list(&self, _path: &Path, _password: Option<&str>) -> anyhow::Result<Vec<ArchiveEntry>> {
        Err(ArchiveError::Unsupported(format!(
            "{} 需要启用 `rar` feature（编译时 --features rar）",
            self.0
        ))
        .into())
    }

    fn extract(&self, _ctx: &ExtractContext) -> anyhow::Result<ExtractSummary> {
        Err(ArchiveError::Unsupported(format!(
            "{} 需要启用 `rar` feature（编译时 --features rar）",
            self.0
        ))
        .into())
    }
}
