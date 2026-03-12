import type { ReactNode } from "react";

export function MetricCard({
  label,
  value,
  detail,
  tone = "neutral",
}: {
  label: string;
  value: ReactNode;
  detail?: ReactNode;
  tone?: "neutral" | "signal" | "verify" | "pressure" | "anomaly";
}) {
  return (
    <div className={`metric-card metric-${tone}`}>
      <div className="metric-label">{label}</div>
      <strong className="metric-value">{value}</strong>
      {detail ? <div className="metric-detail">{detail}</div> : null}
    </div>
  );
}
