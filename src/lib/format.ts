export function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB", "PB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
  return `${(bytes / Math.pow(k, i)).toFixed(i === 0 ? 0 : 1)} ${sizes[i]}`;
}

export function basename(p: string): string {
  const parts = p.replace(/\\/g, "/").split("/").filter(Boolean);
  return parts[parts.length - 1] ?? p;
}

export function percent(value: number): string {
  return `${Math.max(0, Math.min(100, Math.round(value * 100)))}%`;
}

/**
 * 计算压缩率：未压缩 / 压缩。返回百分比字符串，如 "35%"。
 * 压缩后大小未知（0）或为 0 时返回 "-"。
 */
export function compressionRatio(size: number, compressed: number): string {
  if (!compressed || compressed <= 0 || !size || size <= 0) return "-";
  const ratio = compressed / size;
  return `${Math.round(ratio * 100)}%`;
}

/** unix 秒转本地化日期时间字符串。 */
export function formatTime(unixSeconds: number | null): string {
  if (!unixSeconds || unixSeconds <= 0) return "-";
  const date = new Date(unixSeconds * 1000);
  if (Number.isNaN(date.getTime())) return "-";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours()
  )}:${pad(date.getMinutes())}`;
}

