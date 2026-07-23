import { useEffect } from "react";
import { useAppStore } from "../store/useAppStore";

/** 应用主题（light/dark）到 <html> 的 class。 */
export function useTheme() {
  const theme = useAppStore((s) => s.settings.theme);
  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
  }, [theme]);
}
