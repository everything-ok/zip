//! 归档条目、解压选项、摘要等核心数据类型。

use serde::{Deserialize, Serialize};

/// 支持的归档格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveFormat {
    Zip,
    SevenZ,
    Rar,
    Tar,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    TarGz,
    TarBz2,
    TarXz,
    TarZst,
}

impl ArchiveFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::SevenZ => "7z",
            ArchiveFormat::Rar => "rar",
            ArchiveFormat::Tar => "tar",
            ArchiveFormat::Gzip => "gzip",
            ArchiveFormat::Bzip2 => "bzip2",
            ArchiveFormat::Xz => "xz",
            ArchiveFormat::Zstd => "zstd",
            ArchiveFormat::TarGz => "tar.gz",
            ArchiveFormat::TarBz2 => "tar.bz2",
            ArchiveFormat::TarXz => "tar.xz",
            ArchiveFormat::TarZst => "tar.zst",
        }
    }
}

impl std::fmt::Display for ArchiveFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 归档内单个条目的元信息（原样路径，未净化）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchiveEntry {
    /// 归档内相对路径（原样，未净化）。
    pub path: String,
    /// 解压后字节数（未知为 0）。
    pub size: u64,
    /// 压缩后字节数（未知为 0）。
    pub compressed_size: u64,
    /// 是否目录。
    pub is_dir: bool,
    /// 是否加密。
    pub is_encrypted: bool,
    /// 修改时间（unix 秒），未知为 None。
    pub modified: Option<u64>,
}

/// 已存在目标文件的覆盖策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OverwritePolicy {
    /// 跳过（默认）。
    #[default]
    Skip,
    /// 覆盖。
    Overwrite,
    /// 重命名（追加序号）。
    Rename,
    /// 报错。
    Error,
}

/// 一次解压调用的选项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOptions {
    /// 密码（用于加密归档）。
    pub password: Option<String>,
    /// 覆盖策略。
    pub overwrite: OverwritePolicy,
    /// 保留目录修改时间。
    pub keep_dir_mtime: bool,
    /// 是否跳过符号链接（默认 true，防逃逸）。
    pub skip_symlinks: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            password: None,
            overwrite: OverwritePolicy::default(),
            keep_dir_mtime: true,
            skip_symlinks: true,
        }
    }
}

/// 一次解压完成后的统计摘要。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractSummary {
    pub entries_total: usize,
    pub entries_extracted: usize,
    pub entries_skipped: usize,
    pub bytes_written: u64,
    pub cancelled: bool,
}
