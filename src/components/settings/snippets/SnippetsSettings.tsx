import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { Snippets } from "./Snippets";

export const SnippetsSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.snippets.title")}>
        <Snippets />
      </SettingsGroup>
    </div>
  );
};
