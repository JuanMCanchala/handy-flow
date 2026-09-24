import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { SettingsGroup } from "../ui/SettingsGroup";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

const CATEGORIES = ["personal", "work", "email", "other"] as const;
const TONES = ["formal", "casual", "very_casual", "excited"] as const;

const StyleCategoryPicker: React.FC<{ category: (typeof CATEGORIES)[number] }> = ({
  category,
}) => {
  const { t } = useTranslation();
  const { getSetting, setAppStyle, isUpdating } = useSettings();

  const appStyles = getSetting("app_styles") || {};
  const selectedTone = appStyles[category] ?? null;

  const toneOptions = TONES.map((tone) => ({
    value: tone,
    label: t(`settings.style.tones.${tone}`),
  }));

  return (
    <SettingContainer
      title={t(`settings.style.categories.${category}`)}
      description={t(`settings.style.categoryDescriptions.${category}`)}
      descriptionMode="tooltip"
      layout="horizontal"
      grouped={true}
    >
      <Dropdown
        options={toneOptions}
        selectedValue={selectedTone}
        onSelect={(tone) => setAppStyle(category, tone)}
        placeholder={t("settings.style.selectTone")}
        disabled={isUpdating(`app_style:${category}`)}
      />
    </SettingContainer>
  );
};

export const StyleSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const enabled = getSetting("style_per_app_enabled") || false;

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.style.groups.general")}>
        <ToggleSwitch
          checked={enabled}
          onChange={(value) => updateSetting("style_per_app_enabled", value)}
          isUpdating={isUpdating("style_per_app_enabled")}
          label={t("settings.style.enabled.label")}
          description={t("settings.style.enabled.description")}
          descriptionMode="tooltip"
          grouped={true}
        />
      </SettingsGroup>

      {enabled && (
        <SettingsGroup title={t("settings.style.groups.categories")}>
          {CATEGORIES.map((category) => (
            <StyleCategoryPicker key={category} category={category} />
          ))}
        </SettingsGroup>
      )}
    </div>
  );
};
