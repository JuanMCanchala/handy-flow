import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { PageHeader } from "../../ui/PageHeader";
import { Transforms } from "./Transforms";

export const TransformsSettings: React.FC = () => {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={t("sidebar.transforms")} />
      <SettingsGroup title={t("settings.transforms.title")}>
        <Transforms />
      </SettingsGroup>
    </div>
  );
};
