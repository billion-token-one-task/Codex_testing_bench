import { Link } from "react-router-dom";
import { useMemo, useState } from "react";

import { EventRail } from "../components/EventRail";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { RunCard } from "../components/RunCard";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { StateNotice } from "../components/StateNotice";
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
  const liveRunCards = useMemo(
    () => (liveRuns ?? []).slice(0, 6),
    [liveRuns],
  );
  const runStatusCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const run of runs) {
      counts[run.status] = (counts[run.status] ?? 0) + 1;
    }
    return counts;
  }, [runs]);
  const gradingStatusCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const run of runs) {
      counts[run.grading_status] = (counts[run.grading_status] ?? 0) + 1;
    }
    return counts;
  }, [runs]);
  const personalityCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const run of runs) {
      const key = run.personality_mode ?? "none";
      counts[key] = (counts[key] ?? 0) + 1;
    }
    return counts;
  }, [runs]);
  const benchmarkStatusRows = useMemo(
    () =>
      (data?.campaigns ?? [])
        .filter((campaign) => !benchmarkFilter || campaign.benchmark_name === benchmarkFilter)
        .slice(0, 8),
    [benchmarkFilter, data?.campaigns],
  );

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

      <div className="page-grid page-grid-2">
        <Panel title="Run Surface Summary" kicker="What this filtered ledger currently contains">
          {!runs.length ? (
            <StateNotice
              title="筛选后的 run surface 为空"
              body="这通常说明筛选条件太强，或者当前 experiment 还没有把对应 cohort / task class 的 run 写进 workspace 索引。"
              tone="info"
            />
          ) : null}
          <KeyValueGrid
            columns={4}
            items={[
              { label: "Solver Status", value: Object.entries(runStatusCounts).map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
              { label: "Grading Status", value: Object.entries(gradingStatusCounts).map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
              { label: "Personality Mix", value: Object.entries(personalityCounts).map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
              { label: "Live Intersection", value: Array.from(liveRunIds).filter((runId) => runs.some((run) => run.run_id === runId)).length, detail: `${liveRuns?.length ?? 0} global live runs`, tone: "signal" },
              { label: "Visible Total", value: formatCompact(runs.reduce((sum, run) => sum + run.visible_output_total_tokens_est, 0)), tone: "signal" },
              { label: "Tool Total", value: runs.reduce((sum, run) => sum + run.tool_count, 0), tone: "pressure" },
              { label: "Fallback Total", value: runs.reduce((sum, run) => sum + run.personality_fallback_count, 0), tone: "anomaly" },
              { label: "Friction Total", value: runs.reduce((sum, run) => sum + run.harness_friction_count, 0), tone: "anomaly" },
            ]}
          />
        </Panel>

        <Panel title="Live Run Intersection" kicker="Runs currently alive inside this filtered surface">
          {liveRunCards.length ? (
            <div className="run-card-grid-board run-card-grid-2">
              {liveRunCards.map((run) => {
                const indexedRun = runs.find((row) => row.run_id === run.run_id);
                if (indexedRun) {
                  return <RunCard key={`live-indexed-${run.run_id}`} run={indexedRun} compact />;
                }
                return (
                  <div key={`live-shadow-${run.run_id}`} className="compare-block">
                    <div className="compare-heading">{run.cohort_id}</div>
                    <div className="brief-meta">
                      <span>{run.instance_id}</span>
                      <span>{run.progress.current_phase}</span>
                    </div>
                    <div className="brief-meta">
                      <span>{run.model}</span>
                      <span>{run.personality_mode ?? "none"}</span>
                    </div>
                    <div className="signal-bar-stack">
                      <span className="mono-note">{run.current_focus ?? run.latest_message_preview ?? "live run"}</span>
                    </div>
                    <Link to={`/runs/${run.run_id}`} className="artifact-chip">
                      open war room
                    </Link>
                  </div>
                );
              })}
            </div>
          ) : (
            <StateNotice
              title="当前没有 live run 进入这个过滤视角"
              body="如果上方显示有全局 live run，但这里为空，说明这些活跃 run 不在当前筛选子集里。"
              tone="loading"
            />
          )}
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Recent Run Pulse" kicker="Latest structured run activity in this workspace">
          {!recentRunEvents.length ? (
            <StateNotice
              title="recent run pulse 还没有形成"
              body="如果 benchmark 刚刚启动，或者控制台刚接上 SSE，这里会短暂缺少近期事件。"
              tone="loading"
            />
          ) : (
            <EventRail rows={recentRunEvents} emptyLabel="等待 run pulse。" />
          )}
        </Panel>
        <Panel title="Benchmark Campaign Surface" kicker="How this filtered ledger maps back to benchmark batches">
          {!benchmarkStatusRows.length ? (
            <StateNotice
              title="当前过滤条件下没有可读的 benchmark campaign"
              body="这通常意味着 benchmark 过滤太强，或者该 benchmark 的 campaign 还没写进 workspace index。"
              tone="info"
            />
          ) : (
            <div className="campaign-pulse-stack">
              {benchmarkStatusRows.map((campaign) => (
                <div key={campaign.campaign_id} className="brief-card">
                  <div className="brief-head">
                    <strong>{campaign.experiment_name}</strong>
                    <StatusBadge tone={campaign.status.includes("running") ? "running" : campaign.status.includes("completed") || campaign.status.includes("graded") ? "completed" : "neutral"}>
                      {campaign.status}
                    </StatusBadge>
                  </div>
                  <div className="brief-meta">
                    <span>{campaign.benchmark_name}</span>
                    <span>{campaign.stage_name ?? "—"}</span>
                  </div>
                  <div className="brief-meta">
                    <span>{campaign.sample_size} tasks</span>
                    <span>{campaign.cohort_count} cohorts</span>
                  </div>
                  <div className="brief-meta">
                    <span>{campaign.active_run_count} active</span>
                    <span>{campaign.completed_run_count} completed</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>
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
        {!runs.length ? (
          <StateNotice
            title="当前筛选条件下没有 run"
            body="你可以先放宽 model / personality / benchmark / task class 过滤，或者等当前 campaign 再推进一段。"
            tone="info"
          />
        ) : null}
        {liveRuns?.length && !runs.some((run) => liveRunIds.has(run.run_id)) ? (
          <StateNotice
            title="有 live run，但它们被当前过滤条件排除了"
            body="这通常意味着你正在看某个子集，而真实运行中的 run 在另一个 model / personality / benchmark 维度里。"
            tone="warning"
          />
        ) : null}
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

      <Panel title="Campaign Coverage" kicker="Which benchmark / campaign surfaces are represented in this ledger">
        {!benchmarkStatusRows.length ? (
          <StateNotice
            title="当前没有 campaign coverage 可显示"
            body="等 workspace 索引和筛选条件对齐后，这里会展示当前 run ledger 覆盖了哪些 experiment / benchmark。"
            tone="info"
          />
        ) : null}
        <div className="table-wrap">
          <table className="ledger-table">
            <thead>
              <tr>
                <th>Campaign</th>
                <th>Benchmark</th>
                <th>Status</th>
                <th>Runs</th>
                <th>Signals</th>
              </tr>
            </thead>
            <tbody>
              {benchmarkStatusRows.map((campaign) => (
                <tr key={campaign.campaign_id}>
                  <td>
                    <div className="cell-stack">
                      <strong>{campaign.experiment_name}</strong>
                      <span className="mono-note">{campaign.campaign_id}</span>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <span>{campaign.benchmark_name}</span>
                      <span>{campaign.stage_name ?? "—"}</span>
                    </div>
                  </td>
                  <td>
                    <StatusBadge tone={campaign.status.includes("running") ? "running" : campaign.status.includes("graded") || campaign.status.includes("completed") ? "completed" : "neutral"}>
                      {campaign.status}
                    </StatusBadge>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <span>{campaign.completed_run_count} completed</span>
                      <span>{campaign.active_run_count} active</span>
                    </div>
                  </td>
                  <td>
                    <div className="cell-stack">
                      <span>{formatCompact(campaign.total_visible_output_tokens_est)} visible</span>
                      <span>{formatCompact(campaign.total_tool_calls)} tools</span>
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
