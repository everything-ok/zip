//! 格式探测：magic bytes 优先，扩展名兜底。支持分卷重定向。

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::ArchiveError;
use crate::types::ArchiveFormat;

/// 探测归档格式。
///
/// 顺序：magic bytes（对付伪装扩展名） -> TAR 的 `ustar`（偏移 257）-> 扩展名兜底。
/// 若检测到分卷非首卷，自动重定向到首卷路径。
pub fn detect_format(path: &Path) -> anyhow::Result<ArchiveFormat> {
    // 分卷重定向：若传入非首卷，返回首卷路径供调用方替换。
    // 首卷重定向在 detect 前做，避免打开非首卷读 magic。
    if let Some(first) = redirect_to_first_volume(path) {
        // 非首卷 → 用首卷路径继续探测，但调用方需替换 source。
        // 此处只探测格式，路径替换由 dispatcher 层负责。
        return detect_format_inner(&first);
    }
    detect_format_inner(path)
}

/// 返回首卷路径（若当前文件是分卷的非首卷）。None 表示非分卷或已是首卷。
pub fn first_volume_path(path: &Path) -> Option<PathBuf> {
    redirect_to_first_volume(path)
}

fn detect_format_inner(path: &Path) -> anyhow::Result<ArchiveFormat> {
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

/// 分卷重定向：检测文件名是否为分卷的非首卷，返回首卷路径。
///
/// 支持的分卷命名：
/// - RAR5: `xxx.part02.rar` → `xxx.part01.rar`
/// - RAR4: `xxx.r01` → `xxx.rar`
/// - 7z:   `xxx.7z.002` → `xxx.7z.001`
/// - ZIP:  `xxx.zip.002` → `xxx.zip.001`
/// - ZIP:  `xxx.z02` → `xxx.zip`
fn redirect_to_first_volume(path: &Path) -> Option<PathBuf> {
    let orig_name = path.file_name()?.to_str()?;
    let lower = orig_name.to_ascii_lowercase();
    let dir = path.parent()?;

    // RAR5: xxx.partNN.rar (NN > 1) → xxx.part01.rar
    if lower.ends_with(".rar") {
        if let Some(idx) = lower.find(".part") {
            let part_and_ext = &lower[idx..]; // ".part02.rar"
            let num_part = &part_and_ext[5..part_and_ext.len() - 4]; // "02"
            if let Ok(n) = num_part.parse::<u32>() {
                if n > 1 {
                    let width = num_part.len();
                    let first_num = format!("{:0width$}", 1, width = width);
                    let first_name = format!("{}.part{}.rar", &orig_name[..idx], first_num);
                    let candidate = dir.join(&first_name);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    // RAR4: xxx.rNN (NN > 0) → xxx.rar
    if lower.len() > 4 {
        let ext = &lower[lower.len() - 4..];
        if ext.starts_with(".r") && ext[2..].chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = ext[2..].parse::<u32>() {
                if n > 0 {
                    let stem = &orig_name[..orig_name.len() - 4];
                    let candidate = dir.join(format!("{}.rar", stem));
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    // 7z / ZIP 数字分卷: xxx.7z.NNN / xxx.zip.NNN (NNN > 1) → xxx.7z.001 / xxx.zip.001
    for base in [".7z.", ".zip."] {
        if let Some(idx) = lower.find(base) {
            let after_base = &lower[idx + base.len()..];
            if let Ok(n) = after_base.parse::<u32>() {
                if n > 1 {
                    let width = after_base.len();
                    let first_num = format!("{:0width$}", 1, width = width);
                    let first_name = format!("{}{}{}", &orig_name[..idx + base.len()], first_num, "");
                    let candidate = dir.join(&first_name);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    // ZIP 传统分卷: xxx.zNN (NN > 1) → xxx.zip
    if lower.len() > 4 {
        let ext = &lower[lower.len() - 4..];
        if ext.starts_with(".z") && ext[2..].chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = ext[2..].parse::<u32>() {
                if n > 1 {
                    let stem = &orig_name[..orig_name.len() - 4];
                    let candidate = dir.join(format!("{}.zip", stem));
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    None
}
