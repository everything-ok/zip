import { invoke, Channel } from "@tauri-apps/api/core";
import { revealItemInDir, openPath } from "@tauri-apps/plugin-opener";
import type {
  ArchiveErrorDto,
  EntryDto,
  ExtractRequest,
  ListRequest,
  ProgressEvent,
  SummaryDto,
} from "./types";

export async function detectFormat(path: string): Promise<string> {
  return invoke<string>("detect_format", { path });
}

export async function listArchive(req: ListRequest): Promise<EntryDto[]> {
  return invoke<EntryDto[]>("list_archive", { req });
}

export async function extractArchive(
  req: ExtractRequest,
  onProgress: (e: ProgressEvent) => void
): Promise<SummaryDto> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return invoke<SummaryDto>("extract_archive", { req, onProgress: channel });
}

export async function cancelExtraction(taskId: string): Promise<void> {
  return invoke("cancel_extraction", { taskId });
}

/** 在文件管理器中显示指定路径（所在目录并选中）。 */
export async function revealInDir(path: string): Promise<void> {
  return revealItemInDir(path);
}

/** 用系统默认方式打开路径（目录或文件）。 */
export async function openPathNative(path: string): Promise<void> {
  return openPath(path);
}

/**
 * Tauri 命令 reject 时，错误体可能是结构化 DTO（已序列化为 JS 对象），
 * 也可能是裸字符串。统一规整为 `ArchiveErrorDto`。
 */
export function toArchiveError(error: unknown): ArchiveErrorDto {
  if (error && typeof error === "object" && "code" in error && "message" in error) {
    const code = String((error as { code: unknown }).code);
    const message = String((error as { message: unknown }).message);
    return { code: code as ArchiveErrorDto["code"], message };
  }
  const message = String(error);
  return { code: "io", message };
}

