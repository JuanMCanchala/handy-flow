import React from "react";
import ResetIcon from "../icons/ResetIcon";

interface ResetButtonProps {
  onClick: () => void;
  disabled?: boolean;
  className?: string;
  ariaLabel?: string;
  children?: React.ReactNode;
}

export const ResetButton: React.FC<ResetButtonProps> = React.memo(
  ({ onClick, disabled = false, className = "", ariaLabel, children }) => (
    <button
      type="button"
      aria-label={ariaLabel}
      className={`size-7 flex items-center justify-center rounded-md border border-transparent transition-colors duration-[var(--dur-fast)] cursor-default ${
        disabled
          ? "opacity-40 pointer-events-none text-text-tertiary"
          : "hover:bg-fill-hover text-text-secondary hover:text-text"
      } ${className}`}
      onClick={onClick}
      disabled={disabled}
    >
      {children ?? <ResetIcon />}
    </button>
  ),
);
