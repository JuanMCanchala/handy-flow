import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown, SettingContainer } from "@/components/ui";
import { useSettings } from "../../hooks/useSettings";
import type { TranslationTarget } from "@/bindings";

interface TranslationTargetSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const TranslationTargetSelector: React.FC<
  TranslationTargetSelectorProps
> = ({ descriptionMode = "tooltip", grouped = false }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const value = getSetting("translation_target") ?? "auto";

  const options = [
    {
      value: "auto",
      label: t("settings.postProcessing.translate.target.auto"),
    },
    {
      value: "en",
      label: t("settings.postProcessing.translate.target.en"),
    },
    {
      value: "es",
      label: t("settings.postProcessing.translate.target.es"),
    },
  ];

  return (
    <SettingContainer
      title={t("settings.postProcessing.translate.target.title")}
      description={t("settings.postProcessing.translate.target.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    >
      <Dropdown
        selectedValue={value}
        options={options}
        onSelect={(v) => updateSetting("translation_target", v as TranslationTarget)}
        disabled={isUpdating("translation_target")}
      />
    </SettingContainer>
  );
};
