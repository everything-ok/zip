// 与 Rust serde 对齐的前端类型。

export interface EntryDto {
  path: string;
  size: number;
  compressed_size: number;
  is_dir: boolean;
  is_encrypted: boolean;
  modified: number | null;
}

export interface SummaryDto {
  entries_extracted: number;
  entries_skipped: number;
  bytes_written: number;
  cancelled: boolean;
}

/// 与后端 `ArchiveErrorDto` 对齐的结构化错误。
/// 只有 password_required / wrong_password 应触发密码输入流程。
export interface ArchiveErrorDto {
  code:
    | "password_required"
    | "wrong_password"
    | "corrupt"
    | "unsupported"
    | "path_traversal"
    | "path_too_long"
    | "bomb_detected"
    | "file_too_large"
    | "too_many_entries"
    | "too_many_tasks"
    | "conflict"
    | "cancelled"
    | "io"
    | "duplicate_task";
  message: string;
}

export interface ExtractRequest {
  task_id: string;
  source: string;
  dest: string;
  password: string | null;
  overwrite: OverwritePolicy;
  entries?: string[] | null;
}

/** 压缩创建请求。 */
export interface CreateRequest {
  task_id: string;
  dest: string;
  sources: CreateSourceDto[];
  password: string | null;
  level: number | null;
}

/** 压缩源条目：磁盘文件路径 + 归档内路径。 */
export interface CreateSourceDto {
  fs_path: string;
  archive_path: string;
}

/** 压缩支持的格式。 */
export type CreateFormat =
  | "zip"
  | "7z"
  | "tar"
  | "tar.gz"
  | "tar.xz";

export interface ListRequest {
  path: string;
  password: string | null;
}

export type OverwritePolicy = "skip" | "overwrite" | "rename" | "error";

/// 文件关联/右键菜单启动动作。
export type OpenArchiveAction =
  | { action: "open"; path: string }
  | { action: "extractHere"; path: string }
  | { action: "extractToSubdir"; path: string };

export type ProgressEvent =
  | { kind: "started"; total_entries: number; total_bytes: number }
  | { kind: "entry_start"; index: number; total: number; path: string; size: number }
  | { kind: "entry_done"; index: number }
  | {
      kind: "bytes";
      processed: number;
      total: number;
      indeterminate: boolean;
      speed: number;
      eta_secs: number | null;
    }
  | { kind: "finished"; summary: SummaryDto }
  | { kind: "cancelled" }
  | { kind: "error"; error: ArchiveErrorDto };

/// 判断错误是否需要弹密码框。
export function isPasswordError(error: ArchiveErrorDto): boolean {
  return error.code === "password_required" || error.code === "wrong_password";
}

