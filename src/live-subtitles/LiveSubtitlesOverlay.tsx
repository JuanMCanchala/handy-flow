import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useState } from "react";
import "./LiveSubtitlesOverlay.css";

interface LiveSubtitleLine {
  original: string;
  translation: string;
}

interface CopilotAnswerLine {
  question: string;
  answer: string;
}

// Shows the last two translated subtitle lines, and/or (when the copilot is
// running instead) the last two answer suggestions, newest first. Purely a
// passive display: the window itself is transparent, always-on-top,
// click-through, and excluded from screen capture where the platform
// supports it (see live_translate/overlay.rs). It never steals focus.
const LiveSubtitlesOverlay: React.FC = () => {
  const [lines, setLines] = useState<LiveSubtitleLine[]>([]);
  const [answers, setAnswers] = useState<CopilotAnswerLine[]>([]);

  useEffect(() => {
    const unlistenLine = listen<LiveSubtitleLine>("live-subtitle-line", (event) => {
      setLines((prev) => [...prev, event.payload].slice(-2));
    });
    const unlistenAnswer = listen<CopilotAnswerLine>("copilot-answer-line", (event) => {
      // Newest first: prepend, then keep only the two most recent.
      setAnswers((prev) => [event.payload, ...prev].slice(0, 2));
    });
    const unlistenHide = listen("live-subtitles-hide", () => {
      setLines([]);
      setAnswers([]);
    });

    return () => {
      unlistenLine.then((f) => f());
      unlistenAnswer.then((f) => f());
      unlistenHide.then((f) => f());
    };
  }, []);

  if (lines.length === 0 && answers.length === 0) {
    return null;
  }

  return (
    <div className="live-subtitles-container">
      {answers.map((answer, index) => (
        <div className="copilot-answer-card" key={index}>
          <div className="copilot-answer-question">{answer.question}</div>
          <div className="copilot-answer-text">{answer.answer}</div>
        </div>
      ))}
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
