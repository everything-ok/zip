//! 格式探测：magic bytes 优先，扩展名兜底。

use std::io::Read;
use std::path::Path;

use crate::error::ArchiveError;
use crate::types::ArchiveFormat;

/// 探测归档格式。
///
/// 顺序：magic bytes（对付伪装扩展名） -> TAR 的 `ustar`（偏移 257）-> 扩展名兜底。
pub fn detect_format(path: &Path) -> anyhow::Result<ArchiveFormat> {
    let mut f = std::fs::File::open(path)
        .map_err(|e| anyhow::anyhow!("打开文件失败 {}: {e}", path.display()))?;
    let mut head = [0u8; 512];
    let n = f.read(&mut head)?;
    let h = &head[..n];

    // ---- magic bytes ----
    if h.len() >= 4 {
        let p = &h[0..4];
        if p == b"PK\x03\x04" || p == b"PK\x05\x06" || p == b"PK\x07\x08" {
            return Ok(ArchiveFormat::Zip);
        }
    }
    if h.len() >= 6 && &h[0..6] == b"7z\xBC\xAF\x27\x1C" {
        return Ok(ArchiveFormat::SevenZ);
    }
    if h.len() >= 7 {
        // RAR5: 52 61 72 21 1A 07 01 00 ; RAR4: 52 61 72 21 1A 07 00
        if &h[0..7] == b"Rar!\x1A\x07\x01" || &h[0..7] == b"Rar!\x1A\x07\x00" {
            return Ok(ArchiveFormat::Rar);
        }
    }
    if h.len() >= 2 && &h[0..2] == b"\x1F\x8B" {
        return Ok(pick_tar_or_single(path, Single::Gzip));
    }
    if h.len() >= 3 && &h[0..3] == b"BZh" {
        return Ok(pick_tar_or_single(path, Single::Bzip2));
    }
    if h.len() >= 6 && &h[0..6] == b"\xFD7zXZ\x00" {
        return Ok(pick_tar_or_single(path, Single::Xz));
    }
    if h.len() >= 4 && &h[0..4] == b"\x28\xB5\x2F\xFD" {
        return Ok(pick_tar_or_single(path, Single::Zstd));
    }
    // ---- TAR：ustar 在偏移 257 ----
    if h.len() >= 263 && &h[257..262] == b"ustar" {
        return Ok(ArchiveFormat::Tar);
    }

    // ---- 扩展名兜底 ----
    Ok(match ext_of(path).as_deref() {
        Some("zip") => ArchiveFormat::Zip,
        Some("7z") => ArchiveFormat::SevenZ,
        Some("rar") => ArchiveFormat::Rar,
        Some("tar") => ArchiveFormat::Tar,
        Some("gz" | "gzip") => ArchiveFormat::Gzip,
        Some("bz2") => ArchiveFormat::Bzip2,
        Some("xz") => ArchiveFormat::Xz,
        Some("zst" | "zstd") => ArchiveFormat::Zstd,
        Some("tgz") => ArchiveFormat::TarGz,
        Some("tbz2" | "tbz") => ArchiveFormat::TarBz2,
        Some("txz") => ArchiveFormat::TarXz,
        Some("tzst" | "tzs") => ArchiveFormat::TarZst,
        Some(_) => {
            // 试试复合扩展名
            match compound_ext(path).as_deref() {
                Some("tar.gz") => ArchiveFormat::TarGz,
                Some("tar.bz2") => ArchiveFormat::TarBz2,
                Some("tar.xz") => ArchiveFormat::TarXz,
                Some("tar.zst") => ArchiveFormat::TarZst,
                _ => return Err(ArchiveError::UnknownFormat(path.display().to_string()).into()),
            }
        }
        None => return Err(ArchiveError::UnknownFormat(path.display().to_string()).into()),
    })
}

#[derive(Clone, Copy)]
enum Single {
    Gzip,
    Bzip2,
    Xz,
    Zstd,
}

/// 对单文件压缩流，根据扩展名判断它包裹的是否是 tar（.tar.gz -> TarGz）。
fn pick_tar_or_single(path: &Path, s: Single) -> ArchiveFormat {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let is_tar = name.ends_with(".tar.gz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tar.xz")
        || name.ends_with(".tar.zst")
        || name.ends_with(".tgz")
        || name.ends_with(".tbz2")
        || name.ends_with(".tbz")
        || name.ends_with(".txz")
        || name.ends_with(".tzst")
        || name.ends_with(".tzs");
    match (s, is_tar) {
        (Single::Gzip, true) => ArchiveFormat::TarGz,
        (Single::Gzip, false) => ArchiveFormat::Gzip,
        (Single::Bzip2, true) => ArchiveFormat::TarBz2,
        (Single::Bzip2, false) => ArchiveFormat::Bzip2,
        (Single::Xz, true) => ArchiveFormat::TarXz,
        (Single::Xz, false) => ArchiveFormat::Xz,
        (Single::Zstd, true) => ArchiveFormat::TarZst,
        (Single::Zstd, false) => ArchiveFormat::Zstd,
    }
}

fn ext_of(path: &Path) -> Option<String> {
    path.extension()?.to_str().map(|s| s.to_ascii_lowercase())
}

fn compound_ext(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let pats = ["tar.gz", "tar.bz2", "tar.xz", "tar.zst"];
    for p in pats {
        if name.ends_with(&format!(".{p}")) {
            return Some(p.to_string());
        }
    }
    None
}
