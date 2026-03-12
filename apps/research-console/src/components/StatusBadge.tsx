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
  return (
    <span className={`status-badge status-${tone}`}>
      <span className="status-dot" aria-hidden="true" />
      {children}
    </span>
  );
}
