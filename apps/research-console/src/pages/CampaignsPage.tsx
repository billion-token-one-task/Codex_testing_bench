import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { EventRail } from "../components/EventRail";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { RunCard } from "../components/RunCard";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { SignalBar } from "../components/SignalBar";
import { StatusBadge } from "../components/StatusBadge";
import { api } from "../lib/api";
import { formatCompact, formatDate, summarizeMap, truncateMiddle } from "../lib/format";
import {
  useCampaignArtifacts,
  useCampaignDetail,
  useCampaignOperationalSummary,
  useCampaignSelection,
  useRecentEventTypes,
  useWorkspaceIndex,
} from "../lib/store";

export function CampaignsPage() {
  const { data, loading, error } = useWorkspaceIndex();
  const campaigns = data?.campaigns ?? [];
  const [selectedCampaignId, setSelectedCampaignId] = useState<string>("");
  const [artifactMode, setArtifactMode] = useState<"reports" | "datasets">("reports");
  const [selectedArtifactPath, setSelectedArtifactPath] = useState<string | null>(null);
  const [actionBusy, setActionBusy] = useState<string | null>(null);
  const [actionResult, setActionResult] = useState<string>("");
  const activeCampaign = useCampaignSelection(campaigns, selectedCampaignId);
  const campaignDetail = useCampaignDetail(activeCampaign?.campaign_id ?? "");
  const campaignOperational = useCampaignOperationalSummary(activeCampaign?.campaign_id ?? "");
  const activeArtifacts = useCampaignArtifacts(campaignDetail.data ?? null, artifactMode);
  const recentCampaignEvents = useRecentEventTypes(["campaign.updated", "campaign.artifact.updated", "workspace.updated"], 18);
  const recentRunEvents = useRecentEventTypes(["run.updated", "run.live.updated", "run.timeline.appended", "run.token.appended"], 18);

  useEffect(() => {
    if (!activeCampaign) return;
    setSelectedCampaignId(activeCampaign.campaign_id);
  }, [activeCampaign?.campaign_id]);

  useEffect(() => {
    setSelectedArtifactPath(activeArtifacts[0]?.path ?? null);
  }, [artifactMode, activeArtifacts]);

  const selectedArtifact =
    activeArtifacts.find((artifact) => artifact.path === selectedArtifactPath) ?? activeArtifacts[0] ?? null;
  const runs = campaignDetail.data?.runs ?? [];
  const runningRuns = runs.filter((run) => run.status === "running");
  const completedRuns = runs.filter((run) => run.status === "completed");
  const routeHighlights = summarizeMap(
    (campaignOperational.data?.tool_route_counts as Record<string, number> | undefined) ??
      runs.reduce<Record<string, number>>((acc, run) => {
        for (const [name, count] of Object.entries(run.tool_route_counts)) {
          acc[name] = (acc[name] ?? 0) + count;
        }
        return acc;
      }, {}),
    6,
  );
  const toolHighlights = summarizeMap(
    (campaignOperational.data?.tool_name_counts as Record<string, number> | undefined) ??
      runs.reduce<Record<string, number>>((acc, run) => {
        for (const [name, count] of Object.entries(run.tool_name_counts)) {
          acc[name] = (acc[name] ?? 0) + count;
        }
        return acc;
      }, {}),
    6,
  );
  const activeLiveRuns = campaignOperational.data?.active_live_runs ?? [];
  const solverStatusHighlights = summarizeMap(campaignOperational.data?.solver_status_counts, 6);
  const gradingStatusHighlights = summarizeMap(campaignOperational.data?.grading_status_counts, 6);
  const cohortHighlights = summarizeMap(campaignOperational.data?.cohort_counts, 6);
  const taskClassHighlights = summarizeMap(campaignOperational.data?.task_class_counts, 6);
  const personalityHighlights = summarizeMap(campaignOperational.data?.personality_counts, 6);

  const recentCampaignPulse = useMemo(
    () =>
      recentCampaignEvents.filter((event) => {
        const payload = event.payload as { campaign_id?: string; campaignId?: string };
        const eventCampaignId = payload.campaign_id ?? payload.campaignId;
        return !activeCampaign || !eventCampaignId || eventCampaignId === activeCampaign.campaign_id || event.type === "workspace.updated";
      }),
    [activeCampaign, recentCampaignEvents],
  );
  const recentCampaignRunPulse = useMemo(
    () =>
      recentRunEvents.filter((event) => {
        const payload = event.payload as { campaign_id?: string; campaignId?: string; cohortId?: string };
        const eventCampaignId = payload.campaign_id ?? payload.campaignId;
        return !activeCampaign || !eventCampaignId || eventCampaignId === activeCampaign.campaign_id;
      }),
    [activeCampaign, recentRunEvents],
  );

  const latestReportArtifacts = campaignOperational.data?.latest_reports ?? [];
  const latestDatasetArtifacts = campaignOperational.data?.latest_datasets ?? [];
  const maxCohortCount = Math.max(...cohortHighlights.map(([, count]) => count), 1);
  const maxTaskClassCount = Math.max(...taskClassHighlights.map(([, count]) => count), 1);
  const maxPersonalityCount = Math.max(...personalityHighlights.map(([, count]) => count), 1);

  const launchCampaignAction = async (kind: "bootstrap-local" | "run" | "grade" | "report") => {
    if (!activeCampaign) return;
    setActionBusy(kind);
    setActionResult("");
    try {
      const launched = await api.action(kind, { campaign_dir: activeCampaign.path });
      setActionResult(`${launched.kind} → ${launched.process_id}`);
    } catch (error) {
      setActionResult(String(error));
    } finally {
      setActionBusy(null);
    }
  };

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Campaign Control Room"
        title="Campaigns"
        description="这里是研究批次的总账面。左边看 experiment ledger，中间看当前 campaign 的 operational dossier，右边看 artifact / event pulse。"
      />

      <div className="page-grid page-grid-4">
        <MetricCard
          label="Indexed Campaigns"
          value={formatCompact(data?.summary.campaign_count)}
          detail={`${formatCompact(data?.summary.run_count)} runs total`}
          tone="signal"
        />
        <MetricCard
          label="Active Runs"
          value={formatCompact(data?.summary.active_run_count)}
          detail={`${formatCompact(data?.summary.completed_run_count)} completed`}
          tone="pressure"
        />
        <MetricCard
          label="Visible Output"
          value={formatCompact(data?.summary.total_visible_output_tokens_est)}
          detail={`${formatCompact(data?.summary.total_tokens)} total tokens`}
          tone="verify"
        />
        <MetricCard
          label="Tool Surface"
          value={formatCompact(data?.summary.total_tool_calls)}
          detail={`${formatCompact(data?.summary.total_commands)} commands`}
        />
      </div>

      <div className="campaign-layout">
        <Panel title="Campaign Ledger" kicker="Experiments / cohorts / grading surface">
          {loading ? <div className="empty-box">加载 workspace index…</div> : null}
          {error ? <div className="empty-box">{error}</div> : null}
          <div className="ledger-list">
            {campaigns.map((campaign) => (
              <button
                key={campaign.campaign_id}
                type="button"
                className={`ledger-card${activeCampaign?.campaign_id === campaign.campaign_id ? " ledger-card-active" : ""}`}
                onClick={() => setSelectedCampaignId(campaign.campaign_id)}
              >
                <div className="ledger-card-head">
                  <div>
                    <div className="mono-note">{campaign.campaign_id}</div>
                    <strong>{campaign.experiment_name}</strong>
                  </div>
                  <StatusBadge tone={campaign.status.includes("running") ? "running" : campaign.status.includes("graded") || campaign.status.includes("completed") ? "completed" : "neutral"}>
                    {campaign.status}
                  </StatusBadge>
                </div>
                <div className="brief-meta">
                  <span>{campaign.benchmark_name}</span>
                  <span>{campaign.stage_name ?? "—"}</span>
                </div>
                <div className="ledger-card-grid">
                  <div><span className="metric-label">Sample</span><strong>{campaign.sample_size}</strong></div>
                  <div><span className="metric-label">Cohorts</span><strong>{campaign.cohort_count}</strong></div>
                  <div><span className="metric-label">Active</span><strong>{campaign.active_run_count}</strong></div>
                  <div><span className="metric-label">Reports</span><strong>{campaign.report_count}</strong></div>
                </div>
              </button>
            ))}
          </div>
        </Panel>

        <div className="campaign-main-column">
          <Panel
            title="Operational Dossier"
            kicker={activeCampaign?.campaign_id ?? "Select a campaign"}
            actions={
              <SegmentedTabs
                items={[
                  { value: "reports", label: "Reports" },
                  { value: "datasets", label: "Datasets" },
                ]}
                value={artifactMode}
                onChange={(value) => setArtifactMode(value as "reports" | "datasets")}
              />
            }
          >
            {activeCampaign ? (
              <>
                <KeyValueGrid
                  columns={5}
                  items={[
                    { label: "Benchmark", value: activeCampaign.benchmark_name, detail: activeCampaign.stage_name ?? "—" },
                    { label: "Sample", value: activeCampaign.sample_size, detail: `${activeCampaign.selected_instances} selected` },
                    { label: "Cohorts", value: activeCampaign.cohort_count, detail: `parallel ${activeCampaign.max_parallel_runs}` },
                    { label: "Visible", value: formatCompact(activeCampaign.total_visible_output_tokens_est), detail: `${formatCompact(activeCampaign.total_tokens)} total`, tone: "signal" },
                    { label: "Tool / Cmd", value: formatCompact(activeCampaign.total_tool_calls), detail: `${formatCompact(activeCampaign.total_commands)} commands`, tone: "pressure" },
                    { label: "Live Visible", value: formatCompact(campaignOperational.data?.live_visible_output_total_tokens_est), detail: `${formatCompact(campaignOperational.data?.live_total_tokens)} live tokens`, tone: "signal" },
                    { label: "Live Msg / Tool", value: `${campaignOperational.data?.live_message_count ?? 0} / ${campaignOperational.data?.live_tool_count ?? 0}`, detail: `${campaignOperational.data?.live_command_count ?? 0} commands`, tone: "pressure" },
                    { label: "Processes", value: campaignOperational.data?.active_process_count ?? 0, detail: campaignOperational.data?.latest_activity_at ? formatDate(campaignOperational.data.latest_activity_at) : "—" },
                    { label: "Active Cohorts", value: campaignOperational.data?.active_cohorts.join(" · ") || "—" },
                    { label: "Active Tasks", value: campaignOperational.data?.active_instances.slice(0, 3).join(" · ") || "—" },
                  ]}
                />
                <div className="panel-divider" />
                {campaignOperational.data?.operational_warnings?.length ? (
                  <>
                    <div className="warning-tape warning-tape-block">
                      {campaignOperational.data.operational_warnings.map((warning) => (
                        <span key={warning}>{warning}</span>
                      ))}
                    </div>
                    <div className="panel-divider" />
                  </>
                ) : null}
                <div className="chip-row">
                  {(["bootstrap-local", "run", "grade", "report"] as const).map((kind) => (
                    <button
                      key={kind}
                      type="button"
                      className="artifact-chip"
                      disabled={actionBusy !== null}
                      onClick={() => void launchCampaignAction(kind)}
                    >
                      {actionBusy === kind ? `${kind}…` : kind}
                    </button>
                  ))}
                  {actionResult ? <span className="mono-note">{actionResult}</span> : null}
                </div>
                <div className="page-grid page-grid-2">
                  <Panel title="Run Surface" kicker="Current cohort pulse">
                    <div className="split-metrics">
                      <div>
                        <div className="section-label">Running Cohorts</div>
                        {activeLiveRuns.length === 0 && runningRuns.length === 0 ? (
                          <div className="empty-box">当前没有 active run。</div>
                        ) : (
                          <div className="run-card-grid-board run-card-grid-2">
                            {(activeLiveRuns.length > 0 ? activeLiveRuns : runningRuns.slice(0, 4)).slice(0, 4).map((run) =>
                              "progress" in run ? (
                                <div key={run.run_id} className="brief-card">
                                  <div className="brief-head">
                                    <strong>{run.instance_id}</strong>
                                    <StatusBadge tone="running">{run.progress.current_phase}</StatusBadge>
                                  </div>
                                  <div className="brief-meta">
                                    <span>{run.model}</span>
                                    <span>{run.personality_mode ?? "none"}</span>
                                    <span>{run.task_class}</span>
                                  </div>
                                  <div className="ledger-card-grid">
                                    <div><span className="metric-label">Tokens</span><strong>{formatCompact(run.telemetry.total_tokens)}</strong></div>
                                    <div><span className="metric-label">Tools</span><strong>{run.progress.tool_count}</strong></div>
                                    <div><span className="metric-label">Msgs</span><strong>{run.progress.message_count}</strong></div>
                                    <div><span className="metric-label">Cmd</span><strong>{run.progress.command_count}</strong></div>
                                  </div>
                                  <div className="brief-meta">
                                    <span>{run.activity_heat}</span>
                                    <span>{truncateMiddle(run.current_focus ?? "—", 42)}</span>
                                  </div>
                                  {run.warnings.length ? (
                                    <div className="warning-tape">
                                      {run.warnings.slice(0, 2).map((warning) => (
                                        <span key={warning}>{warning}</span>
                                      ))}
                                    </div>
                                  ) : null}
                                </div>
                              ) : (
                                <RunCard key={run.run_id} run={run} compact />
                              ),
                            )}
                          </div>
                        )}
                      </div>
                      <div>
                        <div className="section-label">Completed Surface</div>
                        <ul className="evidence-list">
                          <li>{completedRuns.length} completed runs</li>
                          <li>{runs.filter((run) => run.grading_status.includes("resolved")).length} resolved grading rows</li>
                          <li>{runs.filter((run) => run.grading_status.includes("failed") || run.grading_status.includes("error")).length} grading failures</li>
                        </ul>
                      </div>
                    </div>
                  </Panel>
                  <Panel title="Mechanism Highlights" kicker="Top tools / routes / signals">
                    <div className="split-metrics">
                      <div>
                        <div className="section-label">Top Tools</div>
                        <ul className="evidence-list">
                          {toolHighlights.map(([name, count]) => (
                            <li key={name}>{name} × {count}</li>
                          ))}
                        </ul>
                      </div>
                      <div>
                        <div className="section-label">Top Routes</div>
                        <ul className="evidence-list">
                          {routeHighlights.map(([name, count]) => (
                            <li key={name}>{name} × {count}</li>
                          ))}
                        </ul>
                      </div>
                    </div>
                    <div className="panel-divider" />
                    <div className="split-metrics">
                      <div>
                        <div className="section-label">Solver Status</div>
                        <ul className="evidence-list">
                          {solverStatusHighlights.map(([name, count]) => (
                            <li key={name}>{name} × {count}</li>
                          ))}
                        </ul>
                      </div>
                      <div>
                        <div className="section-label">Grading Status</div>
                        <ul className="evidence-list">
                          {gradingStatusHighlights.map(([name, count]) => (
                            <li key={name}>{name} × {count}</li>
                          ))}
                        </ul>
                      </div>
                    </div>
                  </Panel>
                  <Panel title="Active War Rooms" kicker="Jump directly into the live runs that matter">
                    {activeLiveRuns.length === 0 ? (
                      <div className="empty-box">当前没有可直接跳转的 live war room。</div>
                    ) : (
                      <div className="artifact-list artifact-list-column artifact-ledger">
                        {activeLiveRuns.slice(0, 8).map((run) => (
                          <Link key={run.run_id} to={`/runs/${encodeURIComponent(run.run_id)}`} className="artifact-row">
                            <div className="artifact-row-main">
                              <strong>{run.instance_id}</strong>
                              <span className="artifact-role">{run.cohort_id}</span>
                              <span className="artifact-scope">{run.task_class}</span>
                            </div>
                            <div className="artifact-row-meta">
                              <StatusBadge tone={run.activity_heat === "hot" ? "warning" : run.activity_heat === "stalled" ? "failed" : "running"}>
                                {run.progress.current_phase}
                              </StatusBadge>
                              <span>{formatCompact(run.telemetry.total_tokens)} tok</span>
                              <span>{run.progress.tool_count} tools</span>
                            </div>
                          </Link>
                        ))}
                      </div>
                    )}
                  </Panel>
                </div>
                <div className="page-grid page-grid-3">
                  <Panel title="Cohort Status Matrix" kicker="How this experiment is distributed right now">
                    <div className="signal-bar-stack">
                      {cohortHighlights.map(([name, count]) => (
                        <SignalBar key={name} label={name} value={count} max={maxCohortCount} tone="signal" />
                      ))}
                    </div>
                  </Panel>
                  <Panel title="Task-class Spread" kicker="Which task classes dominate this campaign">
                    <div className="signal-bar-stack">
                      {taskClassHighlights.map(([name, count]) => (
                        <SignalBar key={name} label={name} value={count} max={maxTaskClassCount} tone="pressure" />
                      ))}
                    </div>
                  </Panel>
                  <Panel title="Personality Spread" kicker="Model tone surface across runs">
                    <div className="signal-bar-stack">
                      {personalityHighlights.map(([name, count]) => (
                        <SignalBar key={name} label={name} value={count} max={maxPersonalityCount} tone="verify" />
                      ))}
                    </div>
                  </Panel>
                </div>
                <div className="artifact-browser artifact-browser-tall">
                  <div className="artifact-list artifact-list-column artifact-ledger">
                    {activeArtifacts.map((artifact) => (
                      <button
                        key={artifact.path}
                        className={`artifact-chip${selectedArtifact?.path === artifact.path ? " artifact-chip-active" : ""}`}
                        onClick={() => setSelectedArtifactPath(artifact.path)}
                      >
                        <span>{artifact.name}</span>
                        <span className="artifact-kind">{artifact.kind}</span>
                      </button>
                    ))}
                  </div>
                  <ArtifactViewer artifact={selectedArtifact} />
                </div>
              </>
            ) : (
              <div className="empty-box">选择一个 campaign 查看 dossier。</div>
            )}
          </Panel>
        </div>

        <div className="campaign-side-column">
          <Panel title="Campaign Pulse Rail" kicker="Recent campaign + workspace events">
            <EventRail rows={recentCampaignPulse} emptyLabel="等待 campaign / artifact 事件。" />
          </Panel>
          <Panel title="Run Pulse Rail" kicker="Recent run updates in this workspace">
            <EventRail rows={recentCampaignRunPulse} emptyLabel="等待 run 更新。" />
          </Panel>
          {activeCampaign ? (
            <Panel title="Selected Campaign Metadata" kicker="Filesystem / identity">
              <KeyValueGrid
                columns={2}
                items={[
                  { label: "Path", value: truncateMiddle(activeCampaign.path, 80) },
                  { label: "Created", value: formatDate(activeCampaign.created_at) },
                  { label: "Reports", value: activeCampaign.report_count },
                  { label: "Datasets", value: activeCampaign.dataset_count },
                  { label: "Failed Runs", value: activeCampaign.failed_run_count, tone: activeCampaign.failed_run_count ? "anomaly" : "neutral" },
                  { label: "Active Runs", value: activeCampaign.active_run_count, tone: activeCampaign.active_run_count ? "pressure" : "neutral" },
                  { label: "Infra Failures", value: campaignOperational.data?.unresolved_infra_failure_count ?? 0, tone: (campaignOperational.data?.unresolved_infra_failure_count ?? 0) > 0 ? "anomaly" : "neutral" },
                  { label: "Task Classes", value: taskClassHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                  { label: "Cohorts", value: cohortHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                  { label: "Personality", value: personalityHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                ]}
              />
            </Panel>
          ) : null}
          {activeCampaign ? (
            <Panel title="Latest Deliverables" kicker="Newest reports / datasets">
              <div className="split-metrics">
                <div>
                  <div className="section-label">Reports</div>
                  <ul className="evidence-list">
                    {(latestReportArtifacts.length ? latestReportArtifacts : activeArtifacts.slice(0, 6)).slice(0, 6).map((artifact) => (
                      <li key={artifact.path}>
                        <button
                          type="button"
                          className="artifact-inline-link"
                          onClick={() => {
                            setArtifactMode("reports");
                            setSelectedArtifactPath(artifact.path);
                          }}
                        >
                          {artifact.name}
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>
                <div>
                  <div className="section-label">Datasets</div>
                  <ul className="evidence-list">
                    {(latestDatasetArtifacts.length ? latestDatasetArtifacts : (campaignDetail.data?.datasets ?? []).slice(0, 6)).slice(0, 6).map((artifact) => (
                      <li key={artifact.path}>
                        <button
                          type="button"
                          className="artifact-inline-link"
                          onClick={() => {
                            setArtifactMode("datasets");
                            setSelectedArtifactPath(artifact.path);
                          }}
                        >
                          {artifact.name}
                        </button>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            </Panel>
          ) : null}
        </div>
      </div>
    </div>
  );
}
