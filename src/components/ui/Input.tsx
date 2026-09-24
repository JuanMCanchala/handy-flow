import React from "react";

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  variant?: "default" | "compact";
}

export const Input: React.FC<InputProps> = ({
  className = "",
  variant = "default",
  disabled,
  ...props
}) => {
  const baseClasses =
    "h-8 text-body font-normal bg-surface-sunken border border-transparent rounded-sm text-start transition-colors duration-[var(--dur-fast)] placeholder:text-text-tertiary focus:outline-none focus-visible:shadow-[0_0_0_3px_var(--color-focus)]";

  const interactiveClasses = disabled
    ? "opacity-50 pointer-events-none"
    : "hover:border-border-strong focus:bg-surface focus:border-border-strong";

  const variantClasses = {
    default: "px-3 h-8",
    compact: "px-2 h-7 text-small",
  } as const;

  return (
    <input
      className={`${baseClasses} ${variantClasses[variant]} ${interactiveClasses} ${className}`}
      disabled={disabled}
      {...props}
    />
  );
};
