import { useEffect, useState } from "react";
import { Lock, Eye, EyeOff } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Modal, Button } from "./ui";

export function PasswordPrompt({
  open,
  onClose,
  onSubmit,
  error,
}: {
  open: boolean;
  onClose: () => void;
  onSubmit: (pw: string, remember: boolean) => void;
  error?: string | null;
}) {
  const { t } = useTranslation();
  const [pw, setPw] = useState("");
  const [show, setShow] = useState(false);
  const [remember, setRemember] = useState(false);

  // 打开时清空输入，避免上次残留。
  useEffect(() => {
    if (open) {
      setPw("");
      setShow(false);
    }
  }, [open]);

  const submit = () => {
    if (pw) {
      onSubmit(pw, remember);
      setPw("");
    }
  };

  return (
    <Modal open={open} onClose={onClose} title={t("password.title")}>
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-700 dark:bg-amber-900/20 dark:text-amber-300">
          <Lock size={16} />
          <span>{t("password.placeholder")}</span>
        </div>
        {error && (
          <div className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700 dark:bg-red-900/20 dark:text-red-300">
            {error}
          </div>
        )}
        <div className="relative">
          <input
            type={show ? "text" : "password"}
            autoFocus
            value={pw}
            onChange={(e) => setPw(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") submit();
            }}
            placeholder={t("password.placeholder")}
            className="w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 pr-10 text-sm text-zinc-900 outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100"
          />
          <button
            type="button"
            onClick={() => setShow((s) => !s)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
            aria-label={show ? t("password.hide") : t("password.show")}
          >
            {show ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
        </div>
        <label className="flex cursor-pointer items-center gap-2 text-sm text-zinc-600 dark:text-zinc-400">
          <input
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
            className="h-4 w-4 rounded border-zinc-300 text-indigo-600 focus:ring-indigo-500 dark:border-zinc-600"
          />
          {t("password.remember")}
        </label>
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button onClick={submit}>{t("common.confirm")}</Button>
        </div>
      </div>
    </Modal>
  );
}
