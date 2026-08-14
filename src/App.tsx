import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import {
  Package,
  ChevronLeft,
  AlertCircle,
  ExternalLink,
  Archive,
} from "lucide-react";
import { Header } from "./components/Header";
import { DropZone } from "./components/DropZone";
import { ArchivePreview } from "./components/ArchivePreview";
import { DestinationPicker } from "./components/DestinationPicker";
import { PasswordPrompt } from "./components/PasswordPrompt";
import { TaskQueue } from "./components/TaskQueue";
import { SettingsPanel } from "./components/SettingsPanel";
import { Button, Badge } from "./components/ui";
import { ToastContainer } from "./hooks/useToast";
import { useTheme } from "./hooks/useTheme";
import { useDragDrop } from "./hooks/useDragDrop";
import { useExtract } from "./hooks/useExtract";
import { useAppStore } from "./store/useAppStore";
import {
  listArchive,
  detectFormat,
  toArchiveError,
  popPendingOpen,
  openDefaultAppsSettings,
} from "./lib/ipc";
import { describeError } from "./lib/errorMap";
import { basename } from "./lib/format";
import type {
  ArchiveErrorDto,
  EntryDto,
  OpenArchiveAction,
} from "./lib/types";
import { CompressPanel } from "./components/CompressPanel";
import { ConvertPanel } from "./components/ConvertPanel";
import { FeedbackModal } from "./components/FeedbackModal";

const DEFAULT_GUIDE_KEY = "extractr-default-guide-shown";
const IS_MAC = typeof navigator !== "undefined" && /Mac|iPod|iPhone|iPad/.test(navigator.platform);

