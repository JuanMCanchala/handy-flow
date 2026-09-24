import React from "react";
import { useTranslation } from "react-i18next";
import type { ModelInfo } from "@/bindings";
import {
  getTranslatedModelName,
  getTranslatedModelDescription,
} from "../../lib/utils/modelTranslation";

interface ModelDropdownProps {
  models: ModelInfo[];
  currentModelId: string;
  onModelSelect: (modelId: string) => void;
}

const ModelDropdown: React.FC<ModelDropdownProps> = ({
  models,
  currentModelId,
  onModelSelect,
}) => {
  const { t } = useTranslation();
  const downloadedModels = models.filter((m) => m.is_downloaded);

  const handleModelClick = (modelId: string) => {
    onModelSelect(modelId);
  };

  return (
    <div className="absolute bottom-full start-0 mb-2 w-64 max-h-[60vh] overflow-y-auto bg-surface-raised border border-border rounded-md shadow-pop py-2 z-50">
      {downloadedModels.length > 0 ? (
        <div>
          {downloadedModels.map((model) => (
            <div
              key={model.id}
              onClick={() => handleModelClick(model.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleModelClick(model.id);
                }
              }}
              tabIndex={0}
              role="button"
              className={`w-full px-3 py-2 text-start hover:bg-fill-hover transition-colors cursor-default focus:outline-none ${
                currentModelId === model.id
                  ? "bg-fill-selected text-text"
                  : ""
              }`}
            >
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-small text-text">
                    {getTranslatedModelName(model, t)}
                    {model.is_custom && (
                      <span className="ms-1.5 text-caption font-medium text-text-tertiary uppercase">
                        {t("modelSelector.custom")}
                      </span>
                    )}
                    {model.supports_streaming && (
                      <span className="ms-1.5 text-caption font-medium text-text-secondary uppercase">
                        {t("modelSelector.streaming")}
                      </span>
                    )}
                  </div>
                  <div className="text-caption text-text-tertiary italic pe-4">
                    {getTranslatedModelDescription(model, t)}
                  </div>
                </div>
                {currentModelId === model.id && (
                  <div className="text-caption text-text-secondary">
                    {t("modelSelector.active")}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="px-3 py-2 text-small text-text-secondary">
          {t("modelSelector.noModelsAvailable")}
        </div>
      )}
    </div>
  );
};

export default ModelDropdown;
