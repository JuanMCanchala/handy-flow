import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./Button";

interface PathDisplayProps {
  path: string;
  onOpen: () => void;
  disabled?: boolean;
}

export const PathDisplay: React.FC<PathDisplayProps> = ({
  path,
  onOpen,
  disabled = false,
}) => {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-2">
      <div className="flex-1 min-w-0 px-3 py-2 bg-surface-sunken rounded-sm text-caption font-mono break-all select-text cursor-text">
        {path}
      </div>
      <Button onClick={onOpen} variant="secondary" size="sm" disabled={disabled}>
        {t("common.open")}
      </Button>
    </div>
  );
};
