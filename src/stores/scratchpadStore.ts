import { create } from "zustand";
import { Store, load } from "@tauri-apps/plugin-store";

export interface ScratchpadNote {
  id: string;
  content: string;
  createdAt: number;
  updatedAt: number;
}

const STORE_FILE = "scratchpad.json";
const NOTES_KEY = "notes";

let storePromise: Promise<Store> | null = null;

const getStore = (): Promise<Store> => {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { defaults: {}, autoSave: false });
  }
  return storePromise;
};

const titleFromContent = (content: string): string => {
  const firstLine = content.split("\n", 1)[0]?.trim() ?? "";
  return firstLine;
};

interface ScratchpadStore {
  notes: ScratchpadNote[];
  isLoading: boolean;

  initialize: () => Promise<void>;
  createNote: () => Promise<ScratchpadNote>;
  updateNoteContent: (id: string, content: string) => Promise<void>;
  deleteNote: (id: string) => Promise<void>;
}

export const useScratchpadStore = create<ScratchpadStore>((set, get) => ({
  notes: [],
  isLoading: true,

  initialize: async () => {
    try {
      const store = await getStore();
      const notes = (await store.get<ScratchpadNote[]>(NOTES_KEY)) ?? [];
      set({
        notes: [...notes].sort((a, b) => b.updatedAt - a.updatedAt),
        isLoading: false,
      });
    } catch (error) {
      console.error("Failed to load scratchpad notes:", error);
      set({ isLoading: false });
    }
  },

  createNote: async () => {
    const now = Date.now();
    const note: ScratchpadNote = {
      id: crypto.randomUUID(),
      content: "",
      createdAt: now,
      updatedAt: now,
    };
    const notes = [note, ...get().notes];
    set({ notes });
    try {
      const store = await getStore();
      await store.set(NOTES_KEY, notes);
      await store.save();
    } catch (error) {
      console.error("Failed to save new scratchpad note:", error);
    }
    return note;
  },

  updateNoteContent: async (id, content) => {
    const now = Date.now();
    const notes = get()
      .notes.map((note) =>
        note.id === id ? { ...note, content, updatedAt: now } : note,
      )
      .sort((a, b) => b.updatedAt - a.updatedAt);
    set({ notes });
    try {
      const store = await getStore();
      await store.set(NOTES_KEY, notes);
      await store.save();
    } catch (error) {
      console.error("Failed to save scratchpad note:", error);
    }
  },

  deleteNote: async (id) => {
    const notes = get().notes.filter((note) => note.id !== id);
    set({ notes });
    try {
      const store = await getStore();
      await store.set(NOTES_KEY, notes);
      await store.save();
    } catch (error) {
      console.error("Failed to delete scratchpad note:", error);
    }
  },
}));

export const getScratchpadNoteTitle = titleFromContent;
