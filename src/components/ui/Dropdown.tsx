import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronsUpDown } from "lucide-react";

export interface DropdownOption {
  value: string;
  label: string;
  description?: string;
  disabled?: boolean;
}

interface DropdownProps {
  options: DropdownOption[];
  className?: string;
  menuClassName?: string;
  selectedValue: string | null;
  onSelect: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  onRefresh?: () => void;
}

export const Dropdown: React.FC<DropdownProps> = ({
  options,
  selectedValue,
  onSelect,
  className = "",
  menuClassName,
  placeholder = "Select an option...",
  disabled = false,
  onRefresh,
}) => {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const selectedOption = options.find(
    (option) => option.value === selectedValue,
  );

  const handleSelect = (value: string) => {
    onSelect(value);
    setIsOpen(false);
  };

  const handleToggle = () => {
    if (disabled) return;
    if (!isOpen && onRefresh) onRefresh();
    setIsOpen(!isOpen);
  };

  return (
    <div className={`relative ${className}`} ref={dropdownRef}>
      <button
        type="button"
        className={`h-8 px-3 text-body bg-surface border border-border-strong rounded-md min-w-[200px] w-full text-start grid grid-cols-[1fr_auto] gap-2 items-center transition-colors duration-[var(--dur-fast)] cursor-default ${
          disabled ? "opacity-40 pointer-events-none" : "hover:bg-fill-hover"
        }`}
        onClick={handleToggle}
        disabled={disabled}
      >
        <span className="truncate">{selectedOption?.label || placeholder}</span>
        <ChevronsUpDown size={14} className="text-text-tertiary shrink-0" />
      </button>
      {isOpen && !disabled && (
        <div
          className={`absolute top-full mt-1 bg-surface-raised rounded-md shadow-pop p-1 z-50 max-h-72 overflow-auto ${
            menuClassName ?? "left-0 right-0"
          }`}
        >
          {options.length === 0 ? (
            <div className="px-2 py-1.5 text-small text-text-tertiary">
              {t("common.noOptionsFound")}
            </div>
          ) : (
            options.map((option) => (
              <button
                key={option.value}
                type="button"
                className={`w-full text-body text-start rounded-sm hover:bg-fill-hover transition-colors duration-[var(--dur-fast)] flex items-start justify-between gap-2 cursor-default ${
                  option.description ? "px-2 py-1.5" : "h-7 px-2"
                } ${option.disabled ? "opacity-40 pointer-events-none" : ""}`}
                onClick={() => handleSelect(option.value)}
                disabled={option.disabled}
              >
                <span className="min-w-0">
                  <span className="block whitespace-normal break-words">
                    {option.label}
                  </span>
                  {option.description && (
                    <span className="mt-0.5 block whitespace-normal text-caption font-normal leading-snug text-text-tertiary">
                      {option.description}
                    </span>
                  )}
                </span>
                {selectedValue === option.value && (
                  <Check size={14} className="text-text shrink-0 mt-0.5" />
                )}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
};
