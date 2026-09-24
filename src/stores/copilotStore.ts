import { create } from "zustand";
import { commands } from "@/bindings";
import type { CopilotAnswerEntry, CopilotAnswerLanguage } from "@/bindings";

interface CopilotStore {
  text: string;
  answerLanguage: CopilotAnswerLanguage;
  history: CopilotAnswerEntry[];
  isLoading: boolean;
  isActive: boolean;

  initialize: () => Promise<void>;
  setText: (text: string) => Promise<void>;
  setAnswerLanguage: (language: CopilotAnswerLanguage) => Promise<void>;
  importFile: (path: string) => Promise<string>;
  refreshHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  refreshActive: () => Promise<void>;
  toggle: () => Promise<void>;
}

export const useCopilotStore = create<CopilotStore>((set, get) => ({
  text: "",
  answerLanguage: "auto",
  history: [],
  isLoading: true,
  isActive: false,

  initialize: async () => {
    try {
      const [profileResult, historyResult, activeResult] = await Promise.all([
        commands.getCopilotProfile(),
        commands.getCopilotHistory(),
        commands.isCopilotActive(),
      ]);

      set({
        text: profileResult.status === "ok" ? profileResult.data.text : "",
        answerLanguage:
          profileResult.status === "ok"
            ? profileResult.data.answer_language
            : "auto",
        history: historyResult.status === "ok" ? historyResult.data : [],
        isActive: activeResult.status === "ok" ? activeResult.data : false,
        isLoading: false,
      });
    } catch (error) {
      console.error("Failed to load copilot profile:", error);
      set({ isLoading: false });
    }
  },

  setText: async (text) => {
    set({ text });
    try {
      await commands.setCopilotProfileText(text);
    } catch (error) {
      console.error("Failed to save copilot profile text:", error);
    }
  },

  setAnswerLanguage: async (language) => {
    set({ answerLanguage: language });
    try {
      await commands.setCopilotAnswerLanguage(language);
    } catch (error) {
      console.error("Failed to save copilot answer language:", error);
    }
  },

  importFile: async (path) => {
    const result = await commands.importCopilotProfileFile(path);
    if (result.status === "error") {
      throw new Error(result.error);
    }
    return result.data;
  },

  refreshHistory: async () => {
    try {
      const result = await commands.getCopilotHistory();
      if (result.status === "ok") {
        set({ history: result.data });
      }
    } catch (error) {
      console.error("Failed to load copilot history:", error);
    }
  },

  clearHistory: async () => {
    try {
      await commands.clearCopilotHistory();
      set({ history: [] });
    } catch (error) {
      console.error("Failed to clear copilot history:", error);
    }
  },

  refreshActive: async () => {
    try {
      const result = await commands.isCopilotActive();
      if (result.status === "ok") {
        set({ isActive: result.data });
      }
    } catch (error) {
      console.error("Failed to check copilot active state:", error);
    }
  },

  toggle: async () => {
    const wasActive = get().isActive;
    set({ isActive: !wasActive });
    try {
      const result = await commands.toggleCopilot();
      if (result.status === "error") {
        set({ isActive: wasActive });
        throw new Error(result.error);
      }
    } catch (error) {
      set({ isActive: wasActive });
      throw error;
    }
  },
}));
