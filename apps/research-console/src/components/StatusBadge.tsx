import type { ReactNode } from "react";

export function StatusBadge({
  tone,
  children,
}: {
  tone:
    | "neutral"
    | "running"
    | "completed"
    | "failed"
    | "graded"
    | "warning"
    | "anomaly";
  children: ReactNode;
}) {
  return <span className={`status-badge status-${tone}`}>{children}</span>;
}
