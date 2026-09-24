import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import type { CopilotAnswerLine, LiveSubtitleLine } from "@/bindings";

export interface SuggestedAnswer extends CopilotAnswerLine {
  at: number;
}

interface LiveTranslateState {
  active: boolean;
  startedAt: number | null;
  lines: LiveSubtitleLine[];
  answers: SuggestedAnswer[];
  setActive: (active: boolean) => void;
  clear: () => void;
}

const MAX_LINES = 200;
const MAX_ANSWERS = 50;

export const useLiveTranslateStore = create<LiveTranslateState>()((set) => ({
  active: false,
  startedAt: null,
  lines: [],
  answers: [],
  setActive: (active) =>
    set((state) => ({
      active,
      startedAt: active ? (state.startedAt ?? Date.now()) : null,
    })),
  clear: () => set({ lines: [], answers: [] }),
}));

let initialized = false;

/**
 * Subscribes once, at app start, to the live session events so the feed keeps
 * filling while the user is on another section (or the view isn't mounted).
 */
export function initLiveTranslateStore() {
  if (initialized) return;
  initialized = true;

  listen<boolean>("live-translate-state", (event) => {
    const store = useLiveTranslateStore.getState();
    if (event.payload && !store.active) store.clear();
    store.setActive(event.payload);
  });

  // A line arrives twice: first with only the original, then again (same id)
  // once its translation is ready — update it in place.
  listen<LiveSubtitleLine>("live-subtitle-line", (event) => {
    useLiveTranslateStore.setState((state) => {
      const index = state.lines.findIndex((l) => l.id === event.payload.id);
      if (index >= 0) {
        const lines = [...state.lines];
        lines[index] = event.payload;
        return { lines };
      }
      return { lines: [...state.lines, event.payload].slice(-MAX_LINES) };
    });
  });

  // Answers stream in: same id = the same answer growing. A finished answer
  // with no text failed and is dropped.
  listen<CopilotAnswerLine>("copilot-answer-line", (event) => {
    const line = event.payload;
    useLiveTranslateStore.setState((state) => {
      const index = state.answers.findIndex((a) => a.id === line.id);
      if (line.done && !line.answer) {
        return { answers: state.answers.filter((a) => a.id !== line.id) };
      }
      if (index >= 0) {
        const answers = [...state.answers];
        answers[index] = { ...answers[index], ...line };
        return { answers };
      }
      return {
        answers: [{ ...line, at: Date.now() }, ...state.answers].slice(
          0,
          MAX_ANSWERS,
        ),
      };
    });
  });
}
