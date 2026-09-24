import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useState } from "react";
import "./LiveSubtitlesOverlay.css";

interface LiveSubtitleLine {
  original: string;
  translation: string;
}

// Shows the last two translated subtitle lines. Purely a passive display:
// the window itself is transparent, always-on-top, click-through, and
// excluded from screen capture where the platform supports it (see
// live_translate/overlay.rs). It never steals focus.
const LiveSubtitlesOverlay: React.FC = () => {
  const [lines, setLines] = useState<LiveSubtitleLine[]>([]);

  useEffect(() => {
    const unlistenLine = listen<LiveSubtitleLine>("live-subtitle-line", (event) => {
      setLines((prev) => [...prev, event.payload].slice(-2));
    });
    const unlistenHide = listen("live-subtitles-hide", () => {
      setLines([]);
    });

    return () => {
      unlistenLine.then((f) => f());
      unlistenHide.then((f) => f());
    };
  }, []);

  if (lines.length === 0) {
    return null;
  }

  return (
    <div className="live-subtitles-container">
      {lines.map((line, index) => (
        <div className="live-subtitles-line" key={index}>
          <div className="live-subtitles-original">{line.original}</div>
          <div className="live-subtitles-translation">{line.translation}</div>
        </div>
      ))}
    </div>
  );
};

export default LiveSubtitlesOverlay;
