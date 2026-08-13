import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ArchiveErrorDto, SummaryDto } from "../lib/types";

export type TaskStatus = "pending" | "running" | "done" | "error" | "cancelled";
export type OverwritePolicy = "skip" | "overwrite" | "rename" | "error";
export type ThemeMode = "light" | "dark" | "system";

export interface Task {
  id: string;
  source: string;
  dest: string;
  format?: string;
  progress: number;
  indeterminate: boolean;
  currentFile?: string;
  status: TaskStatus;
  summary?: SummaryDto;
  encrypted: boolean;
  error?: string;
  error_code?: ArchiveErrorDto["code"];
  password?: string | null;
  overwrite: OverwritePolicy;
  speed?: number;
  eta_secs?: number | null;
}

export interface Settings {
  theme: ThemeMode;
  language: "zh" | "en";
  overwrite: OverwritePolicy;
  autoOpenDir: boolean;
  deleteAfterExtract: boolean;
  deleteAfterCompress: boolean;
  recentDirs: string[];
}

interface AppState {
  tasks: Task[];
  settings: Settings;
  addTask: (t: Task) => void;
  updateTask: (id: string, patch: Partial<Task>) => void;
  removeTask: (id: string) => void;
  clearFinished: () => void;
  setSettings: (s: Partial<Settings>) => void;
  addRecentDir: (dir: string) => void;
}

const MAX_RECENT_DIRS = 10;

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      tasks: [],
      settings: {
        theme: "system",
        language: "zh",
        overwrite: "skip",
        autoOpenDir: false,
        deleteAfterExtract: false,
        deleteAfterCompress: false,
        recentDirs: [],
      },
      addTask: (t) => set((s) => ({ tasks: [t, ...s.tasks] })),
      updateTask: (id, patch) =>
        set((s) => ({
          tasks: s.tasks.map((t) => (t.id === id ? { ...t, ...patch } : t)),
        })),
      removeTask: (id) =>
        set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) })),
      clearFinished: () =>
        set((s) => ({
          tasks: s.tasks.filter(
            (t) => t.status === "running" || t.status === "pending"
          ),
        })),
      setSettings: (s) =>
        set((state) => ({ settings: { ...state.settings, ...s } })),
      addRecentDir: (dir) =>
        set((state) => {
          const trimmed = dir.trim();
          if (!trimmed) return {};
          const recent = [
            trimmed,
            ...state.settings.recentDirs.filter((d) => d !== trimmed),
          ].slice(0, MAX_RECENT_DIRS);
          return { settings: { ...state.settings, recentDirs: recent } };
        }),
    }),
    {
      name: "extractr-settings",
      // 只持久化设置，不持久化任务（任务重启后无意义）。
      partialize: (state) => ({ settings: state.settings }),
    }
  )
);
