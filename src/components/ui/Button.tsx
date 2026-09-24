import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?:
    | "primary"
    | "primary-soft"
    | "secondary"
    | "warning"
    | "danger"
    | "danger-ghost"
    | "ghost";
  size?: "sm" | "md" | "lg" | "icon";
}

export const Button: React.FC<ButtonProps> = ({
  children,
  className = "",
  variant = "primary",
  size = "md",
  ...props
}) => {
  const baseClasses =
    "inline-flex items-center justify-center gap-1.5 font-medium rounded-md border transition-[background-color,border-color,color,box-shadow] duration-[var(--dur-fast)] disabled:opacity-40 disabled:pointer-events-none cursor-default focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus";

  const variantClasses = {
    primary:
      "bg-accent text-on-accent border-transparent hover:bg-accent-hover shadow-[0_1px_1px_rgb(0_0_0/0.12)]",
    "primary-soft": "bg-fill-active text-text border-transparent hover:bg-fill-selected",
    secondary:
      "bg-surface text-text border-border-strong hover:bg-fill-hover shadow-[0_1px_1px_rgb(0_0_0/0.04)]",
    warning:
      "bg-surface text-text border-border-strong hover:border-warning hover:text-warning",
    danger: "bg-error text-white border-transparent hover:brightness-95",
    "danger-ghost": "bg-transparent text-error border-transparent hover:bg-error/10",
    ghost:
      "bg-transparent text-text-secondary border-transparent hover:bg-fill-hover hover:text-text",
  };

  const sizeClasses = {
    sm: "h-7 px-2.5 text-caption",
    md: "h-8 px-3.5 text-small",
    lg: "h-9 px-4 text-body",
    icon: "size-7 p-0",
  };

  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
