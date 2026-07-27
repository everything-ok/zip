//! 归档创建器实现。

pub mod zip;

use std::path::Path;

use crate::traits::ArchiveCreator;
use crate::types::ArchiveFormat;

/// 根据目标路径扩展名返回对应创建器。
/// 支持 `.zip`；其他扩展名返回错误（7z/tar.gz 留待后续扩展）。
pub fn creator_for_path(dest: &Path) -> anyhow::Result<Box<dyn ArchiveCreator>> {
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".zip") {
        return Ok(Box::new(zip::ZipCreator));
    }
    anyhow::bail!("暂不支持的目标格式（仅支持 .zip）：{}", dest.display())
}

/// 用于错误信息辅助。
pub fn supported_formats() -> &'static [ArchiveFormat] {
    &[ArchiveFormat::Zip]
}
