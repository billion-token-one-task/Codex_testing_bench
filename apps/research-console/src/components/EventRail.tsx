import { detectToneFromStatus, formatDate, humanizeKey, truncateEnd } from "../lib/format";
import type { LiveEventBucket } from "../lib/types";
import { StatusBadge } from "./StatusBadge";

function classifyEvent(row: LiveEventBucket) {
  if (row.type.includes("patch")) return "verify";
  if (row.type.includes("tool") || row.type.includes("command")) return "pressure";
  if (row.type.includes("personality") || row.type.includes("skill") || row.type.includes("token")) return "signal";
  if (row.type.includes("warning") || row.type.includes("anomaly")) return "failed";
  return "neutral";
}

function eventHeadline(row: LiveEventBucket) {
  const payload = row.payload;
  const direct =
    payload.summary ??
    payload.messagePreview ??
    payload.latest_message_preview ??
    payload.latest_tool ??
    payload.latest_patch ??
    payload.latest_command ??
    payload.latest_mechanism_event ??
    payload.toolName ??
    payload.skillName ??
    payload.title;
  if (typeof direct === "string" && direct.trim()) {
    return truncateEnd(direct, 108);
  }
  return truncateEnd(humanizeKey(row.type), 108);
}

function eventBadges(row: LiveEventBucket) {
  const payload = row.payload;
  return [
    typeof payload.cohortId === "string" ? payload.cohortId : null,
    typeof payload.instanceId === "string" ? payload.instanceId : null,
    typeof payload.artifact === "string" ? payload.artifact : null,
    typeof payload.phase === "string" ? payload.phase : null,
    typeof payload.toolKind === "string" ? payload.toolKind : null,
    typeof payload.toolRoute === "string" ? payload.toolRoute : null,
  ].filter((value): value is string => Boolean(value));
}

export function EventRail({
  title,
  rows,
  emptyLabel = "当前没有事件。",
}: {
  title?: string;
  rows: LiveEventBucket[];
  emptyLabel?: string;
}) {
  if (!rows.length) {
    return <div className="empty-box">{emptyLabel}</div>;
  }

  return (
    <div className="event-rail">
      {title ? <div className="section-label">{title}</div> : null}
      {rows.map((row, index) => {
        const keys = Object.entries(row.payload)
          .filter(([key, value]) => {
            if (["summary", "messagePreview", "row", "payload", "cohortId", "instanceId", "artifact", "phase", "toolKind", "toolRoute"].includes(key)) {
              return false;
            }
            return typeof value === "string" || typeof value === "number" || typeof value === "boolean";
          })
          .slice(0, 5);
        const tone = classifyEvent(row);
        const badges = eventBadges(row);
        const status = typeof row.payload.run_status === "string"
          ? row.payload.run_status
          : typeof row.payload.status === "string"
            ? row.payload.status
            : null;
        return (
          <article key={`${row.type}-${row.timestamp ?? index}`} className={`event-card event-card-${tone}`}>
            <div className="event-card-head">
              <span className="event-kind">{row.type}</span>
              <span className="event-time">{formatDate(row.timestamp)}</span>
            </div>
            <div className="event-card-summary">
              <strong>{eventHeadline(row)}</strong>
              {status ? (
                <StatusBadge tone={detectToneFromStatus(status)}>{status}</StatusBadge>
              ) : (
                <StatusBadge tone={tone === "failed" ? "failed" : tone === "pressure" ? "warning" : tone === "signal" ? "running" : "neutral"}>
                  {tone}
                </StatusBadge>
              )}
            </div>
            {badges.length ? (
              <div className="event-chip-row">
                {badges.map((badge) => (
                  <span key={badge} className="event-chip">{badge}</span>
                ))}
              </div>
            ) : null}
            <div className="event-card-body">
              {keys.length === 0 ? (
                <div className="mono-note">No scalar payload preview.</div>
              ) : (
                keys.map(([key, value]) => (
                  <div key={key} className="event-kv">
                    <span>{humanizeKey(key)}</span>
                    <strong>{truncateEnd(String(value), 88)}</strong>
                  </div>
                ))
              )}
            </div>
          </article>
        );
      })}
    </div>
  );
}
