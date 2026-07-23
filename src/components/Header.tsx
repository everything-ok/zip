import { Archive, Settings as SettingsIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "./ui";

export function Header({ onOpenSettings }: { onOpenSettings: () => void }) {
  const { t } = useTranslation();
  return (
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
      <Button
        variant="ghost"
        onClick={onOpenSettings}
        aria-label={t("settings.title")}
        className="px-2.5"
      >
        <SettingsIcon size={18} />
      </Button>
    </header>
  );
}
