import { useState } from "react";
import { Lock } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Modal, Button } from "./ui";

export function PasswordPrompt({
  open,
  onClose,
  onSubmit,
}: {
  open: boolean;
  onClose: () => void;
  onSubmit: (pw: string) => void;
}) {
  const { t } = useTranslation();
  const [pw, setPw] = useState("");
  return (
    <Modal open={open} onClose={onClose} title={t("password.title")}>
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-700 dark:bg-amber-900/20 dark:text-amber-300">
          <Lock size={16} />
          <span>{t("password.placeholder")}</span>
        </div>
        <input
          type="password"
          autoFocus
          value={pw}
          onChange={(e) => setPw(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && pw) {
              onSubmit(pw);
              setPw("");
            }
          }}
          placeholder={t("password.placeholder")}
          className="rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-900 outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
        />
        <div className="flex justify-end gap-2">
          <Button
            variant="secondary"
            onClick={() => {
              onClose();
              setPw("");
            }}
          >
            {t("common.cancel")}
          </Button>
          <Button
            onClick={() => {
              if (pw) {
                onSubmit(pw);
                setPw("");
              }
            }}
          >
            {t("common.confirm")}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
