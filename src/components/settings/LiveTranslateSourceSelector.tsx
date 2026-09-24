import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { LiveTranslateSource } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { Dropdown, type DropdownOption } from "@/components/ui/Dropdown";
import { SettingContainer } from "@/components/ui/SettingContainer";

interface LiveTranslateSourceSelectorProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const LiveTranslateSourceSelector: React.FC<
  LiveTranslateSourceSelectorProps
> = ({ descriptionMode = "tooltip", grouped = false }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const selectedSource = getSetting("live_translate_source") ?? "microphone";

  const options = useMemo<DropdownOption[]>(
    () => [
      {
        value: "microphone",
        label: t("settings.liveTranslate.source.options.microphone"),
      },
      {
        value: "system_audio",
        label: t("settings.liveTranslate.source.options.systemAudio"),
      },
    ],
    [t],
  );

  return (
    <SettingContainer
      title={t("settings.liveTranslate.source.title")}
      description={t("settings.liveTranslate.source.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="horizontal"
    >
      <Dropdown
        options={options}
        selectedValue={selectedSource}
        onSelect={(value) =>
          updateSetting("live_translate_source", value as LiveTranslateSource)
        }
        disabled={isUpdating("live_translate_source")}
      />
    </SettingContainer>
  );
};
