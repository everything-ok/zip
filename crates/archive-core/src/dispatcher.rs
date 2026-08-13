//! 格式分发：根据 `ArchiveFormat` 返回对应 extractor。

use std::path::{Path, PathBuf};

use crate::detect::{detect_format, first_volume_path};
use crate::extractors;
use crate::traits::ArchiveExtractor;
use crate::types::ArchiveFormat;

/// 推荐入口：自动探测格式并返回解压器。
/// 若传入分卷非首卷，自动重定向到首卷。
pub fn open(path: &Path) -> anyhow::Result<Box<dyn ArchiveExtractor>> {
    let fmt = detect_format(path)?;
    Ok(dispatcher_for_format(fmt))
}

/// 返回分卷首卷路径（若当前文件是分卷非首卷），否则返回 None。
pub fn resolve_first_volume(path: &Path) -> Option<PathBuf> {
    first_volume_path(path)
}

/// 根据已知的格式返回对应 extractor。
pub fn dispatcher_for_format(fmt: ArchiveFormat) -> Box<dyn ArchiveExtractor> {
    use ArchiveFormat::*;
    match fmt {
        Zip => Box::new(extractors::zip::ZipExtractor),
        SevenZ => Box::new(extractors::sevenz::SevenZExtractor),
        Tar => Box::new(extractors::tar::TarExtractor::plain()),
        Gzip => Box::new(extractors::gz::GzExtractor::Gzip),
        Bzip2 => Box::new(extractors::gz::GzExtractor::Bzip2),
        Xz => Box::new(extractors::gz::GzExtractor::Xz),
        Zstd => Box::new(extractors::gz::GzExtractor::Zstd),
        TarGz => Box::new(extractors::tar::TarExtractor::gzip()),
        TarBz2 => Box::new(extractors::tar::TarExtractor::bzip2()),
        TarXz => Box::new(extractors::tar::TarExtractor::xz()),
        TarZst => Box::new(extractors::tar::TarExtractor::zstd()),
        Rar => {
            #[cfg(feature = "rar")]
            {
                Box::new(extractors::rar::RarExtractor)
            }
            #[cfg(not(feature = "rar"))]
            {
                let _ = fmt;
                Box::new(extractors::unsupported::UnsupportedExtractor(
                    ArchiveFormat::Rar,
                ))
            }
        }
    }
}
