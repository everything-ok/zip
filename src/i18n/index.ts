import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zh from "./zh.json";
import en from "./en.json";

// 初始语言优先读持久化的设置；无则按浏览器/系统语言推断，默认中文。
function detectLanguage(): string {
  try {
    const raw = localStorage.getItem("extractr-settings");
    if (raw) {
      const parsed = JSON.parse(raw);
      const lang = parsed?.state?.settings?.language;
      if (lang === "zh" || lang === "en") return lang;
    }
  } catch {
    /* ignore */
  }
  const nav = (typeof navigator !== "undefined" && navigator.language) || "zh";
  return nav.toLowerCase().startsWith("en") ? "en" : "zh";
}

i18n.use(initReactI18next).init({
  resources: {
    zh: { translation: zh },
    en: { translation: en },
  },
  lng: detectLanguage(),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
