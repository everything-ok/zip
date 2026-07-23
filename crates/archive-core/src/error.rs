//! 错误类型。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("无法识别的归档格式: {0}")]
    UnknownFormat(String),

    #[error("路径逃逸（拒绝条目以防 Zip Slip）: {0}")]
    PathTraversal(String),

    #[error("归档已加密，需要密码")]
    PasswordRequired,

    #[error("密码错误")]
    WrongPassword,

    #[error("归档已损坏或不完整: {0}")]
    Corrupt(String),

    #[error("操作已取消")]
    Cancelled,

    #[error("不支持的特性: {0}")]
    Unsupported(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl ArchiveError {
    /// 是否由取消引起。
    pub fn is_cancelled(&self) -> bool {
        matches!(self, ArchiveError::Cancelled)
    }
}

impl From<ArchiveError> for std::io::Error {
    fn from(e: ArchiveError) -> Self {
        match e {
            ArchiveError::Io(io) => io,
            other => std::io::Error::other(other.to_string()),
        }
    }
}
