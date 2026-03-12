import { Link } from "react-router-dom";
import { useMemo, useState } from "react";

import { EventRail } from "../components/EventRail";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { RunCard } from "../components/RunCard";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { StatusBadge } from "../components/StatusBadge";
import { formatCompact, formatDate, summarizeMap, uniqueValues } from "../lib/format";
import { useLiveRuns, useRecentEventTypes, useWorkspaceIndex } from "../lib/store";

type ViewMode = "ledger" | "cards" | "cohort" | "task";

export function RunsPage() {
  const { data } = useWorkspaceIndex();
  const { data: liveRuns } = useLiveRuns();
  const [modelFilter, setModelFilter] = useState("");
  const [personalityFilter, setPersonalityFilter] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [gradingFilter, setGradingFilter] = useState("");
  const [taskClassFilter, setTaskClassFilter] = useState("");
  const [benchmarkFilter, setBenchmarkFilter] = useState("");
  const [frictionOnly, setFrictionOnly] = useState(false);
  const [fallbackOnly, setFallbackOnly] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>("ledger");
  const recentRunEvents = useRecentEventTypes(["run.updated", "run.live.updated", "run.token.appended", "run.tool.appended", "run.patch.appended"], 14);

  const benchmarks = uniqueValues((data?.campaigns ?? []).map((campaign) => campaign.benchmark_name));
  const taskClasses = uniqueValues((data?.runs ?? []).map((run) => run.task_class));
  const runs = useMemo(() => {
    return (data?.runs ?? []).filter((run) => {
      if (modelFilter && !run.model.includes(modelFilter)) return false;
      if (personalityFilter && (run.personality_mode ?? "") !== personalityFilter) return false;
      if (statusFilter && run.status !== statusFilter) return false;
      if (gradingFilter && !run.grading_status.includes(gradingFilter)) return false;
      if (taskClassFilter && run.task_class !== taskClassFilter) return false;
      if (frictionOnly && run.harness_friction_count === 0) return false;
      if (fallbackOnly && run.personality_fallback_count === 0) return false;
      if (benchmarkFilter) {
        const campaign = data?.campaigns.find((item) => item.campaign_id === run.campaign_id);
        if (!campaign || campaign.benchmark_name !== benchmarkFilter) return false;
      }
      return true;
    });
  }, [benchmarkFilter, data?.campaigns, data?.runs, fallbackOnly, frictionOnly, gradingFilter, modelFilter, personalityFilter, statusFilter, taskClassFilter]);

  const groupedByCohort = useMemo(() => {
    return runs.reduce<Record<string, typeof runs>>((acc, run) => {
      acc[run.cohort_id] ??= [];
      acc[run.cohort_id].push(run);
      return acc;
    }, {});
  }, [runs]);
  const groupedByTask = useMemo(() => {
    return runs.reduce<Record<string, typeof runs>>((acc, run) => {
      acc[run.instance_id] ??= [];
      acc[run.instance_id].push(run);
      return acc;
    }, {});
  }, [runs]);
  const liveRunIds = new Set((liveRuns ?? []).map((run) => run.run_id));

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Run Ledger"
        title="Runs"
        description="按 model、personality、benchmark、task class、solver/grading 状态筛选 run；再切换 ledger、cards 或 cohort grouping 视图。"
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Filtered Runs" value={runs.length} detail={`${data?.runs.length ?? 0} total indexed`} />
        <MetricCard label="Visible Output" value={formatCompact(runs.reduce((sum, run) => sum + run.visible_output_total_tokens_est, 0))} detail="estimated visible tokens" tone="signal" />
        <MetricCard label="Tool Surface" value={formatCompact(runs.reduce((sum, run) => sum + run.tool_count, 0))} detail={`${formatCompact(runs.reduce((sum, run) => sum + run.command_count, 0))} commands`} tone="pressure" />
        <MetricCard label="Verification / Friction" value={`${runs.reduce((sum, run) => sum + run.verification_closure_count, 0)} / ${runs.reduce((sum, run) => sum + run.harness_friction_count, 0)}`} detail="closure vs friction" tone="verify" />
      </div>

      <Panel
        title="Run Filters"
        kicker="Search by model / personality / benchmark / task class / status"
        actions={
          <SegmentedTabs
            items={[
              { value: "ledger", label: "Ledger" },
              { value: "cards", label: "Cards" },
              { value: "cohort", label: "Cohort Groups" },
              { value: "task", label: "Task Groups" },
            ]}
            value={viewMode}
            onChange={(value) => setViewMode(value as ViewMode)}
          />
        }
      >
        <div className="filter-row filter-row-wide">
          <input value={modelFilter} onChange={(event) => setModelFilter(event.target.value)} placeholder="model contains…" />
          <select value={personalityFilter} onChange={(event) => setPersonalityFilter(event.target.value)}>
            <option value="">all personality</option>
            <option value="friendly">friendly</option>
            <option value="pragmatic">pragmatic</option>
            <option value="none">none</option>
          </select>
          <select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
            <option value="">all solver status</option>
            <option value="running">running</option>
            <option value="completed">completed</option>
            <option value="failed">failed</option>
          </select>
          <select value={gradingFilter} onChange={(event) => setGradingFilter(event.target.value)}>
            <option value="">all grading status</option>
            <option value="resolved">resolved</option>
            <option value="unresolved">unresolved</option>
            <option value="failed">failed</option>
            <option value="pending">pending</option>
          </select>
          <select value={taskClassFilter} onChange={(event) => setTaskClassFilter(event.target.value)}>
            <option value="">all task classes</option>
            {taskClasses.map((taskClass) => (
              <option key={taskClass} value={taskClass}>
                {taskClass}
              </option>
            ))}
          </select>
          <select value={benchmarkFilter} onChange={(event) => setBenchmarkFilter(event.target.value)}>
            <option value="">all benchmarks</option>
            {benchmarks.map((benchmark) => (
              <option key={benchmark} value={benchmark}>
                {benchmark}
              </option>
            ))}
          </select>
          <label className="checkbox-chip">
            <input type="checkbox" checked={frictionOnly} onChange={(event) => setFrictionOnly(event.target.checked)} />
            friction only
          </label>
          <label className="checkbox-chip">
            <input type="checkbox" checked={fallbackOnly} onChange={(event) => setFallbackOnly(event.target.checked)} />
            fallback only
          </label>
        </div>
      </Panel>

      {viewMode === "ledger" ? (
        <Panel title="Run Ledger" kicker="Dense operational ledger">
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
                  <th>Live Focus</th>
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
                        <StatusBadge tone={run.status === "running" ? "running" : run.status === "completed" ? "completed" : run.status === "failed" ? "failed" : "neutral"}>
                          {run.status}
                        </StatusBadge>
                        {liveRunIds.has(run.run_id) ? <StatusBadge tone="running">live</StatusBadge> : null}
                        <StatusBadge tone={run.grading_status.includes("resolved") ? "graded" : run.grading_status.includes("failed") ? "failed" : "neutral"}>
                          {run.grading_status}
                        </StatusBadge>
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
                        {liveRunIds.has(run.run_id) ? <StatusBadge tone="running">active</StatusBadge> : <span>—</span>}
                        <span className="mono-note">{summarizeMap(run.tool_route_counts, 2).map(([name, count]) => `${name}×${count}`).join(" · ") || "no route pulse"}</span>
                        <span>{run.harness_friction_count ? `friction ${run.harness_friction_count}` : run.personality_fallback_count ? `fallback ${run.personality_fallback_count}` : "steady"}</span>
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
      ) : null}

      {viewMode === "cards" ? (
        <div className="run-card-grid-board">
          {runs.map((run) => (
            <RunCard key={run.run_id} run={run} />
          ))}
        </div>
      ) : null}

      {viewMode === "cohort" ? (
        <div className="page-grid">
          {Object.entries(groupedByCohort).map(([cohortId, cohortRuns]) => (
            <Panel key={cohortId} title={cohortId} kicker={`${cohortRuns.length} runs`}>
              <KeyValueGrid
                columns={4}
                items={[
                  { label: "Runs", value: cohortRuns.length },
                  { label: "Visible", value: formatCompact(cohortRuns.reduce((sum, run) => sum + run.visible_output_total_tokens_est, 0)), tone: "signal" },
                  { label: "Tools", value: cohortRuns.reduce((sum, run) => sum + run.tool_count, 0), tone: "pressure" },
                  { label: "Friction", value: cohortRuns.reduce((sum, run) => sum + run.harness_friction_count, 0), tone: "anomaly" },
                ]}
              />
              <div className="run-card-grid-board">
                {cohortRuns.map((run) => (
                  <RunCard key={run.run_id} run={run} compact />
                ))}
              </div>
            </Panel>
          ))}
        </div>
      ) : null}

      {viewMode === "task" ? (
        <div className="page-grid">
          {Object.entries(groupedByTask).map(([instanceId, taskRuns]) => (
            <Panel key={instanceId} title={instanceId} kicker={`${taskRuns.length} cohort rows`}>
              <KeyValueGrid
                columns={4}
                items={[
                  { label: "Live", value: taskRuns.filter((run) => liveRunIds.has(run.run_id)).length, tone: "pressure" },
                  { label: "Visible", value: formatCompact(taskRuns.reduce((sum, run) => sum + run.visible_output_total_tokens_est, 0)), tone: "signal" },
                  { label: "Tools", value: taskRuns.reduce((sum, run) => sum + run.tool_count, 0) },
                  { label: "Fallback", value: taskRuns.reduce((sum, run) => sum + run.personality_fallback_count, 0), tone: "anomaly" },
                ]}
              />
              <div className="run-card-grid-board run-card-grid-2">
                {taskRuns.map((run) => (
                  <RunCard key={run.run_id} run={run} compact />
                ))}
              </div>
            </Panel>
          ))}
        </div>
      ) : null}

      <Panel title="Run Pulse Rail" kicker="Latest live run events across the filtered surface">
        <EventRail rows={recentRunEvents} emptyLabel="等待更多 run 级动态事件。" />
      </Panel>
    </div>
  );
}
