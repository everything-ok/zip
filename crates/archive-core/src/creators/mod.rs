//! 归档创建器实现。

pub mod sevenz;
pub mod tar;
pub mod zip;

use std::path::Path;

use crate::traits::ArchiveCreator;
use crate::types::ArchiveFormat;

/// 根据目标路径扩展名返回对应创建器。
/// 支持 `.zip` / `.7z` / `.tar.gz` / `.tgz` / `.tar.xz` / `.txz` / `.tar`。
/// 其他扩展名返回错误。
pub fn creator_for_path(dest: &Path) -> anyhow::Result<Box<dyn ArchiveCreator>> {
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".zip") {
        return Ok(Box::new(zip::ZipCreator));
    }
    if name.ends_with(".7z") {
        return Ok(Box::new(sevenz::SevenZCreator));
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Ok(Box::new(tar::TarCreator::gzip()));
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Ok(Box::new(tar::TarCreator::xz()));
    }
    if name.ends_with(".tar") {
        return Ok(Box::new(tar::TarCreator::plain()));
    }
    anyhow::bail!(
        "暂不支持的目标格式（支持 zip/7z/tar.gz/tar.xz/tar）：{}",
        dest.display()
    )
}

/// 用于错误信息辅助。
pub fn supported_formats() -> &'static [ArchiveFormat] {
    &[
        ArchiveFormat::Zip,
        ArchiveFormat::SevenZ,
        ArchiveFormat::TarGz,
        ArchiveFormat::TarXz,
        ArchiveFormat::Tar,
    ]
}
