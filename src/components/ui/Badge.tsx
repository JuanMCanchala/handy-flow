import React from "react";

interface BadgeProps {
  children: React.ReactNode;
  variant?: "primary" | "success" | "secondary";
  className?: string;
}

const Badge: React.FC<BadgeProps> = ({
  children,
  variant = "primary",
  className = "",
}) => {
  const variantClasses = {
    primary: "bg-fill-active text-text-secondary",
    success: "bg-success/12 text-success",
    secondary: "bg-fill-active text-text-secondary",
  };

  return (
    <span
      className={`inline-flex items-center h-5 px-1.5 rounded-xs text-caption font-medium ${variantClasses[variant]} ${className}`}
    >
      {children}
    </span>
  );
};

export default Badge;
