import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Input } from "../../ui/Input";
import { Textarea } from "../../ui/Textarea";
import { Button } from "../../ui/Button";
import { SettingContainer } from "../../ui/SettingContainer";

const NoteTemplatesComponent: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const templates = getSetting("note_templates") || [];

  const [newName, setNewName] = useState("");
  const [newPrompt, setNewPrompt] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editPrompt, setEditPrompt] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  const handleAdd = async () => {
    const name = newName.trim();
    const prompt = newPrompt.trim();
    if (!name || !prompt) return;

    setIsSaving(true);
    try {
      const result = await commands.addNoteTemplate(name, prompt);
      if (result.status === "ok") {
        await refreshSettings();
        setNewName("");
        setNewPrompt("");
      }
    } finally {
      setIsSaving(false);
    }
  };

  const startEdit = (id: string, name: string, prompt: string) => {
    setEditingId(id);
    setEditName(name);
    setEditPrompt(prompt);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditName("");
    setEditPrompt("");
  };

  const handleUpdate = async () => {
    if (!editingId) return;
    const name = editName.trim();
    const prompt = editPrompt.trim();
    if (!name || !prompt) return;

    setIsSaving(true);
    try {
      const result = await commands.updateNoteTemplate(editingId, name, prompt);
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
      const result = await commands.deleteNoteTemplate(id);
      if (result.status === "ok") {
        await refreshSettings();
        if (editingId === id) cancelEdit();
      }
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <>
      <SettingContainer
        title={t("settings.templates.add.title")}
        description={t("settings.templates.add.description")}
        descriptionMode="tooltip"
        layout="stacked"
        grouped
      >
        <div className="space-y-2">
          <Input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder={t("settings.templates.add.namePlaceholder")}
            variant="compact"
            disabled={isSaving}
          />
          <Textarea
            value={newPrompt}
            onChange={(e) => setNewPrompt(e.target.value)}
            placeholder={t("settings.templates.add.promptPlaceholder")}
            disabled={isSaving}
          />
          <Button
            onClick={handleAdd}
            disabled={!newName.trim() || !newPrompt.trim() || isSaving}
            variant="primary"
            size="md"
          >
            {t("settings.templates.add.add")}
          </Button>
        </div>
      </SettingContainer>

      {templates.length > 0 && (
        <div className="px-4 py-2 space-y-2">
          {templates.map((template) =>
            editingId === template.id ? (
              <div
                key={template.id}
                className="flex flex-col gap-2 p-2 rounded-lg border border-border"
              >
                <Input
                  type="text"
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  variant="compact"
                  disabled={isSaving}
                />
                <Textarea
                  value={editPrompt}
                  onChange={(e) => setEditPrompt(e.target.value)}
                  disabled={isSaving}
                />
                <div className="flex gap-2">
                  <Button
                    onClick={handleUpdate}
                    disabled={!editName.trim() || !editPrompt.trim() || isSaving}
                    variant="primary"
                    size="sm"
                  >
                    {t("settings.templates.list.save")}
                  </Button>
                  <Button
                    onClick={cancelEdit}
                    disabled={isSaving}
                    variant="secondary"
                    size="sm"
                  >
                    {t("settings.templates.list.cancel")}
                  </Button>
                </div>
              </div>
            ) : (
              <div
                key={template.id}
                className="flex items-center gap-2 p-2 rounded-lg border border-border"
              >
                <span className="font-medium shrink-0 max-w-40 truncate">
                  {template.name}
                </span>
                <span className="flex-1 truncate text-text-tertiary">
                  {template.prompt}
                </span>
                <Button
                  onClick={() =>
                    startEdit(template.id, template.name, template.prompt)
                  }
                  disabled={isSaving}
                  variant="secondary"
                  size="sm"
                  aria-label={t("settings.templates.list.edit", {
                    name: template.name,
                  })}
                >
                  {t("settings.templates.list.edit")}
                </Button>
                <Button
                  onClick={() => handleDelete(template.id)}
                  disabled={isSaving}
                  variant="secondary"
                  size="sm"
                  aria-label={t("settings.templates.list.remove", {
                    name: template.name,
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

export const NoteTemplates = React.memo(NoteTemplatesComponent);
NoteTemplates.displayName = "NoteTemplates";
