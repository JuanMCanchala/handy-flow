import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <div className="space-y-2">
      {title && (
        <div className="px-1">
          <h2 className="text-overline uppercase text-text-tertiary tracking-wide">
            {title}
          </h2>
          {description && (
            <p className="text-caption text-text-tertiary mt-1">{description}</p>
          )}
        </div>
      )}
      <div className="bg-surface border border-border rounded-lg overflow-visible">
        <div className="divide-y divide-border">{children}</div>
      </div>
    </div>
  );
};
