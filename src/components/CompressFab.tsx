import { useState } from "react";
import { Archive } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CompressPanel } from "./CompressPanel";

interface CreateSource {
  fs_path: string;
  archive_path: string;
}

/** 浮动压缩入口：右下角按钮，点击打开压缩面板。 */
export function CompressFab({
  onCreate,
}: {
  onCreate: (
    dest: string,
    sources: CreateSource[],
    password: string | null,
    level: number | null
  ) => void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <>
      <button
        onClick={() => setOpen(true)}
        aria-label={t("compress.title")}
        title={t("compress.title")}
        className="fixed bottom-6 right-6 z-30 flex h-12 w-12 items-center justify-center rounded-full bg-indigo-600 text-white shadow-lg transition-transform hover:scale-105 hover:bg-indigo-500"
      >
        <Archive size={22} />
      </button>
      {open && (
        <CompressPanel
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}
