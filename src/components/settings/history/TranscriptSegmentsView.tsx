import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Pencil } from "lucide-react";
import { commands, type TranscriptSegmentView } from "@/bindings";
import { Input } from "@/components/ui/Input";

interface TranscriptSegmentsViewProps {
  historyEntryId: number;
  fallbackText: string;
  hasTranscription: boolean;
}

/**
 * Renders a history entry's transcript. If any stored segment carries a
 * diarized speaker label, groups consecutive same-speaker segments under a
 * renameable speaker heading; otherwise falls back to the plain transcript
 * text (dictations, and imports without diarization).
 */
export const TranscriptSegmentsView: React.FC<TranscriptSegmentsViewProps> = ({
  historyEntryId,
  fallbackText,
  hasTranscription,
}) => {
  const { t } = useTranslation();
  const [segments, setSegments] = useState<TranscriptSegmentView[] | null>(
    null,
  );
  const [renamingLabel, setRenamingLabel] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  useEffect(() => {
    let cancelled = false;
    void commands.getTranscriptSegments(historyEntryId).then((result) => {
      if (!cancelled && result.status === "ok") {
        setSegments(result.data);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [historyEntryId]);

  const hasSpeakers = (segments ?? []).some((s) => s.speaker);

  if (!hasSpeakers) {
    return (
      <p
        className={`italic text-small pb-2 ${
          hasTranscription
            ? "text-text select-text cursor-text whitespace-pre-wrap break-words"
            : "text-text-tertiary"
        }`}
      >
        {hasTranscription
          ? fallbackText
          : t("settings.history.transcriptionFailed")}
      </p>
    );
  }

  // Group consecutive segments that share the same speaker label.
  const groups: { speaker: string | null; text: string }[] = [];
  for (const segment of segments ?? []) {
    const last = groups[groups.length - 1];
    if (last && last.speaker === segment.speaker) {
      last.text += ` ${segment.text}`;
    } else {
      groups.push({ speaker: segment.speaker, text: segment.text });
    }
  }

  const startRename = (speaker: string) => {
    setRenamingLabel(speaker);
    setRenameValue(speaker);
  };

  const commitRename = async (oldLabel: string) => {
    const newLabel = renameValue.trim();
    setRenamingLabel(null);
    if (!newLabel || newLabel === oldLabel) return;

    const result = await commands.renameSpeaker(
      historyEntryId,
      oldLabel,
      newLabel,
    );
    if (result.status === "ok") {
      setSegments(
        (segments ?? []).map((s) =>
          s.speaker === oldLabel ? { ...s, speaker: newLabel } : s,
        ),
      );
    }
  };

  return (
    <div className="flex flex-col gap-2 pb-2 select-text cursor-text">
      {groups.map((group, index) => (
        <div key={index} className="flex flex-col gap-0.5">
          {group.speaker && (
            <div className="flex items-center gap-1.5 group">
              {renamingLabel === group.speaker ? (
                <Input
                  autoFocus
                  variant="compact"
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  onBlur={() => commitRename(group.speaker as string)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      void commitRename(group.speaker as string);
                    } else if (e.key === "Escape") {
                      setRenamingLabel(null);
                    }
                  }}
                  className="w-40"
                />
              ) : (
                <>
                  <span className="text-caption font-semibold text-text-secondary">
                    {group.speaker}
                  </span>
                  <button
                    type="button"
                    onClick={() => startRename(group.speaker as string)}
                    title={t("settings.history.renameSpeaker")}
                    className="opacity-0 group-hover:opacity-100 text-text-tertiary hover:text-text transition-opacity"
                  >
                    <Pencil width={12} height={12} />
                  </button>
                </>
              )}
            </div>
          )}
          <p className="text-small text-text whitespace-pre-wrap break-words">
            {group.text}
          </p>
        </div>
      ))}
    </div>
  );
};
