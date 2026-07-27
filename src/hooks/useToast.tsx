import { create } from "zustand";
import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2, XCircle, Info, AlertCircle, X } from "lucide-react";

export type ToastKind = "success" | "error" | "info" | "warning";

export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
}

interface ToastState {
  toasts: Toast[];
  push: (kind: ToastKind, message: string, durationMs?: number) => string;
  dismiss: (id: string) => void;
}

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],
  push: (kind, message, durationMs = 4000) => {
    const id =
      typeof crypto !== "undefined" && crypto.randomUUID
        ? crypto.randomUUID()
        : `t-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    set((s) => ({ toasts: [...s.toasts, { id, kind, message }] }));
    if (durationMs > 0) {
      window.setTimeout(() => get().dismiss(id), durationMs);
    }
    return id;
  },
  dismiss: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

/** 便捷封装：成功/失败/信息提示。 */
export const toast = {
  success: (msg: string) => useToastStore.getState().push("success", msg),
  error: (msg: string) => useToastStore.getState().push("error", msg, 6000),
  info: (msg: string) => useToastStore.getState().push("info", msg),
  warning: (msg: string) => useToastStore.getState().push("warning", msg, 6000),
};

const iconFor = (kind: ToastKind) => {
  switch (kind) {
    case "success":
      return <CheckCircle2 size={18} className="text-emerald-500" />;
    case "error":
      return <XCircle size={18} className="text-red-500" />;
    case "warning":
      return <AlertCircle size={18} className="text-amber-500" />;
    default:
      return <Info size={18} className="text-indigo-500" />;
  }
};

/** 全局 Toast 容器，挂在应用根节点。 */
export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);
  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-[60] flex flex-col gap-2">
      <AnimatePresence>
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            initial={{ opacity: 0, x: 40 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 40 }}
            transition={{ duration: 0.2 }}
            className="pointer-events-auto flex max-w-sm items-start gap-2 rounded-lg border border-zinc-200 bg-white px-3 py-2 shadow-lg dark:border-zinc-700 dark:bg-zinc-800"
          >
            <span className="mt-0.5 shrink-0">{iconFor(t.kind)}</span>
            <span className="min-w-0 flex-1 break-words text-sm text-zinc-800 dark:text-zinc-100">
              {t.message}
            </span>
            <button
              onClick={() => dismiss(t.id)}
              className="shrink-0 text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
            >
              <X size={14} />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
