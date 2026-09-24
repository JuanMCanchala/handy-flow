import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { PageHeader } from "../../ui/PageHeader";
import { Snippets } from "./Snippets";

export const SnippetsSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={t("sidebar.snippets")} />
      <SettingsGroup title={t("settings.snippets.title")}>
        <Snippets />
      </SettingsGroup>
    </div>
  );
};
