import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";

interface AgentNameProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const AgentName: React.FC<AgentNameProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const saved = getSetting("agent_name") ?? "Flow";
    const [value, setValue] = useState(saved);

    useEffect(() => setValue(saved), [saved]);

    const commit = () => {
      if (value.trim() !== saved) updateSetting("agent_name", value.trim());
    };

    return (
      <SettingContainer
        title={t("settings.postProcessing.commandMode.agentName.title")}
        description={t(
          "settings.postProcessing.commandMode.agentName.description",
        )}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Input
          variant="compact"
          value={value}
          disabled={isUpdating("agent_name")}
          placeholder="Flow"
          onChange={(e) => setValue(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => e.key === "Enter" && commit()}
        />
      </SettingContainer>
    );
  },
);
AgentName.displayName = "AgentName";
