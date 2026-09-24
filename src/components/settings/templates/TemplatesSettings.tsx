import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { PageHeader } from "../../ui/PageHeader";
import { NoteTemplates } from "./NoteTemplates";

export const TemplatesSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={t("sidebar.templates")} />
      <SettingsGroup title={t("settings.templates.title")}>
        <NoteTemplates />
      </SettingsGroup>
    </div>
  );
};
