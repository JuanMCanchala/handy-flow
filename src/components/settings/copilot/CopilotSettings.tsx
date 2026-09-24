import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { Upload } from "lucide-react";
import { useCopilotStore } from "@/stores/copilotStore";
import type { CopilotAnswerLanguage } from "@/bindings";
import { Button } from "../../ui/Button";
import { Dropdown, type DropdownOption } from "../../ui/Dropdown";
import { PageHeader } from "../../ui/PageHeader";
import { SettingContainer } from "../../ui/SettingContainer";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { Textarea } from "../../ui/Textarea";
import { ShortcutInput } from "../ShortcutInput";

export const CopilotSettings: React.FC = () => {
  const { t } = useTranslation();
  const {
    text,
    answerLanguage,
    history,
    isLoading,
    isActive,
    initialize,
    setText,
    setAnswerLanguage,
    importFile,
    clearHistory,
    toggle,
  } = useCopilotStore();

  useEffect(() => {
    initialize();
  }, [initialize]);

  const languageOptions = useMemo<DropdownOption[]>(
    () => [
      { value: "auto", label: t("settings.copilot.answerLanguage.options.auto") },
      { value: "en", label: t("settings.copilot.answerLanguage.options.en") },
      { value: "es", label: t("settings.copilot.answerLanguage.options.es") },
      { value: "both", label: t("settings.copilot.answerLanguage.options.both") },
    ],
    [t],
  );

  const handleImport = async () => {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Profile",
          extensions: ["txt", "md", "pdf", "docx"],
        },
      ],
    });
    if (!selected || Array.isArray(selected)) return;

    try {
      const imported = await importFile(selected);
      const merged = text.trim().length > 0 ? `${text}\n\n${imported}` : imported;
      await setText(merged);
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : t("settings.copilot.importError"),
      );
    }
  };

  const handleToggle = async () => {
    try {
      await toggle();
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : t("settings.copilot.toggleError"),
      );
    }
  };

  return (
    <div className="flex flex-col gap-8">
      <PageHeader title={t("sidebar.copilot")} />

      <SettingsGroup title={t("settings.copilot.title")}>
        <ShortcutInput shortcutId="copilot" grouped={true} />
        <SettingContainer
          title={t("settings.copilot.answerLanguage.title")}
          description={t("settings.copilot.answerLanguage.description")}
          descriptionMode="tooltip"
          grouped={true}
          layout="horizontal"
        >
          <Dropdown
            options={languageOptions}
            selectedValue={answerLanguage}
            onSelect={(value) =>
              setAnswerLanguage(value as CopilotAnswerLanguage)
            }
          />
        </SettingContainer>
      </SettingsGroup>

      <div className="flex justify-center">
        <Button
          variant={isActive ? "danger" : "primary"}
          onClick={handleToggle}
          disabled={isLoading}
        >
          {isActive
            ? t("settings.copilot.stopButton")
            : t("settings.copilot.startButton")}
        </Button>
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <h2 className="text-body font-medium text-text">
            {t("settings.copilot.profile.title")}
          </h2>
          <Button
            variant="secondary"
            size="sm"
            onClick={handleImport}
            className="flex items-center gap-2"
          >
            <Upload className="w-4 h-4" />
            <span>{t("settings.copilot.profile.importButton")}</span>
          </Button>
        </div>
        <Textarea
          className="w-full min-h-[220px]"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder={t("settings.copilot.profile.placeholder")}
        />
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <h2 className="text-body font-medium text-text">
            {t("settings.copilot.history.title")}
          </h2>
          {history.length > 0 && (
            <Button variant="ghost" size="sm" onClick={clearHistory}>
              {t("settings.copilot.history.clear")}
            </Button>
          )}
        </div>
        <div className="bg-surface border border-border rounded-lg divide-y divide-border max-h-[320px] overflow-y-auto">
          {history.length === 0 ? (
            <div className="px-4 py-3 text-center text-text-secondary text-small">
              {t("settings.copilot.history.empty")}
            </div>
          ) : (
            [...history].reverse().map((entry) => (
              <div key={entry.id} className="px-3 py-2">
                <p className="text-caption text-text-tertiary">
                  {entry.question}
                </p>
                <p className="text-body text-text">{entry.answer}</p>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
