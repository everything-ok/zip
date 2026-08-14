import { useState, useEffect } from "react";
import {
  Archive,
  Settings as SettingsIcon,
  Info,
  Download,
  MessageSquare,
  PackagePlus,
  HelpCircle,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button, Modal } from "./ui";
import { openUrlNative, checkUpdate, type UpdateInfo } from "../lib/ipc";

const APP_VERSION = "0.2.5";

type MenuTab = "feedback" | "formats" | "about";

export function Header({
  onOpenSettings,
  onCompress,
  onOpenFeedback,
}: {
  onOpenSettings: () => void;
  onCompress: () => void;
  onOpenFeedback: () => void;
}) {
  const { t } = useTranslation();
  const [menuOpen, setMenuOpen] = useState(false);
  const [menuTab, setMenuTab] = useState<MenuTab>("feedback");
  const [update, setUpdate] = useState<UpdateInfo | null>(null);

  // 启动后异步检查更新（失败静默，不打扰用户）。
  useEffect(() => {
    let active = true;
    checkUpdate().then((info) => {
      if (active && info) setUpdate(info);
    });
    return () => {
      active = false;
    };
  }, []);

  return (
    <>
      <header className="flex h-14 shrink-0 items-center justify-between border-b border-zinc-200 px-5 dark:border-zinc-800">
        <div className="flex items-center gap-2.5" />
        <div className="flex items-center gap-1">
          {update && (
            <button
              onClick={() => void openUrlNative(update.url).catch(() => {})}
              title={t("update.available", { version: update.version })}
              className="flex items-center gap-1 rounded-md bg-emerald-100 px-2 py-1 text-xs font-medium text-emerald-700 hover:bg-emerald-200 dark:bg-emerald-900/40 dark:text-emerald-300"
            >
              <Download size={12} /> v{update.version}
            </button>
          )}
          <Button
            variant="ghost"
            onClick={onCompress}
            aria-label={t("compress.title")}
            title={t("compress.title")}
            className="px-2.5"
          >
            <PackagePlus size={18} />
          </Button>
          <Button
            variant="ghost"
            onClick={() => setMenuOpen(true)}
            aria-label={t("menu.title")}
            title={t("menu.title")}
            className="px-2.5"
          >
            <HelpCircle size={18} />
          </Button>
          <Button
            variant="ghost"
            onClick={onOpenSettings}
            aria-label={t("settings.title")}
            className="px-2.5"
          >
            <SettingsIcon size={18} />
          </Button>
        </div>
      </header>

      {/* 帮助与反馈：反馈问题 / 支持格式 / 关于 合并为一个 Modal，顶部分段 Tab 切换。 */}
      <MenuModal
        open={menuOpen}
        tab={menuTab}
        onTabChange={setMenuTab}
        onClose={() => setMenuOpen(false)}
        onOpenFeedback={onOpenFeedback}
      />
    </>
  );
}

function MenuModal({
  open,
  tab,
  onTabChange,
  onClose,
  onOpenFeedback,
}: {
  open: boolean;
  tab: MenuTab;
  onTabChange: (tab: MenuTab) => void;
  onClose: () => void;
  onOpenFeedback: () => void;
}) {
  const { t } = useTranslation();
  return (
    <Modal open={open} onClose={onClose}>
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-zinc-900 dark:text-zinc-100">
            {t("menu.title")}
          </h3>
        </div>
        {/* 分段 Tab：反馈 / 支持格式 / 关于 */}
        <div className="flex gap-1 rounded-lg bg-zinc-100 p-1 dark:bg-zinc-800">
          <MenuTabButton
            active={tab === "feedback"}
            onClick={() => onTabChange("feedback")}
            icon={<MessageSquare size={14} />}
            label={t("menu.feedback")}
          />
          <MenuTabButton
            active={tab === "formats"}
            onClick={() => onTabChange("formats")}
            icon={<Info size={14} />}
            label={t("menu.formats")}
          />
          <MenuTabButton
            active={tab === "about"}
            onClick={() => onTabChange("about")}
            icon={<Archive size={14} />}
            label={t("menu.about")}
          />
        </div>
        <div className="min-h-[260px]">
          {tab === "feedback" && (
            <div className="flex flex-col gap-3">
              <p className="text-sm text-zinc-600 dark:text-zinc-300">
                {t("feedback.hint")}
              </p>
              <Button
                onClick={() => {
                  onClose();
                  onOpenFeedback();
                }}
              >
                <MessageSquare size={16} /> {t("feedback.title")}
              </Button>
            </div>
          )}
          {tab === "formats" && (
            <div className="flex flex-col gap-3 text-sm text-zinc-700 dark:text-zinc-300">
              <p className="leading-relaxed">{t("info.extractDesc")}</p>
              <div>
                <div className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-zinc-500">
                  {t("info.extractFormats")}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {[
                    "ZIP",
                    "7z",
                    "RAR",
                    "TAR",
                    "GZIP",
                    "BZIP2",
                    "XZ",
                    "Zstd",
                    ".tar.gz",
                    ".tar.xz",
                  ].map((f) => (
                    <span
                      key={f}
                      className="rounded-md bg-zinc-100 px-2 py-0.5 text-xs font-medium text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300"
                    >
                      {f}
                    </span>
                  ))}
                </div>
              </div>
              <div>
                <div className="mb-1.5 text-xs font-semibold uppercase tracking-wide text-zinc-500">
                  {t("info.createFormats")}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {["ZIP", "7z", "TAR", ".tar.gz", ".tar.xz"].map((f) => (
                    <span
                      key={f}
                      className="rounded-md bg-indigo-50 px-2 py-0.5 text-xs font-medium text-indigo-600 dark:bg-indigo-900/40 dark:text-indigo-300"
                    >
                      {f}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          )}
          {tab === "about" && (
            <div className="flex flex-col gap-3 text-sm text-zinc-700 dark:text-zinc-300">
              <div className="font-semibold text-zinc-900 dark:text-zinc-100">
                  {t("app.title")} v{APP_VERSION}
              </div>
              <p className="leading-relaxed">{t("about.desc")}</p>
              <ul className="ml-4 list-disc space-y-1 text-xs text-zinc-500">
                <li>{t("about.featureSafe")}</li>
                <li>{t("about.featureFormats")}</li>
                <li>{t("about.featureLight")}</li>
                <li>{t("about.featureCross")}</li>
              </ul>
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}

function MenuTabButton({
  active,
  onClick,
  icon,
  label,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex flex-1 items-center justify-center gap-1.5 rounded-md px-2 py-1.5 text-xs font-medium transition-colors ${
        active
          ? "bg-white text-indigo-600 shadow-sm dark:bg-zinc-900 dark:text-indigo-300"
          : "text-zinc-500 hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-200"
      }`}
    >
      {icon}
      {label}
    </button>
  );
}
