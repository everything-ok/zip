import { useMemo } from "react";
import { File, Folder, Lock } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { EntryDto } from "../lib/types";
import { formatBytes, basename } from "../lib/format";
import { Badge } from "./ui";

export function ArchivePreview({
  path,
  format,
  entries,
}: {
  path: string;
  format?: string;
  entries: EntryDto[];
}) {
  const { t } = useTranslation();
  const total = useMemo(
    () => entries.reduce((s, e) => s + (e.is_dir ? 0 : e.size), 0),
    [entries]
  );
  const encrypted = entries.some((e) => e.is_encrypted);

  return (
    <div className="flex flex-col overflow-hidden rounded-xl border border-zinc-200 bg-white dark:border-zinc-800 dark:bg-zinc-900">
      <div className="flex items-center gap-2 border-b border-zinc-200 px-4 py-3 dark:border-zinc-800">
        <span className="truncate text-sm font-medium text-zinc-900 dark:text-zinc-100">
          {basename(path)}
        </span>
        {format && <Badge color="indigo">{format.toUpperCase()}</Badge>}
        {encrypted && (
          <Badge color="amber">
            <Lock size={11} className="mr-1" />
            {t("preview.encrypted")}
          </Badge>
        )}
        <span className="ml-auto text-xs text-zinc-500">
          {t("preview.items", { count: entries.length })}
        </span>
      </div>
      <div className="max-h-64 overflow-auto">
        {entries.length === 0 ? (
          <div className="px-4 py-10 text-center text-sm text-zinc-400">
            {t("preview.empty")}
          </div>
        ) : (
          <table className="w-full text-sm">
            <tbody>
              {entries.slice(0, 500).map((e, i) => (
                <tr
                  key={i}
                  className="border-b border-zinc-100 last:border-0 dark:border-zinc-800"
                >
                  <td className="px-4 py-1.5">
                    <span className="flex items-center gap-2 truncate">
                      {e.is_dir ? (
                        <Folder size={14} className="shrink-0 text-indigo-400" />
                      ) : (
                        <File size={14} className="shrink-0 text-zinc-400" />
                      )}
                      <span className="truncate text-zinc-700 dark:text-zinc-300">
                        {e.path}
                      </span>
                      {e.is_encrypted && <Lock size={11} className="text-amber-500" />}
                    </span>
                  </td>
                  <td className="whitespace-nowrap px-4 py-1.5 text-right font-mono text-xs text-zinc-500">
                    {e.is_dir ? "-" : formatBytes(e.size)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
      <div className="border-t border-zinc-200 px-4 py-2 text-xs text-zinc-500 dark:border-zinc-800">
        {t("preview.total")}: {formatBytes(total)}
      </div>
    </div>
  );
}
