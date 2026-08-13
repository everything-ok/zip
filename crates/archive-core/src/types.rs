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
    /// 解压资源上限（防解压炸弹与资源耗尽）。None 表示用默认值。
    #[serde(default)]
    pub limits: ExtractLimits,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            password: None,
            overwrite: OverwritePolicy::default(),
            keep_dir_mtime: true,
            skip_symlinks: true,
            limits: ExtractLimits::default(),
        }
    }
}

/// 解压安全上限。默认值保守：足以覆盖正常大归档，同时拦截恶意炸弹。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractLimits {
    /// 解压后总字节上限（默认 4 GiB）。
    pub max_total_bytes: u64,
    /// 单个文件未压缩字节上限（默认 1 GiB）。
    pub max_file_bytes: u64,
    /// 压缩比上限（未压缩 / 压缩，默认 200），用于流式格式无法预知总量时。
    pub max_ratio: u64,
    /// 归档条目数上限（默认 100000）。
    pub max_entries: usize,
    /// 单条目路径总长度上限（默认 4096，兼容长路径但防滥用）。
    pub max_path_len: usize,
}

impl Default for ExtractLimits {
    fn default() -> Self {
        Self {
            max_total_bytes: 4 * 1024 * 1024 * 1024,
            max_file_bytes: 1024 * 1024 * 1024,
            max_ratio: 200,
            max_entries: 100_000,
            max_path_len: 4096,
        }
    }
}

impl ExtractLimits {
    /// 校验条目数是否超限。
    pub fn check_entries(&self, count: usize) -> Result<(), crate::error::ArchiveError> {
        if count > self.max_entries {
            Err(crate::error::ArchiveError::TooManyEntries {
                actual: count,
                max: self.max_entries,
            })
        } else {
            Ok(())
        }
    }

    /// 校验单文件大小是否超限。
    pub fn check_file_size(&self, size: u64) -> Result<(), crate::error::ArchiveError> {
        if size > self.max_file_bytes {
            Err(crate::error::ArchiveError::FileTooLarge {
                actual: size,
                max: self.max_file_bytes,
            })
        } else {
            Ok(())
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
    /// 已成功提交到 dest 的文件路径（相对 dest），取消时用于清理残留。
    #[serde(default)]
    pub extracted_paths: Vec<String>,
}
