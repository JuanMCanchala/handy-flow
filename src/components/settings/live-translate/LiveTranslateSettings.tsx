import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { PageHeader } from "../../ui/PageHeader";
import { Button } from "../../ui/Button";
import { ShortcutInput } from "../ShortcutInput";
import { LiveTranslateSourceSelector } from "../LiveTranslateSourceSelector";

export const LiveTranslateSettings: React.FC = () => {
  const { t } = useTranslation();
  const [isActive, setIsActive] = useState(false);
  const [isToggling, setIsToggling] = useState(false);

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
    </div>
  );
};
