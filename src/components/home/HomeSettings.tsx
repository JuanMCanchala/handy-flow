import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type HistoryEntry, type Insights } from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { HistoryEntryComponent } from "../settings/history/HistorySettings";
import { useHistoryEntries } from "../settings/history/useHistoryEntries";
import { AudioPlayerGroup } from "../ui/AudioPlayer";
import { PageHeader } from "../ui/PageHeader";
import { StatCard } from "../ui/StatCard";
import { AskHistoryBox } from "./AskHistoryBox";
import { FileImportDropZone } from "./FileImportDropZone";
import { LearningSuggestions } from "./LearningSuggestions";

const StatsRow: React.FC = () => {
  const { t } = useTranslation();
  const [insights, setInsights] = useState<Insights | null>(null);

  useEffect(() => {
    let mounted = true;
    commands.getInsights().then((result) => {
      if (mounted && result.status === "ok") {
        setInsights(result.data);
      }
    });
    return () => {
      mounted = false;
    };
  }, []);

  return (
    <div className="grid grid-cols-[repeat(auto-fit,minmax(160px,1fr))] gap-3">
      <StatCard
        label={t("settings.home.stats.totalWords")}
        value={insights ? insights.total_words : "—"}
      />
      <StatCard
        label={t("settings.home.stats.averageWpm")}
        value={insights ? Math.round(insights.average_wpm) : "—"}
      />
      <StatCard
        label={t("settings.home.stats.dayStreak")}
        value={insights ? insights.day_streak : "—"}
      />
    </div>
  );
};

/** Local calendar-day key ("YYYY-MM-DD") for an entry's timestamp (seconds). */
const dayKey = (timestampSeconds: number): string => {
  const date = new Date(timestampSeconds * 1000);
  return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
};

const groupByDay = (entries: HistoryEntry[]): [string, HistoryEntry[]][] => {
  const groups = new Map<string, HistoryEntry[]>();
  for (const entry of entries) {
    const key = dayKey(entry.timestamp);
    const existing = groups.get(key);
    if (existing) {
      existing.push(entry);
    } else {
      groups.set(key, [entry]);
    }
  }
  return Array.from(groups.entries());
};

const useDayGroupLabel = () => {
  const { t, i18n } = useTranslation();

  return (timestampSeconds: number): string => {
    const date = new Date(timestampSeconds * 1000);
    const now = new Date();
    const yesterday = new Date(now);
    yesterday.setDate(now.getDate() - 1);

    const sameDay = (a: Date, b: Date) =>
      a.getFullYear() === b.getFullYear() &&
      a.getMonth() === b.getMonth() &&
      a.getDate() === b.getDate();

    if (sameDay(date, now)) return t("settings.home.today");
    if (sameDay(date, yesterday)) return t("settings.home.yesterday");

    return new Intl.DateTimeFormat(i18n.language, {
      year: "numeric",
      month: "long",
      day: "numeric",
    }).format(date);
  };
};

export const HomeSettings: React.FC = () => {
  const { t } = useTranslation();
  const {
    entries,
    loading,
    toggleSaved,
    getAudioUrl,
    deleteAudioEntry,
    retryHistoryEntry,
    editEntryText,
  } = useHistoryEntries();
  const dayGroupLabel = useDayGroupLabel();

  const groups = groupByDay(entries);

  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={t("settings.home.welcome")} />

      <StatsRow />

      <AskHistoryBox />

      <LearningSuggestions />

      <FileImportDropZone />

      <div className="space-y-4">
        {loading ? (
          <div className="px-4 py-3 text-center text-text-secondary">
            {t("settings.history.loading")}
          </div>
        ) : entries.length === 0 ? (
          <div className="px-4 py-3 text-center text-text-secondary">
            {t("settings.history.empty")}
          </div>
        ) : (
          <AudioPlayerGroup>
            {groups.map(([key, dayEntries]) => (
              <div key={key} className="space-y-2">
                <h2 className="px-1 text-overline uppercase text-text-tertiary tracking-wide">
                  {dayGroupLabel(dayEntries[0].timestamp)}
                </h2>
                <div className="bg-surface border border-border rounded-lg overflow-visible divide-y divide-border">
                  {dayEntries.map((entry) => (
                    <HistoryEntryComponent
                      key={entry.id}
                      entry={entry}
                      onToggleSaved={() => toggleSaved(entry.id)}
                      onCopyText={() =>
                        copyToClipboard(entry.transcription_text)
                      }
                      getAudioUrl={getAudioUrl}
                      deleteAudio={deleteAudioEntry}
                      retryTranscription={retryHistoryEntry}
                      onEditText={editEntryText}
                    />
                  ))}
                </div>
              </div>
            ))}
          </AudioPlayerGroup>
        )}
      </div>
    </div>
  );
};
