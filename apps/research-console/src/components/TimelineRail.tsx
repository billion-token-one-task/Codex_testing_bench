import type { TimelineRow } from "../lib/types";
import { formatDate } from "../lib/format";

export function TimelineRail({
  rows,
  emptyLabel = "暂无可展示时间线",
}: {
  rows: TimelineRow[];
  emptyLabel?: string;
}) {
  if (rows.length === 0) {
    return <div className="empty-box">{emptyLabel}</div>;
  }
  return (
    <div className="timeline-rail">
      {rows.map((row, index) => (
        <article key={`${row.lane}-${row.kind}-${row.timestamp ?? index}`} className={`timeline-card lane-${row.lane}`}>
          <div className="timeline-meta">
            <span className="timeline-lane">{row.lane}</span>
            <span className="timeline-kind">{row.kind}</span>
            <span className="timeline-stamp">{formatDate(row.timestamp)}</span>
          </div>
          <h3>{row.title}</h3>
          <p>{row.summary || "—"}</p>
        </article>
      ))}
    </div>
  );
}