export default function App() {
  useTheme();
  const { t } = useTranslation();
  const { run } = useExtract();
  const { create: createArchive, test } = useExtract();
  const settings = useAppStore((s) => s.settings);
  const addRecentDir = useAppStore((s) => s.addRecentDir);

  const [file, setFile] = useState<string | null>(null);
  const [entries, setEntries] = useState<EntryDto[]>([]);
  const [dest, setDest] = useState("");
  const [format, setFormat] = useState("");
  const [password, setPassword] = useState<string | null>(null);
  const [askPassword, setAskPassword] = useState(false);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [feedbackOpen, setFeedbackOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<ArchiveErrorDto | null>(null);
  // 压缩面板状态提到 App 层：右键压缩 argv 可预填源；解压页不再有浮动按钮。
  const [compressOpen, setCompressOpen] = useState(false);
  const [compressSource, setCompressSource] = useState<string | undefined>();
  const [convertOpen, setConvertOpen] = useState(false);
  // 首次启动引导：提示用户设 Extractr 为默认程序（仅显示一次）。
  const [showDefaultGuide, setShowDefaultGuide] = useState(
    () => !localStorage.getItem(DEFAULT_GUIDE_KEY)
  );

  // 请求世代：只有最新一次选择的异步结果可以更新预览，防止 A→B 切换时
  // 旧请求迟到覆盖新预览。
  const loadToken = useRef(0);

  const loadFile = useCallback(
    async (p: string, suppliedPassword: string | null = null) => {
      const token = ++loadToken.current;
      // 记忆密码：若未显式传密码，尝试从 localStorage 取回该归档的密码。
      const effectivePassword = suppliedPassword ?? localStorage.getItem(`extractr-pw-${p}`);
      setFile(p);
      setEntries([]);
      setPassword(effectivePassword);
      setFormat("");
      setLoadError(null);
      setLoading(true);
      try {
        void detectFormat(p)
          .then((fmt) => {
            if (loadToken.current === token) setFormat(fmt);
          })
          .catch(() => {});
        const list = await listArchive({ path: p, password: effectivePassword });
        // 请求过期：丢弃本次结果，避免覆盖更新的文件预览。
        if (loadToken.current !== token) return;
        setEntries(list);
        // 密码正确后清错误。
        setPasswordError(null);
        const dir = p.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
        const name = basename(p).replace(/\.[^.]+$/, "");
        setDest(`${dir}/${name}`);
      } catch (err) {
        if (loadToken.current !== token) return;
        const dto = toArchiveError(err);
        if (dto.code === "password_required" || dto.code === "wrong_password") {
          setPasswordError(describeError(dto, t));
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

  // 批量拖入：首个文件进入预览，其余自动入队解压（复用单文件目标目录推导）。
  const handleDrop = useCallback(
    (paths: string[]) => {
      if (paths.length === 0) return;
      if (paths.length === 1) {
        void loadFile(paths[0]);
        return;
      }
      // 多文件：第一个进预览，其余按同名子目录直接入队。
      void loadFile(paths[0]);
      for (const p of paths.slice(1)) {
        const dir = p.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
        const name = basename(p).replace(/\.[^.]+$/, "");
        void run(p, `${dir}/${name}`, null, settings.overwrite);
      }
    },
    [loadFile, run, settings.overwrite]
  );

  // 监听文件关联/右键菜单启动事件：后端解析 argv 动作并 emit "open-archive"。
  // loadFile 用 ref 包裹以保证监听器稳定，避免每次重渲染重建。
  const loadFileRef = useRef(loadFile);
  const runRef = useRef(run);
  loadFileRef.current = loadFile;
  runRef.current = run;

  const applyAction = useCallback(
    (action: OpenArchiveAction["action"], path: string) => {
      if (!path) return;
      // 右键菜单已简化为只"用 Extractr 打开"（open），进预览后由 UI 配置目标/覆盖策略/密码。
      // 旧版 argv 的 extractHere/extractToSubdir 仍兼容处理（直接解压，不再有 UI 配置）。
      if (action === "open") {
        void loadFileRef.current(path);
      } else if (action === "extractHere" || action === "extractToSubdir") {
        const dir = path.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
        const target =
          action === "extractHere"
            ? dir || "."
            : `${dir}/${basename(path).replace(/\.[^.]+$/, "")}`;
        void runRef.current(path, target, null, settings.overwrite);
      } else if (action === "compress") {
        // 文件夹右键"用 Extractr 压缩"：打开压缩面板预填该文件夹。
        setCompressSource(path);
        setCompressOpen(true);
      }
    },
    [settings.overwrite]
  );

  useEffect(() => {
    // 去重：监听到 emit 即标记，避免 ready 后 pop 重复执行同一动作。
    let fired = false;
    const unlisten = listen<OpenArchiveAction>("open-archive", (e) => {
      fired = true;
      const { action, path } = e.payload;
      applyAction(action, path);
    });
    // 兜底：若 emit 时 webview 未就绪导致丢失，前端 ready 后取回缓存动作。
    // 仅当监听未收到时消费，避免重复执行。
    popPendingOpen().then((pending) => {
      if (pending && !fired) applyAction(pending.action as never, pending.path);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [applyAction]);

  const dismissDefaultGuide = () => {
    localStorage.setItem(DEFAULT_GUIDE_KEY, "1");
    setShowDefaultGuide(false);
  };

  const { hovering } = useDragDrop(handleDrop);

  const handleExtract = () => {
    if (!file || !dest) return;
    if (entries.some((e) => e.is_encrypted) && !password) {
      setAskPassword(true);
      return;
    }
    addRecentDir(dest);
    void run(file, dest, password, settings.overwrite);
  };

  // 压缩入口回调：CompressPanel 触发。
  const handleCreate = useCallback(
    (
      dest: string,
      sources: { fs_path: string; archive_path: string }[],
      password: string | null,
      level: number | null
    ) => void createArchive(dest, sources, password, level),
    [createArchive]
  );

  return (
    <div className="relative flex h-full flex-col bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <Header
        onOpenSettings={() => setShowSettings(true)}
        onCompress={() => {
          setCompressSource(undefined);
          setCompressOpen(true);
        }}
        onOpenFeedback={() => setFeedbackOpen(true)}
      />
      {showDefaultGuide && !file && !IS_MAC && (
        <div className="flex items-center gap-3 border-b border-indigo-200 bg-indigo-50 px-5 py-2 text-sm text-indigo-700 dark:border-indigo-800 dark:bg-indigo-950/40 dark:text-indigo-300">
          <ExternalLink size={15} className="shrink-0" />
          <span className="flex-1">{t("default.guide")}</span>
          <button
            onClick={() => void openDefaultAppsSettings()}
            className="shrink-0 rounded-md bg-indigo-600 px-2.5 py-1 text-xs font-medium text-white hover:bg-indigo-500"
          >
            {t("default.setNow")}
          </button>
          <button
            onClick={dismissDefaultGuide}
            className="shrink-0 text-xs text-indigo-400 hover:text-indigo-600"
          >
            {t("common.dismiss")}
          </button>
        </div>
      )}
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
                <span className="break-all">{describeError(loadError, t)}</span>
              </div>
            )}
            <ArchivePreview path={file} format={format} entries={entries} loading={loading}
              onConvert={() => setConvertOpen(true)}
              onTest={file ? () => void test(file, password) : undefined}
            />
            <div className="flex flex-col gap-2">
              <label className="text-xs font-medium text-zinc-500">
                {t("dest.label")}
              </label>
              <DestinationPicker
                value={dest}
                onChange={setDest}
                onPickRecent={addRecentDir}
              />
            </div>
            <div className="flex justify-end">
              <Button onClick={handleExtract} disabled={!dest || loading}>
                <Package size={16} /> {t("common.extract")}
              </Button>
            </div>
          </div>
        ) : (
          <DropZone onFile={(p) => loadFile(p)} hovering={hovering} />
        )}
      </main>
      <TaskQueue />
      {/* 压缩面板：右键文件夹"用 Extractr 压缩"或 Header 入口触发。解压页无浮动按钮。 */}
      {compressOpen && (
        <CompressPanel
          onClose={() => {
            setCompressOpen(false);
            setCompressSource(undefined);
          }}
          initialSource={compressSource}
        />
      )}
      {convertOpen && file && (
        <ConvertPanel
          onClose={() => setConvertOpen(false)}
          source={file}
          sourceFormat={format}
        />
      )}
      {hovering && (
        <div className="pointer-events-none fixed inset-0 z-40 flex items-center justify-center bg-indigo-500/10 backdrop-blur-[1px]">
          <div className="rounded-2xl border-2 border-dashed border-indigo-400 bg-white/80 px-10 py-8 text-center shadow-lg dark:bg-zinc-900/80">
            <div className="text-base font-medium text-indigo-600 dark:text-indigo-300">
              {t("drop.hovering")}
            </div>
            <div className="mt-1 text-sm text-zinc-500">
              {t("drop.hoveringHint")}
            </div>
          </div>
        </div>
      )}
      <PasswordPrompt
        open={askPassword}
        onClose={() => {
          setAskPassword(false);
          setPasswordError(null);
        }}
        error={passwordError}
        onSubmit={(pw, remember) => {
          setAskPassword(false);
          setPasswordError(null);
          if (file) {
            void loadFile(file, pw);
            if (remember) {
              localStorage.setItem(`extractr-pw-${file}`, pw);
            }
          }
        }}
      />
      <SettingsPanel open={showSettings} onClose={() => setShowSettings(false)} />
      <FeedbackModal open={feedbackOpen} onClose={() => setFeedbackOpen(false)} />
      <ToastContainer />
    </div>
  );
}
