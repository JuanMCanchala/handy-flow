import React from "react";

type ModelStatus =
  | "ready"
  | "loading"
  | "downloading"
  | "verifying"
  | "extracting"
  | "error"
  | "unloaded"
  | "none";

interface ModelStatusButtonProps {
  status: ModelStatus;
  displayText: string;
  isDropdownOpen: boolean;
  onClick: () => void;
  className?: string;
}

const ModelStatusButton: React.FC<ModelStatusButtonProps> = ({
  status,
  displayText,
  isDropdownOpen,
  onClick,
  className = "",
}) => {
  const getStatusColor = (status: ModelStatus): string => {
    switch (status) {
      case "ready":
        return "bg-success";
      case "loading":
        return "bg-warning animate-pulse";
      case "downloading":
        return "bg-brand animate-pulse";
      case "verifying":
        return "bg-warning animate-pulse";
      case "extracting":
        return "bg-warning animate-pulse";
      case "error":
        return "bg-error";
      case "unloaded":
        return "bg-text-tertiary";
      case "none":
        return "bg-error";
      default:
        return "bg-text-tertiary";
    }
  };

  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 px-2.5 h-7 rounded-md text-caption text-text-secondary hover:bg-fill-hover hover:text-text transition-colors duration-[var(--dur-fast)] cursor-default ${className}`}
      title={`Model status: ${displayText}`}
    >
      <div className={`w-1.5 h-1.5 rounded-full shrink-0 ${getStatusColor(status)}`} />
      <span className="max-w-28 truncate">{displayText}</span>
      <svg
        className={`w-3 h-3 shrink-0 transition-transform ${isDropdownOpen ? "rotate-180" : ""}`}
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M19 9l-7 7-7-7"
        />
      </svg>
    </button>
  );
};

export default ModelStatusButton;
