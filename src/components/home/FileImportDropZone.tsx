import React from "react";
import { Loader2, Upload, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "../ui/Button";
import { useFileImport } from "./useFileImport";

/**
 * Drop zone + "Import file" button for transcribing audio/video files.
 * Progress and errors are surfaced inline; a successful import shows up as a
 * new history entry via the existing `historyUpdatePayload` event.
 */
export const FileImportDropZone: React.FC = () => {
  const { t } = useTranslation();
  const {
    importing,
    progress,
    error,
    isDragOver,
    clearError,
    pickAndImportFile,
    cancelImport,
  } = useFileImport();

  React.useEffect(() => {
    if (error) {
      toast.error(t("settings.home.import.error", { file: error }));
      clearError();
    }
  }, [error, clearError, t]);

  return (
    <div
      className={`relative rounded-lg border-2 border-dashed px-4 py-6 flex flex-col items-center gap-3 text-center transition-colors ${
        isDragOver
          ? "border-logo-primary bg-logo-primary/5"
          : "border-mid-gray/30"
      }`}
    >
      {importing ? (
        <>
          <Loader2 className="w-6 h-6 animate-spin text-mid-gray" />
          <p className="text-sm text-text/70">
            {progress
              ? t("settings.home.import.progress", {
                  current: progress.currentChunk + 1,
                  total: progress.totalChunks,
                })
              : t("settings.home.import.starting")}
          </p>
          <Button variant="secondary" size="sm" onClick={cancelImport}>
            <X className="w-4 h-4" />
            <span>{t("settings.home.import.cancel")}</span>
          </Button>
        </>
      ) : (
        <>
          <Upload className="w-6 h-6 text-mid-gray" />
          <p className="text-sm text-text/70">
            {t("settings.home.import.dropHint")}
          </p>
          <Button
            variant="secondary"
            size="sm"
            onClick={pickAndImportFile}
            className="flex items-center gap-2"
          >
            <Upload className="w-4 h-4" />
            <span>{t("settings.home.import.button")}</span>
          </Button>
        </>
      )}
    </div>
  );
};
