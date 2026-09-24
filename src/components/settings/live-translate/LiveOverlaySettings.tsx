import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands, type SubtitlesPosition } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { Dropdown, type DropdownOption } from "@/components/ui/Dropdown";
import { SettingContainer } from "@/components/ui/SettingContainer";
import { SettingsGroup } from "@/components/ui/SettingsGroup";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";

/** Where the subtitles and the answers panel appear, and how to move them. */
export const LiveOverlayPlacement: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const [moving, setMoving] = useState(false);
  const position = getSetting("live_subtitles_position") ?? "bottom";
  const hasCustom = !!getSetting("live_subtitles_custom_position");
  const answersMoved = !!getSetting("copilot_answers_custom_position");

  useEffect(() => {
    commands.getLiveOverlaysMoveMode().then((result) => {
      if (result.status === "ok") setMoving(result.data);
    });
    // Leaving the page while placing the overlays saves them where they are.
    return () => {
      commands.getLiveOverlaysMoveMode().then((result) => {
        if (result.status === "ok" && result.data)
          commands.setLiveOverlaysMoveMode(false);
      });
    };
  }, []);

  const options = useMemo<DropdownOption[]>(() => {
    const base: DropdownOption[] = [
      { value: "bottom", label: t("liveOverlays.position.bottom") },
      { value: "top", label: t("liveOverlays.position.top") },
    ];
    if (hasCustom || position === "custom")
      base.push({ value: "custom", label: t("liveOverlays.position.custom") });
    return base;
  }, [t, hasCustom, position]);

  const toggleMove = async () => {
    const next = !moving;
    const result = await commands.setLiveOverlaysMoveMode(next);
    if (result.status === "error") {
      toast.error(result.error);
      return;
    }
    setMoving(next);
    if (!next) {
      await refreshSettings();
      toast.success(t("liveOverlays.saved"));
    }
  };

  const resetAnswers = async () => {
    await commands.resetCopilotAnswersPosition();
    await refreshSettings();
  };

  return (
    <SettingsGroup title={t("liveOverlays.title")}>
      <SettingContainer
        title={t("liveOverlays.position.title")}
        description={t("liveOverlays.position.description")}
        descriptionMode="tooltip"
        grouped={true}
        layout="horizontal"
      >
        <Dropdown
          options={options}
          selectedValue={position}
          onSelect={(value) =>
            updateSetting("live_subtitles_position", value as SubtitlesPosition)
          }
          disabled={isUpdating("live_subtitles_position") || moving}
        />
      </SettingContainer>
      <SettingContainer
        title={t("liveOverlays.move.title")}
        description={t("liveOverlays.move.description")}
        descriptionMode="tooltip"
        grouped={true}
        layout="horizontal"
      >
        <div className="flex items-center gap-2">
          {answersMoved && !moving && (
            <Button variant="secondary" size="sm" onClick={resetAnswers}>
              {t("liveOverlays.move.resetAnswers")}
            </Button>
          )}
          <Button
            variant={moving ? "primary" : "secondary"}
            size="sm"
            onClick={toggleMove}
          >
            {moving
              ? t("liveOverlays.move.done")
              : t("liveOverlays.move.start")}
          </Button>
        </div>
      </SettingContainer>
    </SettingsGroup>
  );
};

/**
 * Provider/model used for live translation and answers. A fast endpoint here
 * (Groq, Cerebras) cuts latency without changing dictation cleanup.
 */
export const LiveModelSettings: React.FC = () => {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const providerId = settings?.live_llm_provider_id ?? "";
  const [model, setModel] = useState(settings?.live_llm_model ?? "");

  useEffect(() => {
    setModel(settings?.live_llm_model ?? "");
  }, [settings?.live_llm_model]);

  const effectiveProvider = providerId || settings?.post_process_provider_id;
  const fallbackModel =
    (effectiveProvider && settings?.post_process_models?.[effectiveProvider]) ||
    "";

  const options = useMemo<DropdownOption[]>(() => {
    const providers = settings?.post_process_providers ?? [];
    return [
      { value: "", label: t("liveOverlays.model.sameAsCleanup") },
      ...providers.map((p) => ({ value: p.id, label: p.label })),
    ];
  }, [settings?.post_process_providers, t]);

  const save = async (nextProvider: string, nextModel: string) => {
    const result = await commands.changeLiveLlmSetting(nextProvider, nextModel);
    if (result.status === "error") toast.error(result.error);
    await refreshSettings();
  };

  return (
    <>
      <SettingContainer
        title={t("liveOverlays.model.provider")}
        description={t("liveOverlays.model.providerDescription")}
        descriptionMode="tooltip"
        grouped={true}
        layout="horizontal"
      >
        <Dropdown
          options={options}
          selectedValue={providerId}
          onSelect={(value) => save(value, "")}
        />
      </SettingContainer>
      <SettingContainer
        title={t("liveOverlays.model.model")}
        description={t("liveOverlays.model.modelDescription")}
        descriptionMode="tooltip"
        grouped={true}
        layout="horizontal"
      >
        <Input
          value={model}
          placeholder={fallbackModel.split("/").pop() || ""}
          onChange={(e) => setModel(e.target.value)}
          onBlur={() => {
            if (model !== (settings?.live_llm_model ?? ""))
              save(providerId, model);
          }}
          className="w-64"
        />
      </SettingContainer>
    </>
  );
};
