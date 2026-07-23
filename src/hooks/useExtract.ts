import { useCallback } from "react";
import { extractArchive, cancelExtraction, detectFormat, toArchiveError } from "../lib/ipc";
import { useAppStore } from "../store/useAppStore";
import type { ArchiveErrorDto, ProgressEvent } from "../lib/types";

export function useExtract() {
  const addTask = useAppStore((s) => s.addTask);
  const updateTask = useAppStore((s) => s.updateTask);

  const run = useCallback(
    async (
      source: string,
      dest: string,
      password: string | null,
      overwrite: string
    ) => {
      const id =
        typeof crypto !== "undefined" && crypto.randomUUID
          ? crypto.randomUUID()
          : `t-${Date.now()}-${Math.random().toString(36).slice(2)}`;

      let format = "unknown";
      try {
        format = await detectFormat(source);
      } catch {
        /* ignore */
      }
      addTask({
        id,
        source,
        dest,
        format,
        progress: 0,
        indeterminate: false,
        status: "running",
        encrypted: password != null,
      });

      const onProgress = (e: ProgressEvent) => {
        switch (e.kind) {
          case "entry_start":
            updateTask(id, { currentFile: e.path });
            break;
          case "bytes":
            updateTask(id, {
              progress: e.indeterminate ? 0 : e.total ? e.processed / e.total : 0,
              indeterminate: e.indeterminate || e.total === 0,
            });
            break;
          case "finished":
            updateTask(id, {
              status: "done",
              progress: 1,
              indeterminate: false,
              summary: e.summary,
            });
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
          { task_id: id, source, dest, password, overwrite: overwrite as never },
          onProgress
        );
      } catch (err) {
        const dto: ArchiveErrorDto = toArchiveError(err);
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
    [addTask, updateTask]
  );

  const cancel = useCallback((id: string) => {
    void cancelExtraction(id);
  }, []);

  return { run, cancel };
}
