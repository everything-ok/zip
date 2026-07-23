import { useTranslation } from "react-i18next";
import { Modal } from "./ui";
import { useAppStore } from "../store/useAppStore";

export function SettingsPanel({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const { t, i18n } = useTranslation();
  const settings = useAppStore((s) => s.settings);
  const setSettings = useAppStore((s) => s.setSettings);

  const select =
    "rounded-lg border border-zinc-300 bg-white px-3 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100";

  return (
    <Modal open={open} onClose={onClose} title={t("settings.title")}>
      <div className="flex flex-col gap-4">
        <Row label={t("settings.language")}>
          <select
            value={settings.language}
            onChange={(e) => {
              const lang = e.target.value as "zh" | "en";
              setSettings({ language: lang });
              void i18n.changeLanguage(lang);
            }}
            className={select}
          >
            <option value="zh">中文</option>
            <option value="en">English</option>
          </select>
        </Row>
        <Row label={t("settings.theme")}>
          <select
            value={settings.theme}
            onChange={(e) =>
              setSettings({ theme: e.target.value as "light" | "dark" })
            }
            className={select}
          >
            <option value="light">{t("settings.light")}</option>
            <option value="dark">{t("settings.dark")}</option>
          </select>
        </Row>
        <Row label={t("settings.overwrite")}>
          <select
            value={settings.overwrite}
            onChange={(e) =>
              setSettings({ overwrite: e.target.value as never })
            }
            className={select}
          >
            <option value="skip">{t("settings.ow_skip")}</option>
            <option value="overwrite">{t("settings.ow_overwrite")}</option>
            <option value="rename">{t("settings.ow_rename")}</option>
            <option value="error">{t("settings.ow_error")}</option>
          </select>
        </Row>
      </div>
    </Modal>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-zinc-700 dark:text-zinc-300">{label}</span>
      {children}
    </div>
  );
}
