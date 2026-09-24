import React, { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { SettingContainer, SettingsGroup } from "@/components/ui";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import { Input } from "@/components/ui/Input";
import { ApiKeyField } from "../PostProcessingSettingsApi/ApiKeyField";
import { BaseUrlField } from "../PostProcessingSettingsApi/BaseUrlField";
import { ProviderSelect } from "../PostProcessingSettingsApi/ProviderSelect";

export const CloudSttSettings: React.FC = () => {
  const { t } = useTranslation();
  const { settings, getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const [pendingKey, setPendingKey] = useState<string | null>(null);

  const enabled = getSetting("cloud_stt_enabled") || false;
  const providers = settings?.cloud_stt_providers || [];
  const selectedProviderId =
    settings?.cloud_stt_provider_id || providers[0]?.id || "openai";
  const selectedProvider = useMemo(
    () =>
      providers.find((provider) => provider.id === selectedProviderId) ||
      providers[0],
    [providers, selectedProviderId],
  );

  const baseUrl = selectedProvider?.base_url ?? "";
  const apiKey = settings?.cloud_stt_api_keys?.[selectedProviderId] ?? "";
  const model = settings?.cloud_stt_models?.[selectedProviderId] ?? "";

  const providerOptions = useMemo(
    () =>
      providers.map((provider) => ({
        value: provider.id,
        label: provider.label,
      })),
    [providers],
  );

  const withPending = useCallback(
    async (key: string, action: () => Promise<unknown>) => {
      setPendingKey(key);
      try {
        await action();
        await refreshSettings();
      } finally {
        setPendingKey(null);
      }
    },
    [refreshSettings],
  );

  const handleProviderSelect = useCallback(
    (providerId: string) => {
      if (providerId === selectedProviderId) return;
      void withPending("provider", () =>
        commands.setCloudSttProvider(providerId),
      );
    },
    [selectedProviderId, withPending],
  );

  const handleBaseUrlChange = useCallback(
    (value: string) => {
      if (!selectedProvider?.allow_base_url_edit) return;
      const trimmed = value.trim();
      if (trimmed && trimmed !== baseUrl) {
        void withPending("base_url", () =>
          commands.changeCloudSttBaseUrlSetting(selectedProvider.id, trimmed),
        );
      }
    },
    [selectedProvider, baseUrl, withPending],
  );

  const handleApiKeyChange = useCallback(
    (value: string) => {
      const trimmed = value.trim();
      if (trimmed !== apiKey) {
        void withPending("api_key", () =>
          commands.changeCloudSttApiKeySetting(selectedProviderId, trimmed),
        );
      }
    },
    [apiKey, selectedProviderId, withPending],
  );

  const handleModelChange = useCallback(
    (value: string) => {
      const trimmed = value.trim();
      if (trimmed !== model) {
        void withPending("model", () =>
          commands.changeCloudSttModelSetting(selectedProviderId, trimmed),
        );
      }
    },
    [model, selectedProviderId, withPending],
  );

  return (
    <SettingsGroup title={t("settings.cloudStt.title")}>
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("cloud_stt_enabled", value)}
        isUpdating={isUpdating("cloud_stt_enabled")}
        label={t("settings.cloudStt.enable.label")}
        description={t("settings.cloudStt.enable.description")}
        descriptionMode="tooltip"
        grouped={true}
      />

      {enabled && (
        <>
          <SettingContainer
            title={t("settings.cloudStt.provider.title")}
            description={t("settings.cloudStt.provider.description")}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <ProviderSelect
              options={providerOptions}
              value={selectedProviderId}
              onChange={handleProviderSelect}
              disabled={pendingKey === "provider"}
            />
          </SettingContainer>

          {selectedProvider?.allow_base_url_edit && (
            <SettingContainer
              title={t("settings.cloudStt.baseUrl.title")}
              description={t("settings.cloudStt.baseUrl.description")}
              descriptionMode="tooltip"
              layout="horizontal"
              grouped={true}
            >
              <BaseUrlField
                value={baseUrl}
                onBlur={handleBaseUrlChange}
                placeholder={t("settings.cloudStt.baseUrl.placeholder")}
                disabled={pendingKey === "base_url"}
                className="min-w-[380px]"
              />
            </SettingContainer>
          )}

          <SettingContainer
            title={t("settings.cloudStt.model.title")}
            description={t("settings.cloudStt.model.description")}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <Input
              type="text"
              defaultValue={model}
              key={`${selectedProviderId}-model`}
              onBlur={(event) => handleModelChange(event.target.value)}
              placeholder={t("settings.cloudStt.model.placeholder")}
              variant="compact"
              disabled={pendingKey === "model"}
              className="flex-1 min-w-[320px]"
            />
          </SettingContainer>

          <SettingContainer
            title={t("settings.cloudStt.apiKey.title")}
            description={t("settings.cloudStt.apiKey.description")}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <ApiKeyField
              value={apiKey}
              onBlur={handleApiKeyChange}
              placeholder={t("settings.cloudStt.apiKey.placeholder")}
              disabled={pendingKey === "api_key"}
              className="min-w-[320px]"
            />
          </SettingContainer>
        </>
      )}
    </SettingsGroup>
  );
};
