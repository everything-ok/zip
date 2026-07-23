import { useEffect } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

/** 监听 Tauri 窗口原生拖拽事件（文件落到窗口时回调路径）。 */
export function useDragDrop(onDrop: (paths: string[]) => void) {
  useEffect(() => {
    const unlistenPromise = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "drop") {
        onDrop(e.payload.paths);
      }
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, [onDrop]);
}
