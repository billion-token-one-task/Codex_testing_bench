import type { ReactNode } from "react";

export type KeyValueItem = {
  label: string;
  value: ReactNode;
  detail?: ReactNode;
  tone?: "neutral" | "signal" | "verify" | "pressure" | "anomaly" | "authority";
};

export function KeyValueGrid({
  items,
  columns = 4,
}: {
  items: KeyValueItem[];
  columns?: 2 | 3 | 4 | 5;
}) {
  return (
    <div className={`key-grid key-grid-${columns}`}>
      {items.map((item) => (
        <div key={item.label} className={`key-grid-card key-grid-${item.tone ?? "neutral"}`}>
          <div className="metric-label">{item.label}</div>
          <strong className="key-grid-value">{item.value}</strong>
          {item.detail ? <div className="metric-detail">{item.detail}</div> : null}
        </div>
      ))}
    </div>
  );
}
