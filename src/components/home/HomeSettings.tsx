import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type HistoryEntry, type Insights } from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { HistoryEntryComponent } from "../settings/history/HistorySettings";
import { useHistoryEntries } from "../settings/history/useHistoryEntries";
import { AudioPlayerGroup } from "../ui/AudioPlayer";
import { FileImportDropZone } from "./FileImportDropZone";

const StatCard: React.FC<{ label: string; value: string }> = ({
  label,
  value,
}) => (
  <div className="flex-1 bg-background border border-mid-gray/20 rounded-lg px-4 py-3 flex flex-col gap-1">
    <span className="text-2xl font-semibold">{value}</span>
    <span className="text-xs text-mid-gray uppercase tracking-wide">
      {label}
    </span>
  </div>
);

const StatsCard: React.FC = () => {
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
    <div className="flex gap-3">
      <StatCard
        label={t("settings.home.stats.totalWords")}
        value={insights ? insights.total_words.toLocaleString() : "—"}
      />
      <StatCard
        label={t("settings.home.stats.averageWpm")}
        value={insights ? Math.round(insights.average_wpm).toString() : "—"}
      />
      <StatCard
        label={t("settings.home.stats.dayStreak")}
        value={insights ? insights.day_streak.toString() : "—"}
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
  } = useHistoryEntries();
  const dayGroupLabel = useDayGroupLabel();

  const groups = groupByDay(entries);

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <h1 className="text-xl font-semibold">{t("settings.home.welcome")}</h1>

      <StatsCard />

      <FileImportDropZone />

      <div className="space-y-4">
        {loading ? (
          <div className="px-4 py-3 text-center text-text/60">
            {t("settings.history.loading")}
          </div>
        ) : entries.length === 0 ? (
          <div className="px-4 py-3 text-center text-text/60">
            {t("settings.history.empty")}
          </div>
        ) : (
          <AudioPlayerGroup>
            {groups.map(([key, dayEntries]) => (
              <div key={key} className="space-y-2">
                <h2 className="px-4 text-xs font-medium text-mid-gray uppercase tracking-wide">
                  {dayGroupLabel(dayEntries[0].timestamp)}
                </h2>
                <div className="bg-background border border-mid-gray/20 rounded-lg overflow-visible divide-y divide-mid-gray/20">
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
