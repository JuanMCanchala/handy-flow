import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LiveSubtitleLine } from "@/bindings";
import { useMoveMode } from "./useMoveMode";
import "./LiveSubtitlesOverlay.css";

// Shows the last two subtitle lines, newest at the bottom; each translation
// streams into its line as it is generated. Purely a passive display: the
// window is transparent, always-on-top, click-through, and excluded from
// screen capture where the platform supports it (see
// live_translate/overlay.rs). Suggested answers live in their own panel
// (AnswersOverlay). In move mode the window becomes a drag handle.
const LiveSubtitlesOverlay: React.FC = () => {
  const { t } = useTranslation();
  const moving = useMoveMode();
  const [lines, setLines] = useState<LiveSubtitleLine[]>([]);

  useEffect(() => {
    const unlistenLine = listen<LiveSubtitleLine>(
      "live-subtitle-line",
      (event) => {
        // Same id = the translation for a line already shown: update in place.
        setLines((prev) => {
          const index = prev.findIndex((line) => line.id === event.payload.id);
          if (index >= 0) {
            const next = [...prev];
            next[index] = event.payload;
            return next;
          }
          return [...prev, event.payload].slice(-2);
        });
      },
    );
    const unlistenHide = listen("live-subtitles-hide", () => setLines([]));

    return () => {
      unlistenLine.then((f) => f());
      unlistenHide.then((f) => f());
    };
  }, []);

  if (moving) {
    return (
      <div className="live-subtitles-container">
        <div className="live-subtitles-move" data-tauri-drag-region>
          <div className="live-subtitles-original">
            {t("liveOverlays.dragHint")}
          </div>
          <div className="live-subtitles-translation">
            {t("liveOverlays.subtitlesSample")}
          </div>
        </div>
      </div>
    );
  }

  if (lines.length === 0) {
    return (
      <div className="live-subtitles-container">
        <div className="live-subtitles-line">
          <div className="live-subtitles-original">
            {t("settings.liveTranslate.listening")}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="live-subtitles-container">
      {lines.map((line) => (
        <div className="live-subtitles-line" key={line.id}>
          <div className="live-subtitles-original">{line.original}</div>
          <div className="live-subtitles-translation">
            {line.translation || "…"}
          </div>
        </div>
      ))}
    </div>
  );
};

export default LiveSubtitlesOverlay;
