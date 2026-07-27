import { useState } from "react";
import { Archive, Settings as SettingsIcon, Info } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button, Modal } from "./ui";

const APP_VERSION = "0.1.0";

export function Header({ onOpenSettings }: { onOpenSettings: () => void }) {
  const { t, i18n } = useTranslation();
  const [aboutOpen, setAboutOpen] = useState(false);

  return (
    <>
      <header className="flex h-14 shrink-0 items-center justify-between border-b border-zinc-200 px-5 dark:border-zinc-800">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-600 text-white shadow-sm">
            <Archive size={18} />
          </div>
          <div className="leading-tight">
            <div className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">
              {t("app.title")}
            </div>
            <div className="text-xs text-zinc-500">{t("app.subtitle")}</div>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            onClick={() => setAboutOpen(true)}
            aria-label={t("about.title")}
            className="px-2.5"
          >
            <Info size={18} />
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
      <Modal open={aboutOpen} onClose={() => setAboutOpen(false)} title={t("about.title")}>
        <div className="flex flex-col gap-3 text-sm text-zinc-700 dark:text-zinc-300">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-indigo-600 text-white">
              <Archive size={20} />
            </div>
            <div>
              <div className="font-semibold text-zinc-900 dark:text-zinc-100">
                {t("app.title")} v{APP_VERSION}
              </div>
              <div className="text-xs text-zinc-500">{t("app.subtitle")}</div>
            </div>
          </div>
          <p className="leading-relaxed">{t("about.desc")}</p>
          <ul className="ml-4 list-disc space-y-1 text-xs text-zinc-500">
            <li>{t("about.featureSafe")}</li>
            <li>{t("about.featureFormats")}</li>
            <li>{t("about.featureLight")}</li>
            <li>{t("about.featureCross")}</li>
          </ul>
        </div>
      </Modal>
    </>
  );
}
