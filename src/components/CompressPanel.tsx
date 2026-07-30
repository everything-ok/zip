import { useState } from "react";
import { Archive, X, FolderPlus, File as FileIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { Button, Modal, Badge } from "./ui";
import { useExtract } from "../hooks/useExtract";
import { basename } from "../lib/format";

const FORMATS = ["zip", "7z", "tar.gz", "tar.xz", "tar"] as const;
type Format = (typeof FORMATS)[number];

interface Source {
  fs_path: string;
  archive_path: string;
}

export function CompressPanel({
  onClose,
  initialSource,
}: {
  onClose: () => void;
  initialSource?: string;
}) {
  const { t } = useTranslation();
  const { create } = useExtract();
  const [sources, setSources] = useState<Source[]>(() =>
    initialSource
      ? [{ fs_path: initialSource, archive_path: basename(initialSource) }]
      : []
  );
  // 预填目标目录为源所在目录。
  const [dest, setDest] = useState(() =>
    initialSource
      ? initialSource.replace(/\\/g, "/").split("/").slice(0, -1).join("/")
      : ""
  );
  const [format, setFormat] = useState<Format>("zip");
  const [password, setPassword] = useState("");
  const [level, setLevel] = useState(6);

  const addFiles = async () => {
    const sel = await open({ multiple: true, directory: false });
    if (sel) {
      const list = Array.isArray(sel) ? sel : [sel];
      setSources((prev) => [
        ...prev,
        ...list.map((p) => ({ fs_path: p, archive_path: basename(p) })),
      ]);
    }
  };

  const addDir = async () => {
    const sel = await open({ multiple: true, directory: true });
    if (sel) {
      const list = Array.isArray(sel) ? sel : [sel];
      setSources((prev) => [
        ...prev,
        ...list.map((p) => ({ fs_path: p, archive_path: basename(p) })),
      ]);
    }
  };

  const remove = (i: number) =>
    setSources((prev) => prev.filter((_, idx) => idx !== i));

  const chooseDest = async () => {
    const d = await open({ directory: true });
    if (typeof d === "string") setDest(d);
  };

  const submit = () => {
    if (sources.length === 0 || !dest) return;
    void create(
      `${dest.replace(/\\/g, "/").replace(/\/$/, "")}/archive.${format}`,
      sources,
      password || null,
      level
    );
    onClose();
  };

  const selectClass =
    "rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100";

  return (
    <Modal open={true} onClose={onClose} title={t("compress.title")}>
      <div className="flex flex-col gap-4">
        {/* 源文件列表 */}
        <div className="flex flex-col gap-2">
          <div className="flex gap-2">
            <Button variant="secondary" onClick={addFiles} className="flex-1">
              <FileIcon size={15} /> {t("compress.addFiles")}
            </Button>
            <Button variant="secondary" onClick={addDir} className="flex-1">
              <FolderPlus size={15} /> {t("compress.addDirs")}
            </Button>
          </div>
          <div className="max-h-40 overflow-auto rounded-lg border border-zinc-200 dark:border-zinc-700">
            {sources.length === 0 ? (
              <div className="px-3 py-6 text-center text-xs text-zinc-400">
                {t("compress.empty")}
              </div>
            ) : (
              sources.map((s, i) => (
                <div
                  key={i}
                  className="flex items-center gap-2 border-b border-zinc-100 px-3 py-1.5 last:border-0 dark:border-zinc-800"
                >
                  {s.fs_path.includes(".") ? (
                    <FileIcon size={13} className="shrink-0 text-zinc-400" />
                  ) : (
                    <FolderPlus size={13} className="shrink-0 text-indigo-400" />
                  )}
                  <span className="truncate text-xs text-zinc-700 dark:text-zinc-300">
                    {s.archive_path}
                  </span>
                  <span className="ml-auto shrink-0 text-[10px] text-zinc-400">
                    {s.fs_path.replace(/\\/g, "/").split("/").slice(-2, -1)}
                  </span>
                  <button
                    onClick={() => remove(i)}
                    className="shrink-0 text-zinc-400 hover:text-red-500"
                  >
                    <X size={13} />
                  </button>
                </div>
              ))
            )}
          </div>
        </div>

        {/* 目标格式 + 目录 */}
        <div className="flex flex-col gap-2">
          <label className="text-xs font-medium text-zinc-500">
            {t("compress.format")}
          </label>
          <div className="flex flex-wrap gap-1.5">
            {FORMATS.map((f) => (
              <button
                key={f}
                onClick={() => setFormat(f)}
                className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                  format === f
                    ? "bg-indigo-600 text-white"
                    : "bg-zinc-100 text-zinc-600 hover:bg-zinc-200 dark:bg-zinc-800 dark:text-zinc-300"
                }`}
              >
                {f}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <input
            value={dest}
            onChange={(e) => setDest(e.target.value)}
            placeholder={t("compress.destPlaceholder")}
            className="min-w-0 flex-1 rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
          />
          <Button variant="secondary" onClick={chooseDest}>
            {t("common.browse")}
          </Button>
        </div>

        {/* 压缩级别 */}
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-zinc-500">
            {t("compress.level")}
          </label>
          <div className="flex items-center gap-2">
            <input
              type="range"
              min={0}
              max={9}
              value={level}
              onChange={(e) => setLevel(Number(e.target.value))}
              className="w-32 accent-indigo-600"
            />
            <Badge color="indigo">{level}</Badge>
          </div>
        </div>

        {/* 密码（可选） */}
        {["zip", "7z"].includes(format) && (
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-zinc-500">
              {t("compress.password")} ({t("compress.optional")})
            </label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t("compress.passwordPlaceholder")}
              className="rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
            />
          </div>
        )}

        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button onClick={submit} disabled={sources.length === 0 || !dest}>
            <Archive size={16} /> {t("compress.create")}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
