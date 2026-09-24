import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useSettings } from "../../../hooks/useSettings";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { SettingContainer } from "../../ui/SettingContainer";

const SnippetsComponent: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const snippets = getSetting("snippets") || [];

  const [newTrigger, setNewTrigger] = useState("");
  const [newExpansion, setNewExpansion] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editTrigger, setEditTrigger] = useState("");
  const [editExpansion, setEditExpansion] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  const handleAdd = async () => {
    const trigger = newTrigger.trim();
    const expansion = newExpansion.trim();
    if (!trigger || !expansion) return;

    setIsSaving(true);
    try {
      const result = await commands.addSnippet(trigger, expansion);
      if (result.status === "ok") {
        await refreshSettings();
        setNewTrigger("");
        setNewExpansion("");
      }
    } finally {
      setIsSaving(false);
    }
  };

  const startEdit = (id: string, trigger: string, expansion: string) => {
    setEditingId(id);
    setEditTrigger(trigger);
    setEditExpansion(expansion);
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditTrigger("");
    setEditExpansion("");
  };

  const handleUpdate = async () => {
    if (!editingId) return;
    const trigger = editTrigger.trim();
    const expansion = editExpansion.trim();
    if (!trigger || !expansion) return;

    setIsSaving(true);
    try {
      const result = await commands.updateSnippet(
        editingId,
        trigger,
        expansion,
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
      const result = await commands.deleteSnippet(id);
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
        title={t("settings.snippets.add.title")}
        description={t("settings.snippets.add.description")}
        descriptionMode="tooltip"
        layout="stacked"
        grouped
      >
        <div className="flex items-center gap-2">
          <Input
            type="text"
            className="max-w-40"
            value={newTrigger}
            onChange={(e) => setNewTrigger(e.target.value)}
            placeholder={t("settings.snippets.add.triggerPlaceholder")}
            variant="compact"
            disabled={isSaving}
          />
          <Input
            type="text"
            className="flex-1"
            value={newExpansion}
            onChange={(e) => setNewExpansion(e.target.value)}
            placeholder={t("settings.snippets.add.expansionPlaceholder")}
            variant="compact"
            disabled={isSaving}
          />
          <Button
            onClick={handleAdd}
            disabled={!newTrigger.trim() || !newExpansion.trim() || isSaving}
            variant="primary"
            size="md"
          >
            {t("settings.snippets.add.add")}
          </Button>
        </div>
      </SettingContainer>

      {snippets.length > 0 && (
        <div className="px-4 py-2 space-y-2">
          {snippets.map((snippet) =>
            editingId === snippet.id ? (
              <div
                key={snippet.id}
                className="flex items-center gap-2 p-2 rounded-lg border border-mid-gray/20"
              >
                <Input
                  type="text"
                  className="max-w-40"
                  value={editTrigger}
                  onChange={(e) => setEditTrigger(e.target.value)}
                  variant="compact"
                  disabled={isSaving}
                />
                <Input
                  type="text"
                  className="flex-1"
                  value={editExpansion}
                  onChange={(e) => setEditExpansion(e.target.value)}
                  variant="compact"
                  disabled={isSaving}
                />
                <Button
                  onClick={handleUpdate}
                  disabled={
                    !editTrigger.trim() || !editExpansion.trim() || isSaving
                  }
                  variant="primary"
                  size="sm"
                >
                  {t("settings.snippets.list.save")}
                </Button>
                <Button
                  onClick={cancelEdit}
                  disabled={isSaving}
                  variant="secondary"
                  size="sm"
                >
                  {t("settings.snippets.list.cancel")}
                </Button>
              </div>
            ) : (
              <div
                key={snippet.id}
                className="flex items-center gap-2 p-2 rounded-lg border border-mid-gray/20"
              >
                <span className="font-medium shrink-0 max-w-40 truncate">
                  {snippet.trigger}
                </span>
                <span className="text-mid-gray shrink-0">→</span>
                <span className="flex-1 truncate">{snippet.expansion}</span>
                <Button
                  onClick={() =>
                    startEdit(snippet.id, snippet.trigger, snippet.expansion)
                  }
                  disabled={isSaving}
                  variant="secondary"
                  size="sm"
                  aria-label={t("settings.snippets.list.edit", {
                    trigger: snippet.trigger,
                  })}
                >
                  {t("settings.snippets.list.edit")}
                </Button>
                <Button
                  onClick={() => handleDelete(snippet.id)}
                  disabled={isSaving}
                  variant="secondary"
                  size="sm"
                  aria-label={t("settings.snippets.list.remove", {
                    trigger: snippet.trigger,
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

export const Snippets = React.memo(SnippetsComponent);
Snippets.displayName = "Snippets";
