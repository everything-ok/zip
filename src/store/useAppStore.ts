import { create } from "zustand";
import type { ArchiveErrorDto, SummaryDto } from "../lib/types";

export type TaskStatus = "pending" | "running" | "done" | "error" | "cancelled";
export type OverwritePolicy = "skip" | "overwrite" | "rename" | "error";

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
}

export interface Settings {
  theme: "light" | "dark";
  language: "zh" | "en";
  overwrite: OverwritePolicy;
}

interface AppState {
  tasks: Task[];
  settings: Settings;
  addTask: (t: Task) => void;
  updateTask: (id: string, patch: Partial<Task>) => void;
  removeTask: (id: string) => void;
  clearFinished: () => void;
  setSettings: (s: Partial<Settings>) => void;
}

export const useAppStore = create<AppState>((set) => ({
  tasks: [],
  settings: {
    theme: "light",
    language: "zh",
    overwrite: "skip",
  },
  addTask: (t) => set((s) => ({ tasks: [t, ...s.tasks] })),
  updateTask: (id, patch) =>
    set((s) => ({
      tasks: s.tasks.map((t) => (t.id === id ? { ...t, ...patch } : t)),
    })),
  removeTask: (id) => set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) })),
  clearFinished: () =>
    set((s) => ({
      tasks: s.tasks.filter((t) => t.status === "running" || t.status === "pending"),
    })),
  setSettings: (s) => set((state) => ({ settings: { ...state.settings, ...s } })),
}));
