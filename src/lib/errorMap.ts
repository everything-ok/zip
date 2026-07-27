import type { ArchiveErrorDto } from "./types";

/**
 * 错误码到用户友好文案的映射。前端展示错误时统一走此映射，
 * 避免直接把后端原始 message 暴露给用户。
 *
 * `t` 来自 react-i18next；调用方传入以支持语言切换。
 */
export function describeError(
  error: Pick<ArchiveErrorDto, "code" | "message">,
  t: (key: string, opts?: Record<string, unknown>) => string
): string {
  const fallback = error.message;
  switch (error.code) {
    case "password_required":
      return t("err.passwordRequired");
    case "wrong_password":
      return t("err.wrongPassword");
    case "corrupt":
      return t("err.corrupt");
    case "unsupported":
      return t("err.unsupported");
    case "path_traversal":
      return t("err.pathTraversal");
    case "path_too_long":
      return t("err.pathTooLong");
    case "bomb_detected":
      return t("err.bombDetected");
    case "file_too_large":
      return t("err.fileTooLarge");
    case "too_many_entries":
      return t("err.tooManyEntries");
    case "too_many_tasks":
      return t("err.tooManyTasks");
    case "duplicate_task":
      return t("err.duplicateTask");
    case "conflict":
      return t("err.conflict");
    case "cancelled":
      return t("err.cancelled");
    case "io":
      return t("err.io");
    default:
      return fallback;
  }
}
