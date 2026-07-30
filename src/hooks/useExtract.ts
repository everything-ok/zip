import { useCallback } from "react";
import {
  extractArchive,
  cancelExtraction,
  detectFormat,
  revealInDir,
  toArchiveError,
} from "../lib/ipc";
import { useAppStore } from "../store/useAppStore";
import type { OverwritePolicy, ProgressEvent } from "../lib/types";

export function useExtract() {
  const addTask = useAppStore((s) => s.addTask);
  const updateTask = useAppStore((s) => s.updateTask);
  const tasks = useAppStore((s) => s.tasks);
  const autoOpenDir = useAppStore((s) => s.settings.autoOpenDir);

  /**
   * 内部执行器：用给定 ID 重跑解压（供首次与重试共用）。
   * 重试时已有任务记录，只复位状态再执行。
   */
  const execute = useCallback(
    async (
      id: string,
      source: string,
      dest: string,
      password: string | null,
      overwrite: OverwritePolicy,
      isRetry: boolean
    ) => {
      let format = "unknown";
      try {
        format = await detectFormat(source);
      } catch {
        /* ignore */
      }
      if (isRetry) {
        updateTask(id, {
          status: "running",
          progress: 0,
          indeterminate: false,
          currentFile: undefined,
          error: undefined,
          error_code: undefined,
          summary: undefined,
          format,
        });
      } else {
        addTask({
          id,
          source,
          dest,
          format,
          progress: 0,
          indeterminate: false,
          status: "running",
          encrypted: password != null,
          password,
          overwrite,
        });
      }

      const onProgress = (e: ProgressEvent) => {
        switch (e.kind) {
          case "entry_start":
            updateTask(id, { currentFile: e.path });
            break;
          case "bytes":
            updateTask(id, {
              progress: e.indeterminate ? 0 : e.total ? e.processed / e.total : 0,
              indeterminate: e.indeterminate || e.total === 0,
              speed: e.speed,
              eta_secs: e.eta_secs,
            });
            break;
          case "finished":
            // 先把进度条动画走到 100%，延迟打开目录，避免与完成态切换抖动叠加。
            updateTask(id, {
              progress: 1,
              indeterminate: false,
              summary: e.summary,
            });
            // 让进度条补间动画完成后再切完成态 + 开目录，过渡更自然。
            window.setTimeout(() => {
              updateTask(id, { status: "done" });
              if (autoOpenDir) {
                void revealInDir(dest).catch(() => {});
              }
            }, 450);
            break;
          case "cancelled":
            updateTask(id, { status: "cancelled" });
            break;
          case "error":
            updateTask(id, {
              status: "error",
              error: e.error.message,
              error_code: e.error.code,
            });
            break;
        }
      };

      try {
        await extractArchive(
          { task_id: id, source, dest, password, overwrite },
          onProgress
        );
      } catch (err) {
        const dto = toArchiveError(err);
        if (dto.code === "cancelled") {
          updateTask(id, { status: "cancelled" });
        } else {
          updateTask(id, {
            status: "error",
            error: dto.message,
            error_code: dto.code,
          });
        }
      }
    },
    [addTask, updateTask, autoOpenDir]
  );

  const run = useCallback(
    async (
      source: string,
      dest: string,
      password: string | null,
      overwrite: OverwritePolicy | string
    ) => {
      const id =
        typeof crypto !== "undefined" && crypto.randomUUID
          ? crypto.randomUUID()
          : `t-${Date.now()}-${Math.random().toString(36).slice(2)}`;
      const ow = (overwrite as OverwritePolicy) ?? "skip";
      void execute(id, source, dest, password, ow, false);
    },
    [execute]
  );

  /** 重试失败/取消的任务，复用原 source/dest/password/overwrite，新 task_id。 */
  const retry = useCallback(
    (id: string) => {
      const task = tasks.find((t) => t.id === id);
      if (!task) return;
      void execute(
        id,
        task.source,
        task.dest,
        task.password ?? null,
        task.overwrite ?? "skip",
        true
      );
    },
    [execute, tasks]
  );

  const cancel = useCallback((id: string) => {
    void cancelExtraction(id);
  }, []);

  return { run, cancel, retry };
}
