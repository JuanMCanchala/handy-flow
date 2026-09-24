import React, { useState } from "react";
import { Search, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands, type SearchResult } from "@/bindings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

/**
 * "Ask your history" search box on Home: plain text search runs a SQLite
 * FTS5 query over transcription history (see
 * `src-tauri/src/history_search.rs`); "Ask" sends the same top-k matches to
 * the configured LLM and shows the answer with links back to the source
 * entries.
 */
export const AskHistoryBox: React.FC = () => {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [searching, setSearching] = useState(false);
  const [asking, setAsking] = useState(false);
  const [results, setResults] = useState<SearchResult[] | null>(null);
  const [answer, setAnswer] = useState<string | null>(null);
  const [answerSources, setAnswerSources] = useState<SearchResult[]>([]);

  const handleSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    setAnswer(null);
    try {
      const result = await commands.searchHistory(query);
      if (result.status === "ok") {
        setResults(result.data);
      } else {
        toast.error(t("settings.home.ask.searchError"));
      }
    } finally {
      setSearching(false);
    }
  };

  const handleAsk = async () => {
    if (!query.trim()) return;
    setAsking(true);
    setAnswer(null);
    try {
      const result = await commands.askHistory(query);
      if (result.status === "ok") {
        setAnswer(result.data.answer);
        setAnswerSources(result.data.matches);
        setResults(null);
      } else {
        toast.error(t("settings.home.ask.askError"));
      }
    } finally {
      setAsking(false);
    }
  };

  const busy = searching || asking;

  return (
    <div className="bg-surface border border-border rounded-lg p-4 flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleSearch();
          }}
          placeholder={t("settings.home.ask.placeholder")}
          disabled={busy}
          className="flex-1"
        />
        <Button
          onClick={handleSearch}
          variant="secondary"
          size="md"
          disabled={busy || !query.trim()}
        >
          <Search width={16} height={16} />
          <span>{t("settings.home.ask.search")}</span>
        </Button>
        <Button onClick={handleAsk} size="md" disabled={busy || !query.trim()}>
          <Sparkles width={16} height={16} />
          <span>{t("settings.home.ask.ask")}</span>
        </Button>
      </div>

      {results && (
        <div className="flex flex-col gap-2">
          {results.length === 0 ? (
            <p className="text-small text-text-secondary">
              {t("settings.home.ask.noResults")}
            </p>
          ) : (
            results.map((r) => (
              <div
                key={r.id}
                className="text-small text-text border border-border rounded-md px-3 py-2"
              >
                <p className="font-medium">{r.title}</p>
                <p className="text-text-secondary">{r.snippet}</p>
              </div>
            ))
          )}
        </div>
      )}

      {answer && (
        <div className="flex flex-col gap-2">
          <p className="text-body text-text whitespace-pre-wrap">{answer}</p>
          {answerSources.length > 0 && (
            <div className="flex flex-col gap-1">
              <span className="text-caption uppercase text-text-tertiary tracking-wide">
                {t("settings.home.ask.sources")}
              </span>
              {answerSources.map((source) => (
                <div
                  key={source.id}
                  className="text-small text-text-secondary border border-border rounded-md px-3 py-2"
                >
                  <span className="font-medium text-text">{source.title}</span>
                  {": "}
                  {source.snippet}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
