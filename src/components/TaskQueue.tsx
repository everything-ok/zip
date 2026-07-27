import {
  CheckCircle2,
  XCircle,
  Loader2,
  Ban,
  X,
  FolderOpen,
  RotateCcw,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../store/useAppStore";
import { useExtract } from "../hooks/useExtract";
import { revealInDir } from "../lib/ipc";
import { describeError } from "../lib/errorMap";
import { basename, formatBytes, percent } from "../lib/format";
import { Button } from "./ui";
import type { OverwritePolicy } from "../lib/types";

export function TaskQueue() {
  const { t } = useTranslation();
  const tasks = useAppStore((s) => s.tasks);
  const updateTask = useAppStore((s) => s.updateTask);
  const removeTask = useAppStore((s) => s.removeTask);
  const clearFinished = useAppStore((s) => s.clearFinished);
  const { cancel, retry } = useExtract();

  if (tasks.length === 0) return null;

  return (
    <div className="shrink-0 border-t border-zinc-200 dark:border-zinc-800">
      <div className="flex items-center justify-between px-5 py-2">
        <span className="text-xs font-medium uppercase tracking-wide text-zinc-500">
          {t("queue.title")} ({tasks.length})
        </span>
        <button
          onClick={clearFinished}
          className="text-xs text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
        >
          {t("common.clear")}
        </button>
      </div>
      <div className="max-h-48 overflow-auto px-5 pb-3">
        {tasks.map((task) => (
          <div
            key={task.id}
            className="mb-2 rounded-lg border border-zinc-200 p-3 dark:border-zinc-800"
          >
            <div className="flex items-center gap-2">
              <StatusIcon status={task.status} />
              <span className="min-w-0 truncate text-sm text-zinc-800 dark:text-zinc-200">
                {basename(task.source)}
              </span>
              <span className="ml-auto flex items-center gap-1">
                {task.status === "running" && (
                  <>
                    <select
                      value={task.overwrite ?? "skip"}
                      onChange={(e) =>
                        updateTask(task.id, {
                          overwrite: e.target.value as OverwritePolicy,
                        })
                      }
                      title={t("settings.overwrite")}
                      className="rounded border border-zinc-300 bg-white px-1 py-0.5 text-xs dark:border-zinc-700 dark:bg-zinc-900"
                    >
                      <option value="skip">{t("settings.ow_skip")}</option>
                      <option value="overwrite">{t("settings.ow_overwrite")}</option>
                      <option value="rename">{t("settings.ow_rename")}</option>
                      <option value="error">{t("settings.ow_error")}</option>
                    </select>
                    <Button
                      variant="ghost"
                      onClick={() => cancel(task.id)}
                      className="px-2 py-1 text-xs"
                    >
                      <Ban size={12} /> {t("common.cancel")}
                    </Button>
                  </>
                )}
                {(task.status === "done" ||
                  task.status === "error" ||
                  task.status === "cancelled") && (
                  <span className="flex items-center gap-1">
                    {task.status === "done" && task.dest && (
                      <button
                        onClick={() => void revealInDir(task.dest).catch(() => {})}
                        title={t("common.openDir")}
                        className="text-zinc-400 hover:text-indigo-500"
                      >
                        <FolderOpen size={14} />
                      </button>
                    )}
                    {(task.status === "error" || task.status === "cancelled") && (
                      <button
                        onClick={() => retry(task.id)}
                        title={t("common.retry")}
                        className="text-zinc-400 hover:text-emerald-500"
                      >
                        <RotateCcw size={14} />
                      </button>
                    )}
                    <button
                      onClick={() => removeTask(task.id)}
                      className="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
                    >
                      <X size={14} />
                    </button>
                  </span>
                )}
              </span>
            </div>
            {task.status === "running" && (
              <div className="mt-2">
                {task.indeterminate ? (
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-800">
                    <div className="h-full w-1/3 animate-pulse bg-indigo-400" />
                  </div>
                ) : (
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-800">
                    <div
                      className="h-full bg-indigo-500 transition-all"
                      style={{ width: percent(task.progress) }}
                    />
                  </div>
                )}
                <div className="mt-1 flex justify-between gap-2 text-xs text-zinc-500">
                  <span className="truncate">
                    {task.currentFile
                      ? `${t("progress.current")}: ${task.currentFile}`
                      : t("progress.extracting")}
                  </span>
                  <span className="shrink-0">
                    {task.indeterminate
                      ? t("progress.indeterminate")
                      : percent(task.progress)}
                  </span>
                </div>
              </div>
            )}
            {task.status === "done" && task.summary && (
              <div className="mt-1 text-xs text-emerald-600 dark:text-emerald-400">
                {t("progress.done")} · {task.summary.entries_extracted}{" "}
                {t("preview.items", { count: task.summary.entries_extracted }).replace(
                  /\d+\s*/,
                  ""
                )}{" "}
                · {formatBytes(task.summary.bytes_written)}
              </div>
            )}
            {task.status === "cancelled" && (
              <div className="mt-1 text-xs text-zinc-500">
                {t("progress.cancelled")}
              </div>
            )}
            {task.status === "error" && (
              <div
                className="mt-1 break-words text-xs text-red-500"
                title={task.error}
              >
                {t("progress.error")}:{" "}
                {task.error_code
                  ? describeError(
                      { code: task.error_code, message: task.error ?? "" },
                      t
                    )
                  : task.error}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function StatusIcon({ status }: { status: string }) {
  if (status === "running")
    return <Loader2 size={14} className="shrink-0 animate-spin text-indigo-500" />;
  if (status === "done")
    return <CheckCircle2 size={14} className="shrink-0 text-emerald-500" />;
  if (status === "error")
    return <XCircle size={14} className="shrink-0 text-red-500" />;
  if (status === "cancelled")
    return <Ban size={14} className="shrink-0 text-zinc-400" />;
  return null;
}
