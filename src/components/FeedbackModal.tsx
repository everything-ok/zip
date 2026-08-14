import { useState } from "react";
import { Image as ImageIcon, Video, Send, X, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Button, Modal } from "./ui";
import { basename } from "../lib/format";

const DEFAULT_EMAIL = "2677989813@qq.com";
const MAX_TEXT = 500;
const MAX_IMAGES = 3;
const MAX_VIDEO_MB = 30;

interface FileItem {
  path: string;
  name: string;
  size: number;
}

export function FeedbackModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const [images, setImages] = useState<FileItem[]>([]);
  const [video, setVideo] = useState<FileItem | null>(null);
  const [sending, setSending] = useState(false);
  const [result, setResult] = useState<"idle" | "ok" | "err">("idle");
  const [errMsg, setErrMsg] = useState("");

  const addImages = async () => {
    const sel = await openDialog({
      multiple: true,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "bmp", "webp"] }],
    });
    if (sel) {
      const list = (Array.isArray(sel) ? sel : [sel]) as string[];
      const items = await Promise.all(
        list.map(async (p) => ({
          path: p,
          name: basename(p),
          size: await fileSize(p),
        }))
      );
      setImages((prev) => [...prev, ...items].slice(0, MAX_IMAGES));
    }
  };

  const addVideo = async () => {
    const sel = await openDialog({
      multiple: false,
      filters: [{ name: "Video", extensions: ["mp4"] }],
    });
    if (typeof sel === "string") {
      const size = await fileSize(sel);
      if (size > MAX_VIDEO_MB * 1024 * 1024) {
        setErrMsg(t("feedback.videoTooLarge", { max: `${MAX_VIDEO_MB}MB` }));
        return;
      }
      setVideo({ path: sel, name: basename(sel), size });
    }
  };

  const submit = async () => {
    if (!text.trim()) {
      setErrMsg(t("feedback.textRequired"));
      return;
    }
    setSending(true);
    setResult("idle");
    setErrMsg("");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("send_feedback", {
        req: {
          email: DEFAULT_EMAIL,
          text,
          images: images.map((i) => i.path),
          video: video?.path ?? null,
        },
      });
      setResult("ok");
      setText("");
      setImages([]);
      setVideo(null);
    } catch (e) {
      setResult("err");
      setErrMsg(String(e));
    } finally {
      setSending(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={t("feedback.title")}>
      <div className="flex flex-col gap-4">
        {result === "ok" ? (
          <div className="flex flex-col items-center gap-3 py-6 text-center">
            <div className="flex h-12 w-12 items-center justify-center rounded-full bg-emerald-100 text-emerald-600">
              <Send size={24} />
            </div>
            <div className="text-sm font-medium text-emerald-600">
              {t("feedback.sent")}
            </div>
            <Button variant="secondary" onClick={onClose}>
              {t("common.close")}
            </Button>
          </div>
        ) : (
          <>
            <div className="rounded-lg bg-indigo-50 px-3 py-2 text-xs text-indigo-600 dark:bg-indigo-950/40 dark:text-indigo-300">
              {t("feedback.hint")}
            </div>
            {/* 描述 */}
            <div className="flex flex-col gap-1">
              <div className="flex items-center justify-between">
                <label className="text-xs font-medium text-zinc-500">
                  {t("feedback.description")}
                </label>
                <span className="text-[10px] text-zinc-400">
                  {text.length}/{MAX_TEXT}
                </span>
              </div>
              <textarea
                value={text}
                onChange={(e) => setText(e.target.value.slice(0, MAX_TEXT))}
                rows={4}
                placeholder={t("feedback.descPlaceholder")}
                className="resize-none rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
              />
            </div>
            {/* 图片 */}
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-medium text-zinc-500">
                {t("feedback.images")} ({t("feedback.maxN", { n: MAX_IMAGES })})
              </label>
              <div className="flex flex-wrap gap-2">
                {images.map((img, i) => (
                  <div
                    key={i}
                    className="group relative flex items-center gap-1 rounded-md bg-zinc-100 px-2 py-1 text-xs dark:bg-zinc-800"
                  >
                    <ImageIcon size={12} className="text-zinc-400" />
                    <span className="max-w-[120px] truncate">{img.name}</span>
                    <button
                      onClick={() => setImages((prev) => prev.filter((_, idx) => idx !== i))}
                      className="text-zinc-400 hover:text-red-500"
                    >
                      <X size={11} />
                    </button>
                  </div>
                ))}
                {images.length < MAX_IMAGES && (
                  <button
                    onClick={addImages}
                    className="flex items-center gap-1 rounded-md border border-dashed border-zinc-300 px-2 py-1 text-xs text-zinc-500 hover:border-indigo-400 hover:text-indigo-500 dark:border-zinc-600"
                  >
                    <ImageIcon size={12} /> {t("feedback.add")}
                  </button>
                )}
              </div>
            </div>
            {/* 视频 */}
            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-medium text-zinc-500">
                {t("feedback.video")} ({t("feedback.maxSize", { size: `${MAX_VIDEO_MB}MB` })})
              </label>
              <div className="flex flex-wrap gap-2">
                {video ? (
                  <div className="group relative flex items-center gap-1 rounded-md bg-zinc-100 px-2 py-1 text-xs dark:bg-zinc-800">
                    <Video size={12} className="text-zinc-400" />
                    <span className="max-w-[120px] truncate">{video.name}</span>
                    <button
                      onClick={() => setVideo(null)}
                      className="text-zinc-400 hover:text-red-500"
                    >
                      <X size={11} />
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={addVideo}
                    className="flex items-center gap-1 rounded-md border border-dashed border-zinc-300 px-2 py-1 text-xs text-zinc-500 hover:border-indigo-400 hover:text-indigo-500 dark:border-zinc-600"
                  >
                    <Video size={12} /> {t("feedback.add")}
                  </button>
                )}
              </div>
            </div>
            {errMsg && (
              <div className="rounded-lg bg-red-50 px-3 py-2 text-xs text-red-600 dark:bg-red-900/20 dark:text-red-300">
                {errMsg}
              </div>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={onClose} disabled={sending}>
                {t("common.cancel")}
              </Button>
              <Button onClick={submit} disabled={sending || !text.trim()}>
                {sending ? (
                  <>
                    <Loader2 size={16} className="animate-spin" /> {t("feedback.sending")}
                  </>
                ) : (
                  <>
                    <Send size={16} /> {t("feedback.send")}
                  </>
                )}
              </Button>
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}

/** 获取文件大小（用后端 invoke 包装，避免依赖 plugin-fs）。 */
async function fileSize(path: string): Promise<number> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<number>("file_size", { path });
  } catch {
    return 0;
  }
}
