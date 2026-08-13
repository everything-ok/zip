import { useCallback } from "react";
import {
  extractArchive,
  createArchive,
  convertArchive,
  testArchive,
  cancelExtraction,
  detectFormat,
  revealInDir,
  toArchiveError,
  deleteSource,
} from "../lib/ipc";
import { useAppStore } from "../store/useAppStore";
import type { CreateSourceDto, OverwritePolicy, ProgressEvent, ConvertRequest } from "../lib/types";

export function useExtract() {
  const addTask = useAppStore((s) => s.addTask);
  const updateTask = useAppStore((s) => s.updateTask);
  const tasks = useAppStore((s) => s.tasks);
  const autoOpenDir = useAppStore((s) => s.settings.autoOpenDir);
  const deleteAfterExtract = useAppStore((s) => s.settings.deleteAfterExtract);
  const deleteAfterCompress = useAppStore((s) => s.settings.deleteAfterCompress);

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
              // 解压完成后按设置删除原文件
              if (deleteAfterExtract) {
                void deleteSource(source).catch(() => {});
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
    [addTask, updateTask, autoOpenDir, deleteAfterExtract]
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

  /** 压缩：把一组磁盘文件/目录打包成归档。 */
  const create = useCallback(
    async (
      dest: string,
      sources: CreateSourceDto[],
      password: string | null,
      level: number | null
    ) => {
      const id =
        typeof crypto !== "undefined" && crypto.randomUUID
          ? crypto.randomUUID()
          : `c-${Date.now()}-${Math.random().toString(36).slice(2)}`;
      addTask({
        id,
        source: dest,
        dest,
        format: dest.split(".").pop()?.toUpperCase() ?? "",
        progress: 0,
        indeterminate: false,
        status: "running",
        encrypted: password != null,
        password,
        overwrite: "skip",
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
              speed: e.speed,
              eta_secs: e.eta_secs,
            });
            break;
          case "finished":
            updateTask(id, { progress: 1, indeterminate: false, summary: e.summary });
            window.setTimeout(() => {
              updateTask(id, { status: "done" });
              if (autoOpenDir) {
                void revealInDir(dest).catch(() => {});
              }
              // 压缩完成后按设置删除原文件
              if (deleteAfterCompress) {
                for (const s of sources) {
                  void deleteSource(s.fs_path).catch(() => {});
                }
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
        await createArchive({ task_id: id, dest, sources, password, level }, onProgress);
      } catch (err) {
        const dto = toArchiveError(err);
        updateTask(id, {
          status: "error",
          error: dto.message,
          error_code: dto.code,
        });
      }
    },
    [addTask, updateTask, autoOpenDir, deleteAfterCompress]
  );

  /** 格式转换：解压源归档再用目标格式重新打包。 */
  const convert = useCallback(
    async (req: ConvertRequest) => {
      const id = req.task_id;
      addTask({
        id,
        source: req.source,
        dest: req.dest,
        format: req.dest.split(".").pop()?.toUpperCase() ?? "",
        progress: 0,
        indeterminate: false,
        status: "running",
        encrypted: req.password != null || req.dest_password != null,
        password: req.password,
        overwrite: "skip",
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
              speed: e.speed,
              eta_secs: e.eta_secs,
            });
            break;
          case "finished":
            updateTask(id, { progress: 1, indeterminate: false, summary: e.summary });
            window.setTimeout(() => {
              updateTask(id, { status: "done" });
              if (autoOpenDir) {
                void revealInDir(req.dest).catch(() => {});
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
        await convertArchive(req, onProgress);
      } catch (err) {
        const dto = toArchiveError(err);
        updateTask(id, {
          status: "error",
          error: dto.message,
          error_code: dto.code,
        });
      }
    },
    [addTask, updateTask, autoOpenDir, deleteAfterExtract]
  );

  /** 测试归档完整性（CRC 校验，不写盘）。 */
  const test = useCallback(
    async (source: string, password: string | null) => {
      const id =
        typeof crypto !== "undefined" && crypto.randomUUID
          ? crypto.randomUUID()
          : `t-${Date.now()}-${Math.random().toString(36).slice(2)}`;
      addTask({
        id,
        source,
        dest: "",
        format: "TEST",
        progress: 0,
        indeterminate: true,
        status: "running",
        encrypted: password != null,
        password,
        overwrite: "skip",
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
              speed: e.speed,
              eta_secs: e.eta_secs,
            });
            break;
          case "finished":
            updateTask(id, { progress: 1, indeterminate: false, summary: e.summary });
            window.setTimeout(() => {
              updateTask(id, { status: "done" });
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
        await testArchive({ source, password }, onProgress);
      } catch (err) {
        const dto = toArchiveError(err);
        updateTask(id, {
          status: "error",
          error: dto.message,
          error_code: dto.code,
        });
      }
    },
    [addTask, updateTask]
  );

  return { run, cancel, retry, create, convert, test };
}
