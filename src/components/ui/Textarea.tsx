import React from "react";

interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  variant?: "default" | "compact";
}

export const Textarea: React.FC<TextareaProps> = ({
  className = "",
  variant = "default",
  ...props
}) => {
  const baseClasses =
    "text-body font-normal bg-surface-sunken border border-transparent rounded-sm text-start leading-6 transition-colors duration-[var(--dur-fast)] placeholder:text-text-tertiary hover:border-border-strong focus:outline-none focus:bg-surface focus:border-border-strong focus-visible:shadow-[0_0_0_3px_var(--color-focus)] resize-y";

  const variantClasses = {
    default: "px-3 py-2 min-h-[96px]",
    compact: "px-2 py-1.5 min-h-[80px] text-small",
  };

  return (
    <textarea
      className={`${baseClasses} ${variantClasses[variant]} ${className}`}
      {...props}
    />
  );
};
