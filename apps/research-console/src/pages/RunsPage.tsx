import { Link } from "react-router-dom";
import { useMemo, useState } from "react";

import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { StatusBadge } from "../components/StatusBadge";
import { formatCompact, formatDate, percentFromBps, summarizeMap } from "../lib/format";
import { useWorkspaceIndex } from "../lib/store";

export function RunsPage() {
  const { data } = useWorkspaceIndex();
  const [modelFilter, setModelFilter] = useState("");
  const [personalityFilter, setPersonalityFilter] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [taskClassFilter, setTaskClassFilter] = useState("");

  const runs = useMemo(() => {
    return (data?.runs ?? []).filter((run) => {
      if (modelFilter && !run.model.includes(modelFilter)) return false;
      if (personalityFilter && (run.personality_mode ?? "") !== personalityFilter) return false;
      if (statusFilter && run.status !== statusFilter) return false;
      if (taskClassFilter && run.task_class !== taskClassFilter) return false;
      return true;
    });
  }, [data?.runs, modelFilter, personalityFilter, statusFilter, taskClassFilter]);

  const taskClasses = Array.from(new Set((data?.runs ?? []).map((run) => run.task_class))).sort();

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Run Ledger"
        title="Runs"
        description="按 model、personality、task class 与状态筛选 run。每一行都附带更强的摘要指标，便于快速挑出值得深入研究的样本。"
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Filtered Runs" value={runs.length} detail={`${data?.runs.length ?? 0} total indexed`} />
        <MetricCard label="Visible Output" value={formatCompact(runs.reduce((sum, run) => sum + run.visible_output_total_tokens_est, 0))} detail="estimated visible tokens" tone="signal" />
        <MetricCard label="Tool Calls" value={formatCompact(runs.reduce((sum, run) => sum + run.tool_count, 0))} detail={`${formatCompact(runs.reduce((sum, run) => sum + run.command_count, 0))} commands`} tone="pressure" />
        <MetricCard label="Verification Closures" value={formatCompact(runs.reduce((sum, run) => sum + run.verification_closure_count, 0))} detail={`${formatCompact(runs.reduce((sum, run) => sum + run.harness_friction_count, 0))} friction events`} tone="verify" />
      </div>

      <Panel title="Filterable Run Ledger" kicker="Search model, personality, task-class and status">
        <div className="filter-row">
          <input value={modelFilter} onChange={(event) => setModelFilter(event.target.value)} placeholder="model contains…" />
          <select value={personalityFilter} onChange={(event) => setPersonalityFilter(event.target.value)}>
            <option value="">all personality</option>
            <option value="friendly">friendly</option>
            <option value="pragmatic">pragmatic</option>
            <option value="none">none</option>
          </select>
          <select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
            <option value="">all status</option>
            <option value="running">running</option>
            <option value="completed">completed</option>
            <option value="failed">failed</option>
          </select>
          <select value={taskClassFilter} onChange={(event) => setTaskClassFilter(event.target.value)}>
            <option value="">all task classes</option>
            {taskClasses.map((taskClass) => (
              <option key={taskClass} value={taskClass}>
                {taskClass}
              </option>
            ))}
          </select>
        </div>
        <div className="table-wrap">
          <table className="ledger-table">
            <thead>
              <tr>
                <th>Run</th>
                <th>Model / Personality</th>
                <th>Status</th>
                <th>Task</th>
                <th>Signals</th>
                <th>Mechanism</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {runs.map((run) => (
                <tr key={run.run_id}>
                  <td>
                    <div className="cell-stack">
                      <Link to={`/runs/${run.run_id}`}>{run.instance_id}</Link>
                      <span className="mono-note">{run.cohort_id}</span>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <strong>{run.model}</strong>
                      <span>{run.personality_mode ?? "-"}</span>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <StatusBadge tone={run.status === "running" ? "running" : run.status === "completed" ? "completed" : run.status === "failed" ? "failed" : "neutral"}>{run.status}</StatusBadge>
                      <StatusBadge tone={run.grading_status.includes("resolved") ? "graded" : "neutral"}>{run.grading_status}</StatusBadge>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <span>{run.task_class}</span>
                      <span className="mono-note">{run.repo}</span>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <span>{formatCompact(run.visible_output_total_tokens_est)} visible tok</span>
                      <span>{run.tool_count} tools / {run.command_count} cmd</span>
                      <span>{run.patch_file_count} files patched</span>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <span>{run.verification_closure_count} verification closures</span>
                      <span>{run.personality_fallback_count} personality fallback</span>
                      <span>{summarizeMap(run.tool_name_counts, 2).map(([name, count]) => `${name}×${count}`).join(" · ") || "—"}</span>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <span>{formatDate(run.latest_updated_at)}</span>
                      <span className="mono-note">{Object.keys(run.message_category_counts).slice(0, 2).join(" / ") || "—"}</span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Panel>
    </div>
  );
}
