import { useEffect, useRef } from "react";
import { clsx } from "clsx";
import { AnimatePresence, motion } from "framer-motion";
import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

export function Button({
  variant = "primary",
  className,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  const base =
    "inline-flex items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed";
  const variants: Record<Variant, string> = {
    primary: "bg-indigo-600 text-white hover:bg-indigo-500 shadow-sm",
    secondary:
      "bg-zinc-100 text-zinc-900 hover:bg-zinc-200 dark:bg-zinc-800 dark:text-zinc-100 dark:hover:bg-zinc-700",
    ghost: "text-zinc-600 hover:bg-zinc-100 dark:text-zinc-300 dark:hover:bg-zinc-800",
    danger: "bg-red-600 text-white hover:bg-red-500",
  };
  return <button className={clsx(base, variants[variant], className)} {...props} />;
}

type BadgeColor = "default" | "indigo" | "amber" | "emerald";

export function Badge({
  children,
  color = "default",
}: {
  children: ReactNode;
  color?: BadgeColor;
}) {
  const colors: Record<BadgeColor, string> = {
    default: "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300",
    indigo: "bg-indigo-100 text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300",
    amber: "bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300",
    emerald: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300",
  };
  return (
    <span
      className={clsx(
        "inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium",
        colors[color]
      )}
    >
      {children}
    </span>
  );
}

export function Modal({
  open,
  onClose,
  children,
  title,
}: {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  title?: string;
}) {
  // focus trap：Tab 在对话框内循环，Esc 关闭。
  const panelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const panel = panelRef.current;
    const prevActive = document.activeElement as HTMLElement | null;
    // 打开时聚焦对话框。
    const focusables = () =>
      panel?.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])'
      ) ?? null;
    focusables()?.[0]?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === "Tab") {
        const items = focusables();
        if (!items || items.length === 0) return;
        const first = items[0];
        const last = items[items.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      prevActive?.focus?.();
    };
  }, [open, onClose]);

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
          onClick={onClose}
        >
          <motion.div
            ref={panelRef}
            initial={{ scale: 0.96, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            exit={{ scale: 0.96, opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="w-full max-w-md rounded-2xl bg-white p-5 shadow-xl dark:bg-zinc-900"
            onClick={(e) => e.stopPropagation()}
          >
            {title && (
              <h3 className="mb-3 text-lg font-semibold text-zinc-900 dark:text-zinc-100">
                {title}
              </h3>
            )}
            {children}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
