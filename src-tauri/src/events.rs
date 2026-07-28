//! 前后端通信的 DTO 与进度事件。

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ListRequest {
    pub path: String,
    pub password: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct EntryDto {
    pub path: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_dir: bool,
    pub is_encrypted: bool,
    pub modified: Option<u64>,
}

#[derive(Deserialize)]
pub struct ExtractRequest {
    pub task_id: String,
    pub source: String,
    pub dest: String,
    pub password: Option<String>,
    /// "skip" | "overwrite" | "rename" | "error"
    pub overwrite: String,
    /// 部分解压：只解压这些归档内路径。空或 None 表示解压全部。
    #[serde(default)]
    pub entries: Option<Vec<String>>,
}

/// 流式进度事件（走 `Channel<T>`）。
#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressEvent {
    Started {
        total_entries: usize,
        total_bytes: u64,
    },
    EntryStart {
        index: usize,
        total: usize,
        path: String,
        size: u64,
    },
    EntryDone {
        index: usize,
    },
    Bytes {
        processed: u64,
        total: u64,
        indeterminate: bool,
        speed: u64,
        eta_secs: Option<u64>,
    },
    Finished {
        summary: SummaryDto,
    },
    Cancelled,
    Error {
        error: ArchiveErrorDto,
    },
}

/// 稳定、可序列化的错误 DTO，供前端区分错误类型。
/// 仅 `password_required` / `wrong_password` 应触发密码输入流程。
#[derive(Serialize, Clone)]
pub struct ArchiveErrorDto {
    pub code: String,
    pub message: String,
}

impl ArchiveErrorDto {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }

    /// 从 archive-core 的 `anyhow::Error` 推断错误码。
    /// 优先匹配 `ArchiveError` 的具体变体；否则按字符串特征兜底。
    pub fn from_anyhow(error: &anyhow::Error) -> Self {
        // 优先用已 downcast 的具体 ArchiveError。
        if let Some(archive_error) = error.downcast_ref::<archive_core::ArchiveError>() {
            let message = archive_error.to_string();
            let code = match archive_error {
                archive_core::ArchiveError::PasswordRequired => "password_required",
                archive_core::ArchiveError::WrongPassword => "wrong_password",
                archive_core::ArchiveError::Corrupt(_) => "corrupt",
                archive_core::ArchiveError::Unsupported(_) => "unsupported",
                archive_core::ArchiveError::UnknownFormat(_) => "unsupported",
                archive_core::ArchiveError::PathTraversal(_) => "path_traversal",
                archive_core::ArchiveError::PathTooLong { .. } => "path_too_long",
                archive_core::ArchiveError::BombDetected { .. } => "bomb_detected",
                archive_core::ArchiveError::FileTooLarge { .. } => "file_too_large",
                archive_core::ArchiveError::TooManyEntries { .. } => "too_many_entries",
                archive_core::ArchiveError::Cancelled => "cancelled",
                archive_core::ArchiveError::Io(_) => "io",
            };
            return Self::new(code, message);
        }
        // 兜底：按错误字符串特征推断，避免密码相关错误被误判为损坏。
        let message = error.to_string();
        let lower = message.to_ascii_lowercase();
        let code = if lower.contains("取消") || lower.contains("cancel") {
            "cancelled"
        } else if lower.contains("password") || lower.contains("encrypt") || lower.contains("密码")
        {
            if lower.contains("wrong") || lower.contains("错误") || lower.contains("invalid") {
                "wrong_password"
            } else {
                "password_required"
            }
        } else if lower.contains("已存在") || lower.contains("conflict") || lower.contains("exist")
        {
            "conflict"
        } else if lower.contains("format") || lower.contains("格式") || lower.contains("unsupport")
        {
            "unsupported"
        } else if lower.contains("损坏") || lower.contains("corrupt") {
            "corrupt"
        } else {
            "io"
        };
        Self::new(code, message)
    }
}

#[derive(Serialize, Clone)]
pub struct SummaryDto {
    pub entries_extracted: usize,
    pub entries_skipped: usize,
    pub bytes_written: u64,
    pub cancelled: bool,
}
