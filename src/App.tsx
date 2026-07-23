import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Package, ChevronLeft, AlertCircle } from "lucide-react";
import { Header } from "./components/Header";
import { DropZone } from "./components/DropZone";
import { ArchivePreview } from "./components/ArchivePreview";
import { DestinationPicker } from "./components/DestinationPicker";
import { PasswordPrompt } from "./components/PasswordPrompt";
import { TaskQueue } from "./components/TaskQueue";
import { SettingsPanel } from "./components/SettingsPanel";
import { Button, Badge } from "./components/ui";
import { useTheme } from "./hooks/useTheme";
import { useDragDrop } from "./hooks/useDragDrop";
import { useExtract } from "./hooks/useExtract";
import { useAppStore } from "./store/useAppStore";
import { listArchive, detectFormat, toArchiveError } from "./lib/ipc";
import { basename } from "./lib/format";
import type { ArchiveErrorDto, EntryDto } from "./lib/types";

export default function App() {
  useTheme();
  const { t } = useTranslation();
  const { run } = useExtract();
  const settings = useAppStore((s) => s.settings);

  const [file, setFile] = useState<string | null>(null);
  const [entries, setEntries] = useState<EntryDto[]>([]);
  const [dest, setDest] = useState("");
  const [format, setFormat] = useState("");
  const [password, setPassword] = useState<string | null>(null);
  const [askPassword, setAskPassword] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<ArchiveErrorDto | null>(null);

  // 请求世代：只有最新一次选择的异步结果可以更新预览，防止 A→B 切换时
  // 旧请求迟到覆盖新预览。
  const loadToken = useRef(0);

  const loadFile = useCallback(
    async (p: string, suppliedPassword: string | null = null) => {
      const token = ++loadToken.current;
      setFile(p);
      setEntries([]);
      setPassword(suppliedPassword);
      setFormat("");
      setLoadError(null);
      setLoading(true);
      try {
        void detectFormat(p)
          .then((fmt) => {
            if (loadToken.current === token) setFormat(fmt);
          })
          .catch(() => {});
        const list = await listArchive({ path: p, password: suppliedPassword });
        // 请求过期：丢弃本次结果，避免覆盖更新的文件预览。
        if (loadToken.current !== token) return;
        setEntries(list);
        const dir = p.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
        const name = basename(p).replace(/\.[^.]+$/, "");
        setDest(`${dir}/${name}`);
      } catch (err) {
        if (loadToken.current !== token) return;
        const dto = toArchiveError(err);
        if (dto.code === "password_required" || dto.code === "wrong_password") {
          setAskPassword(true);
        } else {
          setLoadError(dto);
        }
      } finally {
        if (loadToken.current === token) setLoading(false);
      }
    },
    []
  );

  useDragDrop((paths) => {
    // 多文件拖入：当前为单归档预览模型，明确提示而非静默丢弃。
    if (paths.length === 1) {
      void loadFile(paths[0]);
    } else if (paths.length > 1) {
      setLoadError({
        code: "io",
        message: t("drop.oneAtATime"),
      });
    }
  });

  const handleExtract = () => {
    if (!file || !dest) return;
    if (entries.some((e) => e.is_encrypted) && !password) {
      setAskPassword(true);
      return;
    }
    void run(file, dest, password, settings.overwrite);
  };

  return (
    <div className="flex h-full flex-col bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <Header onOpenSettings={() => setShowSettings(true)} />
      <main className="flex min-h-0 flex-1 flex-col overflow-hidden p-5">
        {file ? (
          <div className="flex min-h-0 flex-1 flex-col gap-4">
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                onClick={() => {
                  setFile(null);
                  setEntries([]);
                  setLoadError(null);
                }}
                className="px-2"
              >
                <ChevronLeft size={16} /> {t("common.cancel")}
              </Button>
              {format && <Badge color="indigo">{format.toUpperCase()}</Badge>}
            </div>
            {loadError && (
              <div className="flex items-start gap-2 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300">
                <AlertCircle size={16} className="mt-0.5 shrink-0" />
                <span className="break-all">{loadError.message}</span>
              </div>
            )}
            <ArchivePreview path={file} format={format} entries={entries} />
            <div className="flex flex-col gap-2">
              <label className="text-xs font-medium text-zinc-500">
                {t("dest.label")}
              </label>
              <DestinationPicker value={dest} onChange={setDest} />
            </div>
            <div className="flex justify-end">
              <Button onClick={handleExtract} disabled={!dest || loading}>
                <Package size={16} /> {t("common.extract")}
              </Button>
            </div>
          </div>
        ) : (
          <DropZone onFile={(p) => loadFile(p)} />
        )}
      </main>
      <TaskQueue />
      <PasswordPrompt
        open={askPassword}
        onClose={() => setAskPassword(false)}
        onSubmit={(pw) => {
          setAskPassword(false);
          if (file) void loadFile(file, pw);
        }}
      />
      <SettingsPanel open={showSettings} onClose={() => setShowSettings(false)} />
    </div>
  );
}
