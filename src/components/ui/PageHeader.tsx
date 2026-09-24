import React from "react";

interface PageHeaderProps {
  title: string;
  description?: string;
  actions?: React.ReactNode;
}

export const PageHeader: React.FC<PageHeaderProps> = ({
  title,
  description,
  actions,
}) => {
  return (
    <div
      data-tauri-drag-region
      className="flex items-start justify-between gap-4 pt-6"
    >
      <div>
        <h1 className="font-display text-display text-text">{title}</h1>
        {description && (
          <p className="text-small text-text-secondary mt-1">{description}</p>
        )}
      </div>
      {actions && <div className="shrink-0">{actions}</div>}
    </div>
  );
};
