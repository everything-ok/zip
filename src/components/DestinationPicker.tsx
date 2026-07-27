import { FolderOpen, History, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../store/useAppStore";

export function DestinationPicker({
  value,
  onChange,
  onPickRecent,
}: {
  value: string;
  onChange: (v: string) => void;
  onPickRecent?: (dir: string) => void;
}) {
  const { t } = useTranslation();
  const recentDirs = useAppStore((s) => s.settings.recentDirs);

  const browse = async () => {
    const dir = await open({ directory: true });
    if (typeof dir === "string") {
      onChange(dir);
      onPickRecent?.(dir);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("dest.placeholder")}
          className="min-w-0 flex-1 rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
        />
        <button
          onClick={browse}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-zinc-100 px-3 py-2 text-sm text-zinc-700 transition-colors hover:bg-zinc-200 dark:bg-zinc-800 dark:text-zinc-200 dark:hover:bg-zinc-700"
        >
          <FolderOpen size={16} /> {t("common.browse")}
        </button>
      </div>
      {recentDirs.length > 0 && (
        <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
          <History size={13} className="shrink-0 text-zinc-400" />
          {recentDirs.map((dir) => {
            const name = dir.replace(/\\/g, "/").split("/").pop() || dir;
            return (
              <button
                key={dir}
                title={dir}
                onClick={() => {
                  onChange(dir);
                  onPickRecent?.(dir);
                }}
                className="shrink-0 rounded-md bg-zinc-100 px-2 py-1 text-xs text-zinc-600 transition-colors hover:bg-indigo-100 hover:text-indigo-700 dark:bg-zinc-800 dark:text-zinc-300 dark:hover:bg-indigo-900/40"
              >
                {name}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** 导出供 App 清除最近目录时复用的关闭图标占位。 */
export function RecentClear() {
  const recentDirs = useAppStore((s) => s.settings.recentDirs);
  const setSettings = useAppStore((s) => s.setSettings);
  const { t } = useTranslation();
  if (recentDirs.length === 0) return null;
  return (
    <button
      onClick={() => setSettings({ recentDirs: [] })}
      className="inline-flex items-center gap-1 text-xs text-zinc-400 hover:text-red-500"
    >
      <X size={11} /> {t("common.clearRecent")}
    </button>
  );
}
