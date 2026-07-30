import { invoke, Channel } from "@tauri-apps/api/core";
import { revealItemInDir, openUrl } from "@tauri-apps/plugin-opener";
import type {
  ArchiveErrorDto,
  CreateRequest,
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

/** 创建归档（压缩）。支持 zip/7z/tar.gz/tar.xz/tar。 */
export async function createArchive(
  req: CreateRequest,
  onProgress: (e: ProgressEvent) => void
): Promise<SummaryDto> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return invoke<SummaryDto>("create_archive", { req, onProgress: channel });
}

/** 在文件管理器中显示指定路径（所在目录并选中）。 */
export async function revealInDir(path: string): Promise<void> {
  return revealItemInDir(path);
}

/** 用系统默认浏览器打开 URL。用于反馈/更新等外链。 */
export async function openUrlNative(url: string): Promise<void> {
  return openUrl(url);
}

/// 更新检查信息。
export interface UpdateInfo {
  version: string;
  url: string;
}

/// 后端缓存的首启动作（文件关联/右键启动时前端 ready 前可能丢 emit）。
export interface PendingOpenDto {
  action: "open" | "extractHere" | "extractToSubdir";
  path: string;
}

/** 前端 ready 后取回缓存的首启动作。 */
export async function popPendingOpen(): Promise<PendingOpenDto | null> {
  try {
    return await invoke<PendingOpenDto | null>("pop_pending_open");
  } catch {
    return null;
  }
}

/** 打开系统"默认应用"设置页（引导用户设 Extractr 为默认）。 */
export async function openDefaultAppsSettings(): Promise<void> {
  try {
    await invoke("open_default_apps_settings");
  } catch {
    /* ignore */
  }
}

/** 检查 GitHub 是否有新版本。 */
export async function checkUpdate(): Promise<UpdateInfo | null> {
  try {
    return await invoke<UpdateInfo | null>("check_update");
  } catch {
    return null;
  }
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

