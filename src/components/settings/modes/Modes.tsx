import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import type { ModeOutputFormat } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Input } from "../../ui/Input";
import { Textarea } from "../../ui/Textarea";
import { Button } from "../../ui/Button";
import { Dropdown, type DropdownOption } from "../../ui/Dropdown";
import { SettingContainer } from "../../ui/SettingContainer";

const OUTPUT_FORMATS: ModeOutputFormat[] = [
  "plain",
  "bullet_list",
  "email",
  "markdown",
];

interface ModeFormState {
  name: string;
  prompt: string;
  providerId: string; // "" means no override
  model: string;
  language: string;
  outputFormat: ModeOutputFormat;
  hotkey: string;
}

const emptyForm: ModeFormState = {
  name: "",
  prompt: "",
  providerId: "",
  model: "",
  language: "",
  outputFormat: "plain",
  hotkey: "",
};

const ModesComponent: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const modes = getSetting("modes") || [];
  const providers = getSetting("post_process_providers") || [];
  const activeModeId = getSetting("active_mode_id") ?? null;

  const [newMode, setNewMode] = useState<ModeFormState>(emptyForm);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editMode, setEditMode] = useState<ModeFormState>(emptyForm);
  const [isSaving, setIsSaving] = useState(false);

  const providerOptions: DropdownOption[] = [
    { value: "", label: t("settings.modes.fields.providerNone") },
    ...providers.map((p) => ({ value: p.id, label: p.label })),
  ];

  const outputFormatOptions: DropdownOption[] = OUTPUT_FORMATS.map((format) => ({
    value: format,
    label: t(`settings.modes.outputFormats.${format}`),
  }));

  const handleAdd = async () => {
    const name = newMode.name.trim();
    const prompt = newMode.prompt.trim();
    if (!name || !prompt) return;

    setIsSaving(true);
    try {
      const result = await commands.addMode(
        name,
        prompt,
        newMode.providerId.trim() || null,
        newMode.model.trim() || null,
        newMode.language.trim() || null,
        newMode.outputFormat,
        newMode.hotkey.trim() || null,
      );
      if (result.status === "ok") {
        await refreshSettings();
        setNewMode(emptyForm);
      }
    } finally {
      setIsSaving(false);
    }
  };

  const startEdit = (mode: (typeof modes)[number]) => {
    setEditingId(mode.id);
    setEditMode({
      name: mode.name,
      prompt: mode.prompt,
      providerId: mode.provider_id ?? "",
      model: mode.model ?? "",
      language: mode.language ?? "",
      outputFormat: mode.output_format,
      hotkey: mode.hotkey ?? "",
    });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditMode(emptyForm);
  };

  const handleUpdate = async () => {
    if (!editingId) return;
    const name = editMode.name.trim();
    const prompt = editMode.prompt.trim();
    if (!name || !prompt) return;

    setIsSaving(true);
    try {
      const result = await commands.updateMode(
        editingId,
        name,
        prompt,
        editMode.providerId.trim() || null,
        editMode.model.trim() || null,
        editMode.language.trim() || null,
        editMode.outputFormat,
        editMode.hotkey.trim() || null,
      );
      if (result.status === "ok") {
        await refreshSettings();
        cancelEdit();
      }
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    setIsSaving(true);
    try {
      const result = await commands.deleteMode(id);
      if (result.status === "ok") {
        await refreshSettings();
        if (editingId === id) cancelEdit();
      }
    } finally {
      setIsSaving(false);
    }
  };

  const handleActivate = async (id: string | null) => {
    setIsSaving(true);
    try {
      const result = await commands.setActiveMode(id);
      if (result.status === "ok") {
        await refreshSettings();
      }
    } finally {
      setIsSaving(false);
    }
  };

  const renderFields = (
    state: ModeFormState,
    setState: (state: ModeFormState) => void,
  ) => (
    <div className="space-y-2">
      <Input
        type="text"
        value={state.name}
        onChange={(e) => setState({ ...state, name: e.target.value })}
        placeholder={t("settings.modes.add.namePlaceholder")}
        variant="compact"
        disabled={isSaving}
      />
      <Textarea
        value={state.prompt}
        onChange={(e) => setState({ ...state, prompt: e.target.value })}
        placeholder={t("settings.modes.add.promptPlaceholder")}
        disabled={isSaving}
      />
      <Dropdown
        options={providerOptions}
        selectedValue={state.providerId}
        onSelect={(value) => setState({ ...state, providerId: value })}
        placeholder={t("settings.modes.fields.provider")}
        disabled={isSaving}
      />
      <Input
        type="text"
        value={state.model}
        onChange={(e) => setState({ ...state, model: e.target.value })}
        placeholder={t("settings.modes.fields.model")}
        variant="compact"
        disabled={isSaving}
      />
      <Input
        type="text"
        value={state.language}
        onChange={(e) => setState({ ...state, language: e.target.value })}
        placeholder={t("settings.modes.fields.languagePlaceholder")}
        variant="compact"
        disabled={isSaving}
      />
      <Dropdown
        options={outputFormatOptions}
        selectedValue={state.outputFormat}
        onSelect={(value) =>
          setState({ ...state, outputFormat: value as ModeOutputFormat })
        }
        placeholder={t("settings.modes.fields.outputFormat")}
        disabled={isSaving}
      />
      <Input
        type="text"
        value={state.hotkey}
        onChange={(e) => setState({ ...state, hotkey: e.target.value })}
        placeholder={t("settings.modes.fields.hotkey")}
        variant="compact"
        disabled={isSaving}
      />
    </div>
  );

  return (
    <>
      <SettingContainer
        title={t("settings.modes.add.title")}
        description={t("settings.modes.add.description")}
        descriptionMode="tooltip"
        layout="stacked"
        grouped
      >
        {renderFields(newMode, setNewMode)}
        <Button
          onClick={handleAdd}
          disabled={!newMode.name.trim() || !newMode.prompt.trim() || isSaving}
          variant="primary"
          size="md"
        >
          {t("settings.modes.add.add")}
        </Button>
      </SettingContainer>

      {modes.length > 0 && (
        <div className="px-4 py-2 space-y-2">
          <div className="flex items-center gap-2 p-2 rounded-lg border border-border">
            <span className="font-medium shrink-0">
              {t("settings.modes.active")}
            </span>
            <Dropdown
              options={[
                { value: "", label: t("settings.modes.noneOption") },
                ...modes.map((m) => ({ value: m.id, label: m.name })),
              ]}
              selectedValue={activeModeId ?? ""}
              onSelect={(value) => handleActivate(value || null)}
              disabled={isSaving}
              className="flex-1"
            />
          </div>

          {modes.map((mode) =>
            editingId === mode.id ? (
              <div
                key={mode.id}
                className="flex flex-col gap-2 p-2 rounded-lg border border-border"
              >
                {renderFields(editMode, setEditMode)}
                <div className="flex gap-2">
                  <Button
                    onClick={handleUpdate}
                    disabled={
                      !editMode.name.trim() || !editMode.prompt.trim() || isSaving
                    }
                    variant="primary"
                    size="sm"
                  >
                    {t("settings.modes.list.save")}
                  </Button>
                  <Button
                    onClick={cancelEdit}
                    disabled={isSaving}
                    variant="secondary"
                    size="sm"
                  >
                    {t("settings.modes.list.cancel")}
                  </Button>
                </div>
              </div>
            ) : (
              <div
                key={mode.id}
                className="flex items-center gap-2 p-2 rounded-lg border border-border"
              >
                <span className="font-medium shrink-0 max-w-40 truncate">
                  {mode.name}
                </span>
                <span className="flex-1 truncate text-text-tertiary">
                  {mode.prompt}
                </span>
                <Button
                  onClick={() => startEdit(mode)}
                  disabled={isSaving}
                  variant="secondary"
                  size="sm"
                  aria-label={t("settings.modes.list.edit", {
                    name: mode.name,
                  })}
                >
                  {t("settings.modes.list.edit")}
                </Button>
                <Button
                  onClick={() => handleDelete(mode.id)}
                  disabled={isSaving}
                  variant="secondary"
                  size="sm"
                  aria-label={t("settings.modes.list.remove", {
                    name: mode.name,
                  })}
                >
                  <svg
                    className="w-3 h-3"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </Button>
              </div>
            ),
          )}
        </div>
      )}
    </>
  );
};

export const Modes = React.memo(ModesComponent);
Modes.displayName = "Modes";
