import { useEffect } from "react";
import { useAppStore } from "../store/useAppStore";
import type { ThemeMode } from "../store/useAppStore";

/** 解析主题模式为实际生效值：system 时跟随操作系统偏好。 */
function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode === "system") {
    return window.matchMedia?.("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return mode;
}

/** 应用主题（light/dark/system）到 <html> 的 class，system 模式跟随系统并实时响应。 */
export function useTheme() {
  const theme = useAppStore((s) => s.settings.theme);
  useEffect(() => {
    const apply = () => {
      const effective = resolveTheme(theme);
      document.documentElement.classList.toggle("dark", effective === "dark");
    };
    apply();
    // system 模式下监听系统主题变化，实时切换。
    if (theme === "system" && window.matchMedia) {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      mq.addEventListener("change", apply);
      return () => mq.removeEventListener("change", apply);
    }
  }, [theme]);
}
