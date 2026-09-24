import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  Check,
  Copy,
  Download,
  FolderOpen,
  Pencil,
  RotateCcw,
  Sparkles,
  Star,
  Trash2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { save } from "@tauri-apps/plugin-dialog";
import { commands, type HistoryEntry } from "@/bindings";
import { formatDateTime } from "@/utils/dateFormat";
import { AudioPlayer, AudioPlayerGroup } from "../../ui/AudioPlayer";
import { Button } from "../../ui/Button";
import { PageHeader } from "../../ui/PageHeader";
import { GenerateNotesDialog } from "../../notes/GenerateNotesDialog";
import { NotesView } from "../../notes/NotesView";
import { Textarea } from "../../ui/Textarea";
import { copyToClipboard } from "./clipboard";
import { useHistoryEntries } from "./useHistoryEntries";

type ExportFormat = "txt" | "srt" | "vtt";
const EXPORT_FORMATS: ExportFormat[] = ["txt", "srt", "vtt"];

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`size-7 rounded-md flex items-center justify-center transition-colors duration-[var(--dur-fast)] cursor-default disabled:pointer-events-none disabled:text-text-tertiary/40 ${
      active
        ? "text-brand hover:text-accent-hover"
        : "text-text-secondary hover:bg-fill-hover hover:text-text"
    }`}
    title={title}
  >
    {children}
  </button>
);

interface OpenRecordingsButtonProps {
  onClick: () => void;
  label: string;
}

const OpenRecordingsButton: React.FC<OpenRecordingsButtonProps> = ({
  onClick,
  label,
}) => (
  <Button
    onClick={onClick}
    variant="secondary"
    size="sm"
    className="flex items-center gap-2"
    title={label}
  >
    <FolderOpen className="w-4 h-4" />
    <span>{label}</span>
  </Button>
);

export const HistorySettings: React.FC = () => {
  const { t } = useTranslation();
  const {
    entries,
    loading,
    hasMore,
    entriesRef,
    loadPage,
    toggleSaved,
    getAudioUrl,
    deleteAudioEntry,
    retryHistoryEntry,
    editEntryText,
  } = useHistoryEntries();
  const sentinelRef = useRef<HTMLDivElement>(null);

  // Infinite scroll via IntersectionObserver
  useEffect(() => {
    if (loading) return;

    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;

    const observer = new IntersectionObserver(
      (observerEntries) => {
        const first = observerEntries[0];
        if (first.isIntersecting) {
          const lastEntry = entriesRef.current[entriesRef.current.length - 1];
          if (lastEntry) {
            loadPage(lastEntry.id);
          }
        }
      },
      { threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loading, hasMore, loadPage, entriesRef]);

  const openRecordingsFolder = async () => {
    try {
      const result = await commands.openRecordingsFolder();
      if (result.status !== "ok") {
        throw new Error(String(result.error));
      }
    } catch (error) {
      console.error("Failed to open recordings folder:", error);
    }
  };

  let content: React.ReactNode;

  if (loading) {
    content = (
      <div className="px-4 py-3 text-center text-text-secondary">
        {t("settings.history.loading")}
      </div>
    );
  } else if (entries.length === 0) {
    content = (
      <div className="px-4 py-3 text-center text-text-secondary">
        {t("settings.history.empty")}
      </div>
    );
  } else {
    content = (
      <>
        <AudioPlayerGroup>
          <div className="divide-y divide-border">
            {entries.map((entry) => (
              <HistoryEntryComponent
                key={entry.id}
                entry={entry}
                onToggleSaved={() => toggleSaved(entry.id)}
                onCopyText={() => copyToClipboard(entry.transcription_text)}
                getAudioUrl={getAudioUrl}
                deleteAudio={deleteAudioEntry}
                retryTranscription={retryHistoryEntry}
                onEditText={editEntryText}
              />
            ))}
          </div>
        </AudioPlayerGroup>
        {/* Sentinel for infinite scroll */}
        <div ref={sentinelRef} className="h-1" />
      </>
    );
  }

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        title={t("sidebar.history")}
        actions={
          <OpenRecordingsButton
            onClick={openRecordingsFolder}
            label={t("settings.history.openFolder")}
          />
        }
      />
      <div className="bg-surface border border-border rounded-lg overflow-visible">
        {content}
      </div>
    </div>
  );
};

export interface HistoryEntryProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => Promise<boolean>;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  retryTranscription: (id: number) => Promise<void>;
  onEditText: (id: number, text: string) => Promise<void>;
}

