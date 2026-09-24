import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Copy } from "lucide-react";
import { commands, type LiveSubtitleLine } from "@/bindings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { PageHeader } from "../../ui/PageHeader";
import { Button } from "../../ui/Button";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { ShortcutInput } from "../ShortcutInput";
import { LiveTranslateSourceSelector } from "../LiveTranslateSourceSelector";
import { useSettings } from "../../../hooks/useSettings";
import {
  useLiveTranslateStore,
  type SuggestedAnswer,
} from "../../../stores/liveTranslateStore";
import "./LiveTranslate.css";

type View = "conversation" | "answers";

const formatElapsed = (ms: number) => {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
};

const useElapsed = (startedAt: number | null) => {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (startedAt === null) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [startedAt]);
  return startedAt === null ? 0 : now - startedAt;
};

interface ColumnProps {
  from: "en" | "es";
  lines: LiveSubtitleLine[];
  emptyText: string;
}

/** One translation direction; newest line at the bottom, auto-scrolled. */
const TranslationColumn: React.FC<ColumnProps> = ({
  from,
  lines,
  emptyText,
}) => {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const pinnedRef = useRef(true);

  useLayoutEffect(() => {
    const el = scrollRef.current;
    if (el && pinnedRef.current) el.scrollTop = el.scrollHeight;
  }, [lines]);

  const to = from === "en" ? "es" : "en";

  return (
    <section className="lt-column" aria-label={`${from} → ${to}`}>
      <header className="lt-column-head">
        <span className="lt-direction">
          {from}
          <span aria-hidden="true" className="lt-arrow">
            →
          </span>
          {to}
        </span>
      </header>
      <div
        ref={scrollRef}
        className="lt-column-body"
        onScroll={() => {
          const el = scrollRef.current;
          if (el)
            pinnedRef.current =
              el.scrollHeight - el.scrollTop - el.clientHeight < 24;
        }}
      >
        {lines.length === 0 ? (
          <p className="lt-empty">{emptyText}</p>
        ) : (
          lines.map((line) => (
            <article key={line.id} className="lt-line">
              <p className="lt-original">{line.original}</p>
              {line.translation ? (
                <p className="lt-translation">{line.translation}</p>
              ) : (
                <p className="lt-translation lt-pending">
                  {t("settings.liveTranslate.view.translating")}
                </p>
              )}
            </article>
          ))
        )}
      </div>
    </section>
  );
};

const AnswerCard: React.FC<{ answer: SuggestedAnswer }> = ({ answer }) => {
  const { t } = useTranslation();
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(answer.answer);
      toast.success(t("settings.liveTranslate.view.copied"));
    } catch {
      toast.error(t("settings.liveTranslate.view.copyFailed"));
    }
  };
  return (
    <article className="lt-answer">
      <p className="lt-answer-question">{answer.question}</p>
      <p className="lt-answer-text">{answer.answer}</p>
      <div className="lt-answer-foot">
        <time className="lt-answer-time">
          {new Date(answer.at).toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          })}
        </time>
        <button type="button" className="lt-copy" onClick={copy}>
          <Copy width={14} height={14} aria-hidden="true" />
          {t("settings.liveTranslate.view.copy")}
        </button>
      </div>
    </article>
  );
};

export const LiveTranslateSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const { active, startedAt, lines, answers, setActive } =
    useLiveTranslateStore();
  const [isToggling, setIsToggling] = useState(false);
  const [view, setView] = useState<View>("conversation");
  const elapsed = useElapsed(startedAt);
  const suggestAnswers = getSetting("live_translate_suggest_answers") ?? false;

  useEffect(() => {
    commands.isLiveTranslateActive().then((result) => {
      if (result.status === "ok") setActive(result.data);
    });
  }, [setActive]);

  const handleToggle = async () => {
    setIsToggling(true);
    try {
      const result = await commands.toggleLiveTranslate();
      if (result.status === "error") toast.error(result.error);
    } finally {
      setIsToggling(false);
    }
  };

  const enLines = lines.filter((line) => line.source_lang !== "es");
  const esLines = lines.filter((line) => line.source_lang === "es");

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t("sidebar.liveTranslate")}
        actions={
          <div className="flex items-center gap-3">
            <span
              className={`lt-status ${active ? "is-live" : ""}`}
              aria-live="polite"
            >
              <span className="lt-dot" aria-hidden="true" />
              {active
                ? formatElapsed(elapsed)
                : t("settings.liveTranslate.view.idle")}
            </span>
            <Button
              variant={active ? "danger" : "primary"}
              onClick={handleToggle}
              disabled={isToggling}
            >
              {active
                ? t("settings.liveTranslate.stopButton")
                : t("settings.liveTranslate.startButton")}
            </Button>
          </div>
        }
      />

      <div
        className="lt-tabs"
        role="tablist"
        aria-label={t("sidebar.liveTranslate")}
      >
        <button
          role="tab"
          aria-selected={view === "conversation"}
          className="lt-tab"
          onClick={() => setView("conversation")}
        >
          {t("settings.liveTranslate.view.conversation")}
        </button>
        <button
          role="tab"
          aria-selected={view === "answers"}
          className="lt-tab"
          onClick={() => setView("answers")}
        >
          {t("settings.liveTranslate.view.answers")}
          {answers.length > 0 && (
            <span className="lt-count">{answers.length}</span>
          )}
        </button>
      </div>

      {view === "conversation" ? (
        <div className="lt-columns">
          <TranslationColumn
            from="en"
            lines={enLines}
            emptyText={t("settings.liveTranslate.view.emptyEn")}
          />
          <TranslationColumn
            from="es"
            lines={esLines}
            emptyText={t("settings.liveTranslate.view.emptyEs")}
          />
        </div>
      ) : (
        <div className="lt-answers">
          {answers.length === 0 ? (
            <div className="lt-answers-empty">
              <p>
                {suggestAnswers
                  ? t("settings.liveTranslate.view.answersEmptyOn")
                  : t("settings.liveTranslate.view.answersEmptyOff")}
              </p>
              {!suggestAnswers && (
                <Button
                  variant="secondary"
                  onClick={() =>
                    updateSetting("live_translate_suggest_answers", true)
                  }
                >
                  {t("settings.liveTranslate.view.enableAnswers")}
                </Button>
              )}
            </div>
          ) : (
            answers.map((answer) => (
              <AnswerCard key={answer.id} answer={answer} />
            ))
          )}
        </div>
      )}

      <SettingsGroup title={t("settings.liveTranslate.title")}>
        <ShortcutInput shortcutId="live_subtitles" grouped={true} />
        <LiveTranslateSourceSelector descriptionMode="tooltip" grouped={true} />
        <ToggleSwitch
          checked={suggestAnswers}
          onChange={(enabled) =>
            updateSetting("live_translate_suggest_answers", enabled)
          }
          isUpdating={isUpdating("live_translate_suggest_answers")}
          label={t("settings.liveTranslate.suggestAnswers.label")}
          description={t("settings.liveTranslate.suggestAnswers.description")}
          descriptionMode="tooltip"
          grouped={true}
        />
        <ToggleSwitch
          checked={getSetting("hide_from_screen_share") ?? true}
          onChange={(enabled) =>
            updateSetting("hide_from_screen_share", enabled)
          }
          isUpdating={isUpdating("hide_from_screen_share")}
          label={t("settings.liveTranslate.hideFromShare.label")}
          description={t("settings.liveTranslate.hideFromShare.description")}
          descriptionMode="tooltip"
          grouped={true}
        />
      </SettingsGroup>
    </div>
  );
};
