import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";

export function DestinationPicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (v: string) => void;
}) {
  const { t } = useTranslation();
  const browse = async () => {
    const dir = await open({ directory: true });
    if (typeof dir === "string") onChange(dir);
  };
  return (
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
  );
}
