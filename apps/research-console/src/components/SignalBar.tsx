type SignalBarProps = {
  label: string;
  value: number;
  max: number;
  detail?: string;
  tone?: "neutral" | "signal" | "pressure" | "verify" | "anomaly" | "authority";
};

export function SignalBar({
  label,
  value,
  max,
  detail,
  tone = "neutral",
}: SignalBarProps) {
  const safeValue = Number.isFinite(value) ? value : 0;
  const safeMax = Number.isFinite(max) && max > 0 ? max : 1;
  const ratio = Math.max(0, Math.min(1, safeValue / safeMax));

  return (
    <div className={`signal-bar signal-bar-${tone}`}>
      <div className="signal-bar-head">
        <span className="signal-bar-label">{label}</span>
        <strong>{safeValue}</strong>
      </div>
      <div className="signal-bar-track" aria-hidden="true">
        <div className="signal-bar-fill" style={{ width: `${ratio * 100}%` }} />
      </div>
      {detail ? <div className="signal-bar-detail">{detail}</div> : null}
    </div>
  );
}
