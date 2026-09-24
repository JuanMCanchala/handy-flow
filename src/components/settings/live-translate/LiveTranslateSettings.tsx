import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { commands, type LiveSubtitleLine } from "@/bindings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { PageHeader } from "../../ui/PageHeader";
import { Button } from "../../ui/Button";
import { ShortcutInput } from "../ShortcutInput";
import { LiveTranslateSourceSelector } from "../LiveTranslateSourceSelector";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { useSettings } from "../../../hooks/useSettings";

export const LiveTranslateSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [isActive, setIsActive] = useState(false);
  const [isToggling, setIsToggling] = useState(false);
  const [feed, setFeed] = useState<LiveSubtitleLine[]>([]);

  // Mirror of the overlay inside the app: every line as it is transcribed,
  // then updated in place when its translation arrives.
  useEffect(() => {
    const unlisten = listen<LiveSubtitleLine>("live-subtitle-line", (event) => {
      setFeed((prev) => {
        const index = prev.findIndex((line) => line.id === event.payload.id);
        if (index >= 0) {
          const next = [...prev];
          next[index] = event.payload;
          return next;
        }
        return [event.payload, ...prev].slice(0, 30);
      });
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    commands.isLiveTranslateActive().then((result) => {
      if (!cancelled && result.status === "ok") {
        setIsActive(result.data);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleToggle = async () => {
    setIsToggling(true);
    try {
      const result = await commands.toggleLiveTranslate();
      if (result.status === "error") {
        toast.error(result.error);
      } else {
        setIsActive((prev) => !prev);
      }
    } finally {
      setIsToggling(false);
    }
  };

  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={t("sidebar.liveTranslate")} />
      <SettingsGroup title={t("settings.liveTranslate.title")}>
        <ShortcutInput shortcutId="live_subtitles" grouped={true} />
        <LiveTranslateSourceSelector descriptionMode="tooltip" grouped={true} />
        <ToggleSwitch
          checked={getSetting("live_translate_suggest_answers") ?? false}
          onChange={(enabled) =>
            updateSetting("live_translate_suggest_answers", enabled)
          }
          isUpdating={isUpdating("live_translate_suggest_answers")}
          label={t("settings.liveTranslate.suggestAnswers.label")}
          description={t("settings.liveTranslate.suggestAnswers.description")}
          descriptionMode="tooltip"
          grouped={true}
        />
      </SettingsGroup>
      <div className="flex justify-center">
        <Button
          variant={isActive ? "danger" : "primary"}
          onClick={handleToggle}
          disabled={isToggling}
        >
          {isActive
            ? t("settings.liveTranslate.stopButton")
            : t("settings.liveTranslate.startButton")}
        </Button>
      </div>
      <SettingsGroup title={t("settings.liveTranslate.feed.title")}>
        <div className="flex flex-col gap-3 p-4 max-h-96 overflow-y-auto">
          {feed.length === 0 ? (
            <p className="text-small text-text-secondary">
              {isActive
                ? t("settings.liveTranslate.listening")
                : t("settings.liveTranslate.feed.empty")}
            </p>
          ) : (
            feed.map((line) => (
              <div key={line.id} className="flex flex-col gap-0.5">
                <span className="text-small text-text-secondary">
                  {line.original}
                </span>
                <span className="text-text font-medium">
                  {line.translation || "…"}
                </span>
              </div>
            ))
          )}
        </div>
      </SettingsGroup>
    </div>
  );
};
