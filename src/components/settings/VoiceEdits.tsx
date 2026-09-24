import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface VoiceEditsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const VoiceEdits: React.FC<VoiceEditsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const voiceEditsEnabled = getSetting("voice_edits_enabled") ?? true;

    return (
      <ToggleSwitch
        checked={voiceEditsEnabled}
        onChange={(enabled) => updateSetting("voice_edits_enabled", enabled)}
        isUpdating={isUpdating("voice_edits_enabled")}
        label={t("settings.general.voiceEdits.label")}
        description={t("settings.general.voiceEdits.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);
