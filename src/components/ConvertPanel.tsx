import { useState } from "react";
import { ArrowRight, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { Button, Modal, Badge } from "./ui";
import { useExtract } from "../hooks/useExtract";
import { basename } from "../lib/format";
import type { ConvertRequest } from "../lib/types";

const FORMATS = ["zip", "7z", "tar.gz", "tar.xz", "tar"] as const;
type Format = (typeof FORMATS)[number];

export function ConvertPanel({
  onClose,
  source,
  sourceFormat,
}: {
  onClose: () => void;
  source: string;
  sourceFormat: string;
}) {
  const { t } = useTranslation();
  const { convert } = useExtract();
  const [dest, setDest] = useState(() => {
    const dir = source.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
    const name = basename(source).replace(/\.[^.]+$/, "");
    return `${dir}/${name}`;
  });
  const [format, setFormat] = useState<Format>("zip");
  const [password, setPassword] = useState("");
  const [destPassword, setDestPassword] = useState("");
  const [level, setLevel] = useState(6);

  const chooseDest = async () => {
    const d = await open({ directory: true });
    if (typeof d === "string") setDest(d);
  };

  const submit = () => {
    if (!source || !dest) return;
    const name = (basename(source).replace(/\.[^.]+$/, "") || "archive");
    const destPath = `${dest.replace(/\\/g, "/").replace(/\/$/, "")}/${name}.${format}`;
    const req: ConvertRequest = {
      task_id: typeof crypto !== "undefined" && crypto.randomUUID
        ? crypto.randomUUID()
        : `cv-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      source,
      dest: destPath,
      password: password || null,
      dest_password: destPassword || null,
      level,
    };
    void convert(req);
    onClose();
  };

  const selectClass =
    "rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100";

  return (
    <Modal open={true} onClose={onClose} title={t("convert.title")}>
      <div className="flex flex-col gap-4">
        {/* 源文件 */}
        <div className="flex items-center gap-2 rounded-lg bg-zinc-100 px-3 py-2 dark:bg-zinc-800">
          <span className="truncate text-sm text-zinc-700 dark:text-zinc-300">
            {basename(source)}
          </span>
          {sourceFormat && <Badge color="indigo">{sourceFormat.toUpperCase()}</Badge>}
          <ArrowRight size={14} className="ml-auto shrink-0 text-zinc-400" />
        </div>

        {/* 目标格式 */}
        <div className="flex flex-col gap-2">
          <label className="text-xs font-medium text-zinc-500">
            {t("convert.targetFormat")}
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

        {/* 目标目录 */}
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

        {/* 源密码（若源归档加密） */}
        <div className="flex flex-col gap-1">
          <label className="text-xs font-medium text-zinc-500">
            {t("convert.sourcePassword")} ({t("compress.optional")})
          </label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder={t("compress.passwordPlaceholder")}
            className={selectClass}
          />
        </div>

        {/* 目标密码（可选） */}
        {["zip", "7z"].includes(format) && (
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-zinc-500">
              {t("convert.destPassword")} ({t("compress.optional")})
            </label>
            <input
              type="password"
              value={destPassword}
              onChange={(e) => setDestPassword(e.target.value)}
              placeholder={t("compress.passwordPlaceholder")}
              className={selectClass}
            />
          </div>
        )}

        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button onClick={submit} disabled={!dest}>
            <ArrowRight size={16} /> {t("convert.convert")}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
