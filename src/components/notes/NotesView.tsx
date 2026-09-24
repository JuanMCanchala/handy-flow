import React from "react";
import { useTranslation } from "react-i18next";
import { Copy, Download } from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { MarkdownContent } from "../whats-new/MarkdownContent";
import { Button } from "../ui/Button";
import { copyToClipboard } from "../settings/history/clipboard";

interface ActionItem {
  text: string;
  checked: boolean;
}

/** Parses `- [ ]` / `- [x]` checklist lines out of notes markdown, in order. */
const parseActionItems = (markdown: string): ActionItem[] => {
  const items: ActionItem[] = [];
  for (const line of markdown.split("\n")) {
    const trimmed = line.trimStart();
    const match = /^[-*]\s\[([ xX])\]\s+(.+)$/.exec(trimmed);
    if (match) {
      items.push({ checked: match[1].toLowerCase() === "x", text: match[2] });
    }
  }
  return items;
};

interface NotesViewProps {
  entryId: number;
  entryTitle: string;
  markdown: string;
  onMarkdownChange: (markdown: string) => void;
}

export const NotesView: React.FC<NotesViewProps> = ({
  entryId,
  entryTitle,
  markdown,
  onMarkdownChange,
}) => {
  const { t } = useTranslation();
  const actionItems = parseActionItems(markdown);

  const handleToggleItem = async (index: number) => {
    const result = await commands.toggleNoteActionItem(entryId, index);
    if (result.status === "ok") {
      onMarkdownChange(result.data);
    } else {
      toast.error(t("notes.toggleItemError"));
    }
  };

  const handleCopy = async () => {
    const copied = await copyToClipboard(markdown);
    if (!copied) {
      toast.error(t("notes.copyError"));
    }
  };

  const handleExport = async () => {
    const destPath = await save({
      defaultPath: `${entryTitle}.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (!destPath) return;

    try {
      await writeTextFile(destPath, markdown);
    } catch (error) {
      console.error("Failed to export notes:", error);
      toast.error(t("notes.exportError"));
    }
  };

  return (
    <div className="flex flex-col gap-3 p-3 rounded-lg border border-border bg-surface-sunken">
      <div className="flex items-center justify-between">
        <h3 className="text-small font-medium text-text">{t("notes.title")}</h3>
        <div className="flex items-center gap-1">
          <Button
            onClick={handleCopy}
            variant="secondary"
            size="sm"
            title={t("notes.copy")}
          >
            <Copy width={14} height={14} />
          </Button>
          <Button
            onClick={handleExport}
            variant="secondary"
            size="sm"
            title={t("notes.export")}
          >
            <Download width={14} height={14} />
          </Button>
        </div>
      </div>

      <MarkdownContent markdown={markdown} />

      {actionItems.length > 0 && (
        <div className="space-y-1.5 pt-2 border-t border-border">
          <h4 className="text-caption font-medium text-text-tertiary uppercase tracking-wide">
            {t("notes.actionItems")}
          </h4>
          {actionItems.map((item, index) => (
            <label
              key={index}
              className="flex items-start gap-2 cursor-default"
            >
              <input
                type="checkbox"
                checked={item.checked}
                onChange={() => handleToggleItem(index)}
                className="mt-1"
              />
              <span
                className={`text-small ${
                  item.checked
                    ? "text-text-tertiary line-through"
                    : "text-text"
                }`}
              >
                {item.text}
              </span>
            </label>
          ))}
        </div>
      )}
    </div>
  );
};
