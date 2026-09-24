import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import type { DiarizationModelInfo, DiarizationModelKind } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { SettingContainer, SettingsGroup } from "@/components/ui";
import { ToggleSwitch } from "@/components/ui/ToggleSwitch";
import { Slider } from "@/components/ui/Slider";
import { Button } from "@/components/ui/Button";

const formatSize = (bytes: number): string => {
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(0)} MB`;
};

export const DiarizationSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, resetSetting, isUpdating } =
    useSettings();
  const [models, setModels] = useState<DiarizationModelInfo[]>([]);
  const [downloadingKind, setDownloadingKind] =
    useState<DiarizationModelKind | null>(null);

  const enabled = getSetting("diarization_enabled") || false;
  const threshold = getSetting("diarization_cluster_threshold") ?? 0.2;

  const refreshModels = useCallback(async () => {
    const result = await commands.getDiarizationModelsStatus();
    if (result.status === "ok") {
      setModels(result.data);
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      void refreshModels();
    }
  }, [enabled, refreshModels]);

  const handleDownload = async (kind: DiarizationModelKind) => {
    setDownloadingKind(kind);
    try {
      await commands.downloadDiarizationModel(kind);
    } finally {
      setDownloadingKind(null);
      await refreshModels();
    }
  };

  const handleDelete = async (kind: DiarizationModelKind) => {
    await commands.deleteDiarizationModel(kind);
    await refreshModels();
  };

  const modelLabel = (kind: DiarizationModelKind): string =>
    kind === "Segmentation"
      ? t("settings.diarization.models.segmentation")
      : t("settings.diarization.models.embedding");

  return (
    <SettingsGroup title={t("settings.diarization.title")}>
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("diarization_enabled", value)}
        isUpdating={isUpdating("diarization_enabled")}
        label={t("settings.diarization.enable.label")}
        description={t("settings.diarization.enable.description")}
        descriptionMode="tooltip"
        grouped={true}
      />

      {enabled && (
        <>
          <Slider
            value={threshold}
            onChange={(value) =>
              updateSetting("diarization_cluster_threshold", value)
            }
            onReset={() => resetSetting("diarization_cluster_threshold")}
            isResetting={isUpdating("diarization_cluster_threshold")}
            min={0.05}
            max={0.5}
            step={0.01}
            label={t("settings.diarization.threshold.title")}
            description={t("settings.diarization.threshold.description")}
            descriptionMode="tooltip"
            grouped={true}
          />

          {models.map((model) => (
            <SettingContainer
              key={model.kind}
              title={modelLabel(model.kind)}
              description={t("settings.diarization.models.description", {
                size: formatSize(model.size_bytes),
              })}
              descriptionMode="tooltip"
              layout="horizontal"
              grouped={true}
            >
              {model.is_downloaded ? (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => handleDelete(model.kind)}
                >
                  {t("settings.diarization.models.delete")}
                </Button>
              ) : (
                <Button
                  variant="primary-soft"
                  size="sm"
                  disabled={downloadingKind === model.kind}
                  onClick={() => handleDownload(model.kind)}
                >
                  {downloadingKind === model.kind
                    ? t("settings.diarization.models.downloading")
                    : t("settings.diarization.models.download", {
                        size: formatSize(model.size_bytes),
                      })}
                </Button>
              )}
            </SettingContainer>
          ))}
        </>
      )}
    </SettingsGroup>
  );
};
