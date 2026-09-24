import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { commands } from "@/bindings";

/**
 * Whether the overlays are in "move mode" (draggable, shown with a frame so
 * the user can place them). Toggled from the Live translate settings.
 */
export const useMoveMode = () => {
  const [moving, setMoving] = useState(false);

  useEffect(() => {
    commands.getLiveOverlaysMoveMode().then((result) => {
      if (result.status === "ok") setMoving(result.data);
    });
    const unlisten = listen<boolean>("live-overlays-move-mode", (event) =>
      setMoving(event.payload),
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return moving;
};
