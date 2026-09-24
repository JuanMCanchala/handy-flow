import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";
import type { FlowBarVisibility as FlowBarVisibilityValue } from "@/bindings";

interface FlowBarVisibilityProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const FlowBarVisibility: React.FC<FlowBarVisibilityProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const options = [
      {
        value: "text_fields",
        label: t("settings.general.flowBar.options.textFields"),
      },
      {
        value: "always",
        label: t("settings.general.flowBar.options.always"),
      },
      {
        value: "never",
        label: t("settings.general.flowBar.options.never"),
      },
    ];

    const selected = (getSetting("flow_bar_visibility") ||
      "text_fields") as FlowBarVisibilityValue;

    return (
      <SettingContainer
        title={t("settings.general.flowBar.title")}
        description={t("settings.general.flowBar.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Dropdown
          options={options}
          selectedValue={selected}
          onSelect={(value) =>
            updateSetting(
              "flow_bar_visibility",
              value as FlowBarVisibilityValue,
            )
          }
          disabled={isUpdating("flow_bar_visibility")}
        />
      </SettingContainer>
    );
  },
);
