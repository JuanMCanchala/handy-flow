import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import "./FlowBar.css";
import { commands } from "@/bindings";

// Time the collapse animation needs before the backend may shrink the window
// back to the resting pill (kept just above the CSS transition duration so the
// toolbar is never clipped mid-collapse).
const COLLAPSE_DELAY_MS = 200;

const LANGUAGE_LABELS: Record<string, string> = { es: "ES", en: "EN" };

const languageLabel = (language: string): string =>
  LANGUAGE_LABELS[language] ?? "AUTO";

interface FlowBarProps {
  position: "top" | "bottom";
}

// Idle Flow bar: a thin pill that expands into a toolbar while hovered.
const FlowBar: React.FC<FlowBarProps> = ({ position }) => {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [language, setLanguage] = useState("auto");
  const collapseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    commands
      .getAppSettings()
      .then((result) => {
        if (result.status === "ok") {
          setLanguage(result.data.selected_language ?? "auto");
        }
      })
      .catch(() => {
        // Keep the default label if settings can't be read.
      });
  }, []);

  useEffect(
    () => () => {
      if (collapseTimer.current) clearTimeout(collapseTimer.current);
    },
    [],
  );

  const handleEnter = () => {
    if (collapseTimer.current) {
      clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }
    // Grow the window first so the toolbar has room to animate into.
    commands.flowBarSetExpanded(true).catch(() => {});
    setExpanded(true);
  };

  const handleLeave = () => {
    setExpanded(false);
    if (collapseTimer.current) clearTimeout(collapseTimer.current);
    collapseTimer.current = setTimeout(() => {
      collapseTimer.current = null;
      commands.flowBarSetExpanded(false).catch(() => {});
    }, COLLAPSE_DELAY_MS);
  };

  const cycleLanguage = async () => {
    const result = await commands.flowBarCycleLanguage();
    if (result.status === "ok") setLanguage(result.data);
  };

  return (
    <div
      className={`ov-stage ${position} fbar-stage`}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
    >
      <div className={`fbar ${expanded ? "expanded" : ""}`}>
        <div className="fbar-tools">
          <button
            className="fbar-chip"
            title={t("overlay.flowBar.language")}
            aria-label={t("overlay.flowBar.language")}
            onClick={cycleLanguage}
          >
            {languageLabel(language)}
          </button>
          <button
            className="fbar-btn"
            title={t("overlay.flowBar.mic")}
            aria-label={t("overlay.flowBar.mic")}
            onClick={() => commands.flowBarToggleTranscribe()}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path
                d="M8 2a1.8 1.8 0 0 0-1.8 1.8v3.4a1.8 1.8 0 0 0 3.6 0V3.8A1.8 1.8 0 0 0 8 2Z"
                fill="currentColor"
              />
              <path
                d="M4.4 7.4a3.6 3.6 0 0 0 7.2 0M8 11v2.4M6.2 13.4h3.6"
                stroke="currentColor"
                strokeWidth="1.3"
                strokeLinecap="round"
                fill="none"
              />
            </svg>
          </button>
          <button
            className="fbar-btn"
            title={t("overlay.flowBar.liveTranslate")}
            aria-label={t("overlay.flowBar.liveTranslate")}
            onClick={() => commands.toggleLiveTranslate()}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <circle
                cx="8"
                cy="8"
                r="6"
                stroke="currentColor"
                strokeWidth="1.3"
                fill="none"
              />
              <path
                d="M2 8h12M8 2c1.6 1.7 2.4 3.7 2.4 6S9.6 12.3 8 14c-1.6-1.7-2.4-3.7-2.4-6S6.4 3.7 8 2Z"
                stroke="currentColor"
                strokeWidth="1.3"
                fill="none"
              />
            </svg>
          </button>
          <button
            className="fbar-btn"
            title={t("overlay.flowBar.scratchpad")}
            aria-label={t("overlay.flowBar.scratchpad")}
            onClick={() => commands.flowBarOpenScratchpad()}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path
                d="M4 2.5h8v11H4z"
                stroke="currentColor"
                strokeWidth="1.3"
                fill="none"
              />
              <path
                d="M6 5.5h4M6 8h4M6 10.5h2.5"
                stroke="currentColor"
                strokeWidth="1.3"
                strokeLinecap="round"
                fill="none"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
};

export default FlowBar;
