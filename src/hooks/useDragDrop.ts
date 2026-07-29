import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

export interface DragDropState {
  /** 当前是否有文件正悬停在窗口上。 */
  hovering: boolean;
}

export type DragDropHandler = (paths: string[]) => void;

/**
 * 监听 Tauri 窗口原生拖拽事件，返回悬停态并回调落点。
 * - `enter`/`over`：标记悬停，供 UI 高亮；
 * - `drop`：回调路径并清除悬停；
 * - `leave`：清除悬停。
 *
 * 用 ref 持有最新 onDrop，监听只建一次，避免重渲染重建。
 */
export function useDragDrop(onDrop: DragDropHandler) {
  const [hovering, setHovering] = useState(false);
  const handlerRef = useRef(onDrop);
  handlerRef.current = onDrop;

  useEffect(() => {
    const unlistenPromise = getCurrentWebview().onDragDropEvent((e) => {
      const payload = e.payload as {
        type: "enter" | "over" | "leave" | "drop";
        paths?: string[];
      };
      switch (payload.type) {
        case "enter":
        case "over":
          setHovering(true);
          break;
        case "leave":
          setHovering(false);
          break;
        case "drop":
          setHovering(false);
          if (payload.paths && payload.paths.length > 0) {
            handlerRef.current(payload.paths);
          }
          break;
      }
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  }, []);

  return { hovering };
}
