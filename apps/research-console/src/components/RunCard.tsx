import { Link } from "react-router-dom";

import { detectToneFromStatus, formatCompact, formatDate, formatRelativeTime, summarizeMap } from "../lib/format";
import type { RunIndexEntry } from "../lib/types";
import { StatusBadge } from "./StatusBadge";

export function RunCard({
  run,
  compact = false,
  selected = false,
  onSelect,
}: {
  run: RunIndexEntry;
  compact?: boolean;
  selected?: boolean;
  onSelect?: () => void;
}) {
  const toolHighlights = summarizeMap(run.tool_name_counts, compact ? 2 : 3)
    .map(([name, count]) => `${name}×${count}`)
    .join(" · ");
  const routeHighlights = summarizeMap(run.tool_route_counts, compact ? 2 : 3)
    .map(([name, count]) => `${name}×${count}`)
    .join(" · ");

  return (
    <article
      className={`run-card${compact ? " run-card-compact" : ""}${selected ? " run-card-selected" : ""}${onSelect ? " run-card-clickable" : ""}`}
      onClick={onSelect}
      role={onSelect ? "button" : undefined}
      tabIndex={onSelect ? 0 : undefined}
      onKeyDown={onSelect ? (event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onSelect();
        }
      } : undefined}
    >
      <div className="run-card-head">
        <div>
          <div className="run-card-kicker">{run.cohort_id}</div>
          <h3>
            <Link to={`/runs/${run.run_id}`}>{run.instance_id}</Link>
          </h3>
        </div>
        <div className="run-status-stack">
          <StatusBadge tone={detectToneFromStatus(run.status)}>{run.status}</StatusBadge>
          <StatusBadge tone={detectToneFromStatus(run.grading_status)}>{run.grading_status}</StatusBadge>
        </div>
      </div>

      <div className="brief-meta">
        <span>{run.model}</span>
        <span>{run.personality_mode ?? "none"}</span>
        <span>{run.task_class}</span>
      </div>

      <div className="brief-meta">
        <span>verify {run.verification_closure_count}</span>
        <span>fallback {run.personality_fallback_count}</span>
        <span>friction {run.harness_friction_count}</span>
      </div>

      <div className="brief-meta">
        <span>anomaly {run.anomaly_count}</span>
        <span>patch {run.patch_file_count}</span>
        <span>msg {run.message_metric_count}</span>
      </div>

      <div className="run-card-grid">
        <div>
          <span className="metric-label">Visible</span>
          <strong>{formatCompact(run.visible_output_total_tokens_est)}</strong>
        </div>
        <div>
          <span className="metric-label">Tools</span>
          <strong>{run.tool_count}</strong>
        </div>
        <div>
          <span className="metric-label">Commands</span>
          <strong>{run.command_count}</strong>
        </div>
        <div>
          <span className="metric-label">Tokens</span>
          <strong>{formatCompact(run.total_tokens)}</strong>
        </div>
      </div>

      <div className="run-card-notes">
        <div className="note-row">
          <span className="metric-label">Top tools</span>
          <span>{toolHighlights || "—"}</span>
        </div>
        <div className="note-row">
          <span className="metric-label">Routes</span>
          <span>{routeHighlights || "—"}</span>
        </div>
        <div className="note-row">
          <span className="metric-label">Updated</span>
          <span>{formatRelativeTime(run.latest_updated_at)} · {formatDate(run.latest_updated_at)}</span>
        </div>
      </div>
    </article>
  );
}
