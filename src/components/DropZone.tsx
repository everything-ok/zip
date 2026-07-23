import { UploadCloud } from "lucide-react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";

export function DropZone({ onFile }: { onFile: (path: string) => void }) {
  const { t } = useTranslation();

  const pick = async () => {
    // 单归档预览模型：一次只接受一个文件，避免静默丢弃其余选择。
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Archives",
          extensions: [
            "zip",
            "7z",
            "rar",
            "tar",
            "gz",
            "bz2",
            "xz",
            "zst",
            "tgz",
            "tbz2",
            "txz",
          ],
        },
      ],
    });
    if (typeof selected === "string") onFile(selected);
  };

  return (
    <div className="flex flex-1 items-center justify-center p-8">
      <button
        onClick={pick}
        className="group flex w-full max-w-xl flex-col items-center justify-center gap-3 rounded-2xl border-2 border-dashed border-zinc-300 bg-zinc-50 px-8 py-16 text-center transition-colors hover:border-indigo-400 hover:bg-indigo-50 dark:border-zinc-700 dark:bg-zinc-900/50 dark:hover:border-indigo-500 dark:hover:bg-indigo-950/30"
      >
        <div className="flex h-14 w-14 items-center justify-center rounded-full bg-indigo-100 text-indigo-600 transition-transform group-hover:scale-105 dark:bg-indigo-900/40 dark:text-indigo-300">
          <UploadCloud size={28} />
        </div>
        <div className="text-base font-medium text-zinc-700 dark:text-zinc-200">
          {t("drop.hint")}
        </div>
        <div className="text-sm text-zinc-500">{t("drop.click")}</div>
      </button>
    </div>
  );
}