export const HistoryEntryComponent: React.FC<HistoryEntryProps> = ({
  entry,
  onToggleSaved,
  onCopyText,
  getAudioUrl,
  deleteAudio,
  retryTranscription,
  onEditText,
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [retrying, setRetrying] = useState(false);
  const [exportMenuOpen, setExportMenuOpen] = useState(false);
  const exportMenuRef = useRef<HTMLDivElement>(null);
  const [notesMarkdown, setNotesMarkdown] = useState(entry.notes_markdown);
  const [generateDialogOpen, setGenerateDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editedText, setEditedText] = useState(entry.transcription_text);
  const [saving, setSaving] = useState(false);

  const hasTranscription = entry.transcription_text.trim().length > 0;

  useEffect(() => {
    if (!exportMenuOpen) return;
    const handleClickOutside = (event: MouseEvent) => {
      if (
        exportMenuRef.current &&
        !exportMenuRef.current.contains(event.target as Node)
      ) {
        setExportMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [exportMenuOpen]);

  const handleExport = async (format: ExportFormat) => {
    setExportMenuOpen(false);
    const destPath = await save({
      defaultPath: `${entry.title}.${format}`,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    });
    if (!destPath) return;

    const result = await commands.exportTranscript(entry.id, format, destPath);
    if (result.status !== "ok") {
      toast.error(t("settings.history.exportError"));
    }
  };

  const handleLoadAudio = useCallback(
    () => getAudioUrl(entry.file_name),
    [getAudioUrl, entry.file_name],
  );

  const handleGenerateNotes = async (templateId: string) => {
    const result = await commands.generateNotes(entry.id, templateId);
    if (result.status === "ok") {
      setNotesMarkdown(result.data);
    } else {
      toast.error(t("notes.generateError"));
    }
  };

  const handleCopyText = async () => {
    if (!hasTranscription) {
      return;
    }

    const copied = await onCopyText();
    if (!copied) {
      toast.error(t("settings.history.copyError"));
      return;
    }

    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("settings.history.deleteError"));
    }
  };

  const handleRetranscribe = async () => {
    try {
      setRetrying(true);
      await retryTranscription(entry.id);
    } catch (error) {
      console.error("Failed to re-transcribe:", error);
      toast.error(t("settings.history.retranscribeError"));
    } finally {
      setRetrying(false);
    }
  };

  const handleStartEdit = () => {
    setEditedText(entry.transcription_text);
    setIsEditing(true);
  };

  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditedText(entry.transcription_text);
  };

  const handleSaveEdit = async () => {
    if (editedText === entry.transcription_text) {
      setIsEditing(false);
      return;
    }
    try {
      setSaving(true);
      await onEditText(entry.id, editedText);
      setIsEditing(false);
    } catch (error) {
      console.error("Failed to save edited transcription:", error);
      toast.error(t("settings.history.editError"));
    } finally {
      setSaving(false);
    }
  };

  const formattedDate = formatDateTime(String(entry.timestamp), i18n.language);

  return (
    <div className="px-4 py-2 pb-5 flex flex-col gap-3">
      <div className="flex justify-between items-center">
        <p className="text-body font-medium tabular">{formattedDate}</p>
        <div className="flex items-center">
          {!isEditing && (
            <IconButton
              onClick={handleStartEdit}
              disabled={!hasTranscription || retrying}
              title={t("settings.history.edit")}
            >
              <Pencil width={16} height={16} />
            </IconButton>
          )}
          <IconButton
            onClick={handleCopyText}
            disabled={!hasTranscription || retrying}
            title={t("settings.history.copyToClipboard")}
          >
            {showCopied ? (
              <Check width={16} height={16} />
            ) : (
              <Copy width={16} height={16} />
            )}
          </IconButton>
          <div className="relative" ref={exportMenuRef}>
            <IconButton
              onClick={() => setExportMenuOpen((open) => !open)}
              disabled={!hasTranscription || retrying}
              title={t("settings.history.export")}
            >
              <Download width={16} height={16} />
            </IconButton>
            {exportMenuOpen && (
              <div className="absolute right-0 top-full mt-1 z-10 bg-background border border-mid-gray/20 rounded-md shadow-lg overflow-hidden">
                {EXPORT_FORMATS.map((format) => (
                  <button
                    key={format}
                    onClick={() => handleExport(format)}
                    className="block w-full px-3 py-1.5 text-left text-xs text-text/80 hover:bg-mid-gray/10 cursor-pointer"
                  >
                    {format.toUpperCase()}
                  </button>
                ))}
              </div>
            )}
          </div>
          <IconButton
            onClick={() => setGenerateDialogOpen(true)}
            disabled={!hasTranscription || retrying}
            title={t("notes.generate")}
          >
            <Sparkles width={16} height={16} />
          </IconButton>
          <IconButton
            onClick={onToggleSaved}
            disabled={retrying}
            active={entry.saved}
            title={
              entry.saved
                ? t("settings.history.unsave")
                : t("settings.history.save")
            }
          >
            <Star
              width={16}
              height={16}
              fill={entry.saved ? "currentColor" : "none"}
            />
          </IconButton>
          <IconButton
            onClick={handleRetranscribe}
            disabled={retrying}
            title={t("settings.history.retranscribe")}
          >
            <RotateCcw
              width={16}
              height={16}
              style={
                retrying
                  ? { animation: "spin 1s linear infinite reverse" }
                  : undefined
              }
            />
          </IconButton>
          <IconButton
            onClick={handleDeleteEntry}
            disabled={retrying}
            title={t("settings.history.delete")}
          >
            <Trash2 width={16} height={16} />
          </IconButton>
        </div>
      </div>

      {isEditing ? (
        <div className="flex flex-col gap-2">
          <Textarea
            variant="compact"
            value={editedText}
            onChange={(e) => setEditedText(e.target.value)}
            disabled={saving}
            autoFocus
            className="w-full"
          />
          <div className="flex items-center gap-2 justify-end">
            <Button
              onClick={handleCancelEdit}
              variant="secondary"
              size="sm"
              disabled={saving}
            >
              <X width={14} height={14} />
              <span>{t("settings.history.cancelEdit")}</span>
            </Button>
            <Button onClick={handleSaveEdit} size="sm" disabled={saving}>
              <Check width={14} height={14} />
              <span>{t("settings.history.saveEdit")}</span>
            </Button>
          </div>
        </div>
      ) : (
        <p
          className={`italic text-small pb-2 ${
            retrying
              ? ""
              : hasTranscription
                ? "text-text select-text cursor-text whitespace-pre-wrap break-words"
                : "text-text-tertiary"
          }`}
          style={
            retrying
              ? { animation: "transcribe-pulse 3s ease-in-out infinite" }
              : undefined
          }
        >
          {retrying && (
            <style>{`
              @keyframes transcribe-pulse {
                0%, 100% { color: color-mix(in srgb, var(--color-text) 40%, transparent); }
                50% { color: color-mix(in srgb, var(--color-text) 90%, transparent); }
              }
            `}</style>
          )}
          {retrying
            ? t("settings.history.transcribing")
            : hasTranscription
              ? entry.transcription_text
              : t("settings.history.transcriptionFailed")}
        </p>
      )}

      <AudioPlayer onLoadRequest={handleLoadAudio} className="w-full" />

      {notesMarkdown && (
        <NotesView
          entryId={entry.id}
          entryTitle={entry.title}
          markdown={notesMarkdown}
          onMarkdownChange={setNotesMarkdown}
        />
      )}

      <GenerateNotesDialog
        open={generateDialogOpen}
        onOpenChange={setGenerateDialogOpen}
        onGenerate={handleGenerateNotes}
      />
    </div>
  );
};
