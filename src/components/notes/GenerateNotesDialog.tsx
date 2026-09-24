import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { useSettings } from "../../hooks/useSettings";

interface GenerateNotesDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onGenerate: (templateId: string) => Promise<void>;
}

export const GenerateNotesDialog: React.FC<GenerateNotesDialogProps> = ({
  open,
  onOpenChange,
  onGenerate,
}) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const templates = getSetting("note_templates") || [];
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);

  const handleGenerate = async () => {
    if (!selectedId) return;
    setIsGenerating(true);
    try {
      await onGenerate(selectedId);
      onOpenChange(false);
    } finally {
      setIsGenerating(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("notes.generateDialog.title")}
      closeLabel={t("common.close")}
      dismissible={!isGenerating}
      footer={
        <>
          <Button
            onClick={() => onOpenChange(false)}
            disabled={isGenerating}
            variant="secondary"
            size="md"
          >
            {t("notes.generateDialog.cancel")}
          </Button>
          <Button
            onClick={handleGenerate}
            disabled={!selectedId || isGenerating}
            variant="primary"
            size="md"
          >
            {isGenerating
              ? t("notes.generateDialog.generating")
              : t("notes.generateDialog.generate")}
          </Button>
        </>
      }
    >
      {templates.length === 0 ? (
        <p className="text-small text-text-secondary">
          {t("notes.generateDialog.noTemplates")}
        </p>
      ) : (
        <div className="space-y-2">
          {templates.map((template) => (
            <label
              key={template.id}
              className="flex items-start gap-2 p-2 rounded-lg border border-border cursor-default"
            >
              <input
                type="radio"
                name="note-template"
                checked={selectedId === template.id}
                onChange={() => setSelectedId(template.id)}
                className="mt-1"
              />
              <span className="text-small text-text">{template.name}</span>
            </label>
          ))}
        </div>
      )}
    </Dialog>
  );
};
