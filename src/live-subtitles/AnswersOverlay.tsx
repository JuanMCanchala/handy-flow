import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { CopilotAnswerLine } from "@/bindings";
import { useMoveMode } from "./useMoveMode";
import "./AnswersOverlay.css";

const MAX_VISIBLE = 2;

// Suggested answers, docked top-right. The newest answer streams in word by
// word at reading size; the previous one stays underneath, quieter, in case
// the interviewer circles back. Transparent and click-through (see
// live_translate/overlay.rs) except in move mode, where the whole panel is a
// drag handle.
const AnswersOverlay: React.FC = () => {
  const { t } = useTranslation();
  const moving = useMoveMode();
  const [answers, setAnswers] = useState<CopilotAnswerLine[]>([]);

  useEffect(() => {
    const unlistenAnswer = listen<CopilotAnswerLine>(
      "copilot-answer-line",
      (event) => {
        const line = event.payload;
        setAnswers((prev) => {
          if (line.done && !line.answer) {
            return prev.filter((a) => a.id !== line.id);
          }
          const index = prev.findIndex((a) => a.id === line.id);
          if (index >= 0) {
            const next = [...prev];
            next[index] = line;
            return next;
          }
          return [line, ...prev].slice(0, MAX_VISIBLE);
        });
      },
    );
    const unlistenHide = listen("live-subtitles-hide", () => setAnswers([]));
    return () => {
      unlistenAnswer.then((f) => f());
      unlistenHide.then((f) => f());
    };
  }, []);

  if (moving) {
    return (
      <div className="ans-root">
        <div className="ans-move" data-tauri-drag-region>
          <p className="ans-move-title">{t("liveOverlays.answersPanel")}</p>
          <p className="ans-move-hint">{t("liveOverlays.dragHint")}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="ans-root" aria-live="polite">
      {answers.map((answer, index) => (
        <article
          key={answer.id}
          className={`ans-card ${index === 0 ? "is-current" : "is-previous"}`}
        >
          <p className="ans-question">{answer.question}</p>
          {answer.answer ? (
            <p className="ans-text">
              {answer.answer}
              {!answer.done && <span className="ans-caret" aria-hidden />}
            </p>
          ) : (
            <p className="ans-thinking">
              <span />
              <span />
              <span />
              <span className="sr-only">{t("liveOverlays.thinking")}</span>
            </p>
          )}
        </article>
      ))}
    </div>
  );
};

export default AnswersOverlay;
