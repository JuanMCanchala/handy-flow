import React from "react";
import { AlertCircle, AlertTriangle, Info, CheckCircle } from "lucide-react";

type AlertVariant = "error" | "warning" | "info" | "success";

interface AlertProps {
  variant?: AlertVariant;
  /** When true, removes rounded corners for use inside containers */
  contained?: boolean;
  children: React.ReactNode;
  className?: string;
}

const variantStyles: Record<
  AlertVariant,
  { container: string; icon: string; text: string }
> = {
  error: {
    container: "border border-error/30 bg-error/8",
    icon: "text-error",
    text: "text-text",
  },
  warning: {
    container: "border border-warning/30 bg-warning/8",
    icon: "text-warning",
    text: "text-text",
  },
  info: {
    container: "border border-border bg-fill-hover",
    icon: "text-text-secondary",
    text: "text-text",
  },
  success: {
    container: "border border-success/30 bg-success/8",
    icon: "text-success",
    text: "text-text",
  },
};

const variantIcons: Record<AlertVariant, React.ElementType> = {
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
  success: CheckCircle,
};

export const Alert: React.FC<AlertProps> = ({
  variant = "error",
  contained = false,
  children,
  className = "",
}) => {
  const styles = variantStyles[variant];
  const Icon = variantIcons[variant];

  return (
    <div
      className={`flex items-start gap-3 px-4 py-3 ${styles.container} ${contained ? "" : "rounded-lg"} ${className}`}
    >
      <Icon className={`w-5 h-5 shrink-0 mt-0.5 ${styles.icon}`} />
      <p className={`text-small ${styles.text}`}>{children}</p>
    </div>
  );
};
