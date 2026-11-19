import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useState } from "react";

/** Reports whether a file drag hovers the window and forwards the first dropped path. */
export function useDropTarget(onDrop: (path: string) => void): boolean {
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" || payload.type === "over") return setDragging(true);
      setDragging(false);
      const [path] = payload.type === "drop" ? payload.paths : [];
      if (path) onDrop(path);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [onDrop]);

  return dragging;
}
