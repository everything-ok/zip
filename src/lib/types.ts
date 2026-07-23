// 与 Rust serde 对齐的前端类型。

export interface EntryDto {
  path: string;
  size: number;
  is_dir: boolean;
  is_encrypted: boolean;
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
}

export interface ListRequest {
  path: string;
  password: string | null;
}

export type OverwritePolicy = "skip" | "overwrite" | "rename" | "error";

export type ProgressEvent =
  | { kind: "started"; total_entries: number; total_bytes: number }
  | { kind: "entry_start"; index: number; total: number; path: string; size: number }
  | { kind: "entry_done"; index: number }
  | {
      kind: "bytes";
      processed: number;
      total: number;
      indeterminate: boolean;
    }
  | { kind: "finished"; summary: SummaryDto }
  | { kind: "cancelled" }
  | { kind: "error"; error: ArchiveErrorDto };

/// 判断错误是否需要弹密码框。
export function isPasswordError(error: ArchiveErrorDto): boolean {
  return error.code === "password_required" || error.code === "wrong_password";
}

