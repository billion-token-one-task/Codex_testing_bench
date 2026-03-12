import { useMemo, useState } from "react";

import { ActionLauncher } from "../components/ActionLauncher";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { StatusBadge } from "../components/StatusBadge";
import { api } from "../lib/api";
import { formatCompact, formatDate, truncateMiddle } from "../lib/format";
import { useProcesses, useRecentEventLines, useWorkspaceIndex } from "../lib/store";

export function LivePage() {
  const { data: processes } = useProcesses();
  const { data: workspace } = useWorkspaceIndex();
  const recentLines = useRecentEventLines(120);
  const [stopping, setStopping] = useState<string | null>(null);

  const running = useMemo(
    () => (processes ?? []).filter((process) => process.status === "running"),
    [processes],
  );

  const stop = async (processId: string) => {
    setStopping(processId);
    try {
      await api.action("stop", { process_id: processId });
    } finally {
      setStopping(null);
    }
  };

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Live Mission Control"
        title="Live"
        description="监视受管进程、实时输出、活跃 campaign，以及 bench 当前的并行槽位占用情况。这里用来盯求解、评分、报告重建与调试流程。"
        actions={<ActionLauncher />}
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Running Processes" value={running.length} detail={`${processes?.length ?? 0} tracked total`} tone="signal" />
        <MetricCard label="Active Runs" value={workspace?.summary.active_run_count ?? 0} detail={`${workspace?.summary.completed_run_count ?? 0} completed`} tone="pressure" />
        <MetricCard label="Recent Output Lines" value={recentLines.length} detail="stdout / stderr across managed processes" tone="verify" />
        <MetricCard label="Observed Tool Calls" value={formatCompact(workspace?.summary.total_tool_calls)} detail={`${formatCompact(workspace?.summary.total_commands)} commands`} />
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Managed Process Matrix" kicker="Launch / stop / inspect">
          <div className="table-wrap">
            <table className="ledger-table">
              <thead>
                <tr>
                  <th>Kind</th>
                  <th>Status</th>
                  <th>Started</th>
                  <th>Command</th>
                  <th>CWD</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {(processes ?? []).map((process) => (
                  <tr key={process.id}>
                    <td>{process.kind}</td>
                    <td><StatusBadge tone={process.status === "running" ? "running" : "neutral"}>{process.status}</StatusBadge></td>
                    <td>{formatDate(process.started_at)}</td>
                    <td className="mono-cell">{truncateMiddle(process.command.join(" "), 88)}</td>
                    <td className="mono-cell">{truncateMiddle(process.cwd, 34)}</td>
                    <td>
                      <button
                        type="button"
                        disabled={process.status !== "running" || stopping === process.id}
                        onClick={() => void stop(process.id)}
                      >
                        {stopping === process.id ? "Stopping…" : "Stop"}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Panel>

        <Panel title="Live Console" kicker="Streaming stdout / stderr">
          <div className="console-pane">
            {recentLines.map((line) => (
              <div key={line.id} className={`console-line console-${line.stream}`}>
                <span>{line.timestamp.slice(11, 19)}</span>
                <span>{line.stream.toUpperCase()}</span>
                <span>{line.processId}</span>
                <span>{line.line}</span>
              </div>
            ))}
          </div>
        </Panel>
      </div>
    </div>
  );
}
