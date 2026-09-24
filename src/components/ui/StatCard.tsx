import React from "react";

interface StatCardProps {
  label: string;
  value: number | string;
}

export const StatCard: React.FC<StatCardProps> = ({ label, value }) => {
  const display =
    typeof value === "number" ? new Intl.NumberFormat().format(value) : value;

  return (
    <div className="rounded-lg bg-surface border border-border px-5 py-4 flex flex-col gap-1">
      <span className="font-display text-display-xl tabular text-text">
        {display}
      </span>
      <span className="text-small text-text-secondary">{label}</span>
    </div>
  );
};
