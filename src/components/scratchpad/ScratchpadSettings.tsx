import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { Plus, Trash2 } from "lucide-react";
import {
  getScratchpadNoteTitle,
  useScratchpadStore,
  type ScratchpadNote,
} from "@/stores/scratchpadStore";
import { formatDateTime } from "@/utils/dateFormat";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { PageHeader } from "../ui/PageHeader";
import { Textarea } from "../ui/Textarea";

export const ScratchpadSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { notes, isLoading, initialize, createNote, updateNoteContent, deleteNote } =
    useScratchpadStore();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [draft, setDraft] = useState("");

  useEffect(() => {
    initialize();
  }, [initialize]);

  useEffect(() => {
    if (!selectedId && notes.length > 0) {
      setSelectedId(notes[0].id);
    }
  }, [notes, selectedId]);

  const selectedNote = useMemo(
    () => notes.find((note) => note.id === selectedId) ?? null,
    [notes, selectedId],
  );

  useEffect(() => {
    setDraft(selectedNote?.content ?? "");
  }, [selectedNote?.id, selectedNote?.content]);

  const filteredNotes = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) return notes;
    return notes.filter((note) => note.content.toLowerCase().includes(query));
  }, [notes, searchQuery]);

  const handleCreateNote = async () => {
    const note = await createNote();
    setSelectedId(note.id);
  };

  const handleContentChange = (value: string) => {
    setDraft(value);
    if (selectedNote) {
      updateNoteContent(selectedNote.id, value);
    }
  };

  const handleDeleteNote = async (note: ScratchpadNote) => {
    const confirmed = await ask(t("settings.scratchpad.deleteConfirm"), {
      title: t("settings.scratchpad.deleteTitle"),
      kind: "warning",
    });
    if (!confirmed) return;

    await deleteNote(note.id);
    if (selectedId === note.id) {
      setSelectedId(null);
    }
  };

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t("sidebar.scratchpad")}
        actions={
          <Button
            onClick={handleCreateNote}
            variant="secondary"
            size="sm"
            className="flex items-center gap-2"
            title={t("settings.scratchpad.newNote")}
          >
            <Plus className="w-4 h-4" />
            <span>{t("settings.scratchpad.newNote")}</span>
          </Button>
        }
      />

      <div className="bg-surface border border-border rounded-lg overflow-hidden flex h-[480px]">
        <div className="w-1/3 border-e border-border flex flex-col min-h-0">
          <div className="p-2 border-b border-border">
            <Input
              variant="compact"
              className="w-full"
              placeholder={t("settings.scratchpad.searchPlaceholder")}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
          <div className="flex-1 overflow-y-auto divide-y divide-border">
            {isLoading ? (
              <div className="px-4 py-3 text-center text-text-secondary text-small">
                {t("settings.scratchpad.loading")}
              </div>
            ) : filteredNotes.length === 0 ? (
              <div className="px-4 py-3 text-center text-text-secondary text-small">
                {t("settings.scratchpad.empty")}
              </div>
            ) : (
              filteredNotes.map((note) => {
                const title =
                  getScratchpadNoteTitle(note.content) ||
                  t("settings.scratchpad.untitled");
                const isActive = note.id === selectedId;
                return (
                  <div
                    key={note.id}
                    onClick={() => setSelectedId(note.id)}
                    className={`px-3 py-2 cursor-default flex items-center justify-between gap-2 group ${
                      isActive ? "bg-fill-selected" : "hover:bg-fill-hover"
                    }`}
                  >
                    <div className="min-w-0">
                      <p className="text-body font-medium truncate">{title}</p>
                      <p className="text-caption text-text-tertiary">
                        {formatDateTime(
                          String(Math.floor(note.updatedAt / 1000)),
                          i18n.language,
                        )}
                      </p>
                    </div>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteNote(note);
                      }}
                      title={t("settings.scratchpad.delete")}
                      className="p-1 rounded-md text-text-tertiary hover:text-error opacity-0 group-hover:opacity-100 transition-opacity shrink-0 cursor-default"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                );
              })
            )}
          </div>
        </div>

        <div className="flex-1 flex flex-col min-h-0">
          {selectedNote ? (
            <Textarea
              key={selectedNote.id}
              className="flex-1 w-full h-full resize-none rounded-none focus:bg-transparent hover:bg-transparent"
              value={draft}
              onChange={(e) => handleContentChange(e.target.value)}
              placeholder={t("settings.scratchpad.editorPlaceholder")}
              autoFocus
            />
          ) : (
            <div className="flex-1 flex items-center justify-center text-text-secondary text-small">
              {t("settings.scratchpad.noNoteSelected")}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
