import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { PageHeader } from "../../ui/PageHeader";
import { Modes } from "./Modes";

export const ModesSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={t("sidebar.modes")} />
      <SettingsGroup title={t("settings.modes.title")}>
        <Modes />
      </SettingsGroup>
    </div>
  );
};
