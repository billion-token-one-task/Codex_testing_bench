import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { ActionLauncher } from "../components/ActionLauncher";
import { EventRail } from "../components/EventRail";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { RunCard } from "../components/RunCard";
import { SignalBar } from "../components/SignalBar";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { StatusBadge } from "../components/StatusBadge";
import { api } from "../lib/api";
import { formatCompact, formatDate, formatDateFull, formatDurationMs, truncateMiddle } from "../lib/format";
import {
  readTableRows,
  useActiveRuns,
  useArtifactTail,
  useCampaignOperationalSummary,
  useEventStreamStatus,
  useLiveOverview,
  useLiveRuns,
  useProcessDetail,
  useProcesses,
  useRecentEventLines,
  useRecentEventTypes,
  useRunDetail,
  useRunEventBuckets,
  useRunOperationalSummary,
  useWorkspaceIndex,
} from "../lib/store";

export function LivePage() {
  const stream = useEventStreamStatus();
  const { data: liveOverview } = useLiveOverview();
  const { data: processes } = useProcesses();
  const { data: workspace } = useWorkspaceIndex();
  const { data: liveRuns } = useLiveRuns();
  const activeRuns = useActiveRuns(workspace ?? null);
  const recentLines = useRecentEventLines(140);
  const recentTools = useRecentEventTypes(["run.tool.appended", "run.command.appended"], 14);
  const recentPatches = useRecentEventTypes(["run.patch.appended"], 14);
  const recentMechanisms = useRecentEventTypes(
    [
      "run.personality.appended",
      "run.mechanism.appended",
      "run.skill.appended",
      "run.token.appended",
      "run.phase.changed",
      "run.focus.changed",
      "run.warning.appended",
    ],
    18,
  );
  const recentMessages = useRecentEventTypes(["run.message.appended"], 14);
  const [stopping, setStopping] = useState<string | null>(null);
  const [selectedCampaignReport, setSelectedCampaignReport] = useState<string | null>(null);
  const [selectedRunId, setSelectedRunId] = useState<string>("");
  const [selectedProcessId, setSelectedProcessId] = useState<string>("");

  const runningProcesses = useMemo(
    () => liveOverview?.running_processes ?? (processes ?? []).filter((process) => process.status === "running"),
    [liveOverview?.running_processes, processes],
  );
  const harnessRuns = liveOverview?.active_live_runs ?? liveRuns ?? [];
  const activeCampaign = liveOverview?.active_campaign ?? workspace?.campaigns?.find((campaign) => campaign.status === "running") ?? workspace?.campaigns?.[0] ?? null;
  const activeCampaignOperational = useCampaignOperationalSummary(activeCampaign?.campaign_id ?? "");
  const activeCampaignReports = activeCampaign?.report_paths?.map((path) => ({
    name: path.split("/").pop() ?? path,
    path,
    kind: "human_readable_dossier",
    exists: true,
    role: "report",
    scope: "campaign_reports",
    format: path.endsWith(".md") ? "markdown" : path.endsWith(".csv") ? "csv" : "text",
    previewable: true,
  })) ?? [];
  const selectedReportArtifact =
    activeCampaignReports.find((artifact) => artifact.path === selectedCampaignReport) ?? activeCampaignReports[0] ?? null;
  const livePhaseCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const run of harnessRuns) {
      counts[run.progress.current_phase] = (counts[run.progress.current_phase] ?? 0) + 1;
    }
    return counts;
  }, [harnessRuns]);
  const activeWarnings = activeCampaignOperational.data?.operational_warnings ?? [];
  const heatCounts = activeCampaignOperational.data?.heat_counts ?? {};
  const focusSamples = activeCampaignOperational.data?.focus_samples ?? [];
  const latestMessagePreviews = activeCampaignOperational.data?.latest_message_previews ?? [];
  const liveInstances = activeCampaignOperational.data?.active_instances ?? [];
  const activeCohorts = activeCampaignOperational.data?.active_cohorts ?? [];
  const campaignSolverCounts = activeCampaignOperational.data?.solver_status_counts ?? {};
  const spotlightRuns = useMemo(
    () => (harnessRuns.length > 0 ? harnessRuns.map((run) => run.run_id) : activeRuns.map((run) => run.run_id)),
    [activeRuns, harnessRuns],
  );
  const hottestRuns = liveOverview?.hottest_live_runs ?? [];
  const stalledRuns = liveOverview?.stalled_live_runs ?? [];
  const processDossiers = liveOverview?.process_dossiers ?? [];
  const latestGlobalWarnings = liveOverview?.latest_global_warnings ?? [];
  const latestGlobalMessagePreviews = liveOverview?.latest_global_message_previews ?? [];
  const latestGlobalFocusSamples = liveOverview?.latest_global_focus_samples ?? [];

  const selectedRun = useMemo(
    () => harnessRuns.find((run) => run.run_id === selectedRunId)
      ?? activeRuns.find((run) => run.run_id === selectedRunId)
      ?? null,
    [activeRuns, harnessRuns, selectedRunId],
  );
  const selectedRunOperational = useRunOperationalSummary(selectedRunId);
  const selectedRunDetail = useRunDetail(selectedRunId);
  const selectedRunEvents = useRunEventBuckets(selectedRunId, [
    "run.message.appended",
    "run.tool.appended",
    "run.command.appended",
    "run.patch.appended",
    "run.personality.appended",
    "run.mechanism.appended",
    "run.skill.appended",
    "run.token.appended",
    "run.phase.changed",
    "run.focus.changed",
    "run.warning.appended",
    "run.timeline.appended",
  ], 40);
  const selectedRunAttemptLogArtifact = useMemo(
    () => selectedRunDetail.data?.attempt_artifacts.find((artifact) => artifact.name === "attempt-log.txt") ?? null,
    [selectedRunDetail.data?.attempt_artifacts],
  );
  const selectedRunAttemptTail = useArtifactTail(selectedRunAttemptLogArtifact?.path ?? null, 80, Boolean(selectedRunAttemptLogArtifact));
  const selectedRunMessageRows = readTableRows(selectedRunDetail.data ?? null, "messageMetrics");
  const selectedRunToolRows = readTableRows(selectedRunDetail.data ?? null, "toolEvents");
  const selectedRunPatchRows = readTableRows(selectedRunDetail.data ?? null, "patchChain");
  const selectedProcess = useMemo(
    () => (processes ?? []).find((process) => process.id === selectedProcessId) ?? runningProcesses[0] ?? (processes ?? [])[0] ?? null,
    [processes, runningProcesses, selectedProcessId],
  );
  const selectedProcessDetail = useProcessDetail(selectedProcess?.id ?? "");

  const selectedRunEventCounts = useMemo(() => ({
    message: selectedRunEvents.filter((event) => event.type === "run.message.appended").length,
    tool: selectedRunEvents.filter((event) => event.type === "run.tool.appended" || event.type === "run.command.appended").length,
    patch: selectedRunEvents.filter((event) => event.type === "run.patch.appended").length,
    mechanism: selectedRunEvents.filter((event) => event.type === "run.personality.appended" || event.type === "run.skill.appended" || event.type === "run.token.appended").length,
  }), [selectedRunEvents]);
  useEffect(() => {
    if (!selectedRunId && spotlightRuns[0]) {
      setSelectedRunId(spotlightRuns[0]);
      return;
    }
    if (selectedRunId && spotlightRuns.length > 0 && !spotlightRuns.includes(selectedRunId)) {
      setSelectedRunId(spotlightRuns[0]);
    }
  }, [selectedRunId, spotlightRuns]);
  useEffect(() => {
    if (!selectedProcessId && (runningProcesses[0] ?? (processes ?? [])[0])) {
      setSelectedProcessId((runningProcesses[0] ?? (processes ?? [])[0])?.id ?? "");
      return;
    }
    if (selectedProcessId && (processes ?? []).length > 0 && !(processes ?? []).some((process) => process.id === selectedProcessId)) {
      setSelectedProcessId((runningProcesses[0] ?? (processes ?? [])[0])?.id ?? "");
    }
  }, [processes, runningProcesses, selectedProcessId]);

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
        kicker="Mission Control"
        title="Live"
        description="这里是实时战情室。看并行槽位、活跃 run、最新消息/工具/patch/机制事件，以及受管进程输出。"
        actions={<ActionLauncher />}
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Live Runs" value={harnessRuns.length || activeRuns.length} detail={`${workspace?.summary.active_run_count ?? 0} workspace active`} tone="signal" />
        <MetricCard label="Running Processes" value={runningProcesses.length} detail={`${processes?.length ?? 0} tracked`} tone="pressure" />
        <MetricCard label="Recent Console Lines" value={recentLines.length} detail="stdout / stderr / launcher output" tone="verify" />
        <MetricCard label="Observed Signal" value={formatCompact((liveOverview?.workspace ?? workspace)?.summary.total_tokens)} detail={`${formatCompact((liveOverview?.workspace ?? workspace)?.summary.total_visible_output_tokens_est)} visible`} />
      </div>

      <div className="live-layout">
        <div className="live-main-column">
          <Panel title="Mission Status Strip" kicker="Active campaign operational picture">
            <KeyValueGrid
              columns={5}
              items={[
                { label: "Campaign", value: activeCampaign?.campaign_id ?? "—", detail: activeCampaign?.experiment_name ?? "—" },
                { label: "Active Runs", value: harnessRuns.length || activeRuns.length, detail: `${activeCampaign?.active_run_count ?? 0} campaign active`, tone: "signal" },
                { label: "Completed", value: activeCampaign?.completed_run_count ?? 0, detail: `${activeCampaign?.failed_run_count ?? 0} failed`, tone: "verify" },
                { label: "Visible Tok", value: formatCompact(activeCampaign?.total_visible_output_tokens_est), detail: formatCompact(activeCampaign?.total_tokens), tone: "pressure" },
                { label: "Reports / Datasets", value: `${activeCampaign?.report_count ?? 0} / ${activeCampaign?.dataset_count ?? 0}`, detail: activeCampaign?.status ?? "—" },
                { label: "Live Focus", value: harnessRuns[0]?.current_focus ?? "—", detail: harnessRuns[0]?.activity_heat ?? "idle", tone: "authority" },
                { label: "Live Msg / Tool", value: `${harnessRuns.reduce((sum, run) => sum + run.progress.message_count, 0)} / ${harnessRuns.reduce((sum, run) => sum + run.progress.tool_count, 0)}`, detail: `${harnessRuns.reduce((sum, run) => sum + run.progress.command_count, 0)} cmd`, tone: "signal" },
                { label: "Live Warnings", value: activeCampaignOperational.data?.active_warning_count ?? harnessRuns.reduce((sum, run) => sum + run.warnings.length, 0), detail: activeWarnings[0] ?? harnessRuns.flatMap((run) => run.warnings).slice(0, 1)[0] ?? "none", tone: "anomaly" },
                { label: "Heat Mix", value: Object.entries(heatCounts).map(([name, count]) => `${name}×${count}`).join(" · ") || "—", detail: `${activeCampaignOperational.data?.stalled_live_run_count ?? 0} stalled` },
                { label: "Personality Fallback", value: activeCampaignOperational.data?.personality_fallback_live_count ?? 0, detail: `${activeCohorts.length} active cohorts`, tone: "anomaly" },
                { label: "Instances", value: liveInstances.length, detail: liveInstances.slice(0, 2).join(" · ") || "—" },
              ]}
            />
            {activeWarnings.length || harnessRuns.some((run) => run.warnings.length) ? (
              <>
                <div className="panel-divider" />
                <div className="warning-tape warning-tape-block">
                  {(activeWarnings.length ? activeWarnings : harnessRuns.flatMap((run) => run.warnings.slice(0, 2))).slice(0, 6).map((warning, index) => (
                    <span key={`${warning}-${index}`}>{warning}</span>
                  ))}
                </div>
              </>
            ) : null}
            {Object.keys(livePhaseCounts).length || focusSamples.length || latestGlobalFocusSamples.length ? (
              <div className="focus-grid">
                {Object.entries(livePhaseCounts).map(([phase, count]) => (
                  <div key={phase} className="focus-note">
                    <span className="metric-label">{phase}</span>
                    <strong>{count}</strong>
                  </div>
                ))}
                {(latestGlobalFocusSamples.length ? latestGlobalFocusSamples : focusSamples).slice(0, 4).map((focus) => (
                  <div key={focus} className="focus-note">
                    <span className="metric-label">focus</span>
                    <strong>{truncateMiddle(focus, 32)}</strong>
                  </div>
                ))}
              </div>
            ) : null}
          </Panel>

          <div className="page-grid page-grid-2">
            <Panel title="Control Plane Health" kicker="Transport / index / observer readiness">
              <KeyValueGrid
                columns={4}
                items={[
                  { label: "Stream", value: stream.status, detail: stream.lastEventAt ? formatDateFull(stream.lastEventAt) : "no live event yet", tone: stream.status === "connected" ? "signal" : stream.status === "degraded" ? "anomaly" : "neutral" },
                  { label: "Events Seen", value: stream.eventCount, detail: `${stream.errorCount} errors` },
                  { label: "Workspace Refresh", value: formatDateFull(workspace?.generated_at), detail: workspace?.repo_root ?? "—" },
                  { label: "Latest Process Output", value: formatDateFull(liveOverview?.latest_process_output_at), detail: `${runningProcesses.length} active processes` },
                  { label: "Active Campaign", value: activeCampaign?.campaign_id ?? "—", detail: activeCampaign?.experiment_name ?? "—" },
                  { label: "Warnings", value: latestGlobalWarnings.length, detail: latestGlobalWarnings[0] ?? "none", tone: latestGlobalWarnings.length ? "anomaly" : "neutral" },
                  { label: "Focus Samples", value: latestGlobalFocusSamples.length || focusSamples.length, detail: (latestGlobalFocusSamples[0] ?? focusSamples[0] ?? "—") },
                  { label: "Reports / Datasets", value: `${activeCampaign?.report_count ?? 0} / ${activeCampaign?.dataset_count ?? 0}`, detail: activeCampaign?.status ?? "—" },
                ]}
              />
            </Panel>
            <Panel title="Jump Desk" kicker="Fast paths into the current live investigation">
              <div className="artifact-list artifact-list-column artifact-ledger">
                {selectedRun ? (
                  <Link to={`/runs/${encodeURIComponent(selectedRun.run_id)}`} className="artifact-row">
                    <div className="artifact-row-main">
                      <strong>Open focused war room</strong>
                      <span className="artifact-role">{selectedRun.instance_id}</span>
                      <span className="artifact-scope">{selectedRun.cohort_id}</span>
                    </div>
                    <div className="artifact-row-meta">
                      <span>{("progress" in selectedRun ? selectedRun.progress.current_phase : selectedRun.status) ?? "—"}</span>
                    </div>
                  </Link>
                ) : null}
                {activeCampaign ? (
                  <Link to="/campaigns" className="artifact-row">
                    <div className="artifact-row-main">
                      <strong>Back to campaign desk</strong>
                      <span className="artifact-role">{activeCampaign.campaign_id}</span>
                      <span className="artifact-scope">{activeCampaign.experiment_name}</span>
                    </div>
                  </Link>
                ) : null}
                <Link to="/compare" className="artifact-row">
                  <div className="artifact-row-main">
                    <strong>Open compare workbench</strong>
                    <span className="artifact-role">2x2 matrix</span>
                    <span className="artifact-scope">model / personality deltas</span>
                  </div>
                </Link>
                <Link to="/artifacts" className="artifact-row">
                  <div className="artifact-row-main">
                    <strong>Open artifact archive</strong>
                    <span className="artifact-role">reports / datasets / raw truth</span>
                  </div>
                </Link>
              </div>
            </Panel>
          </div>

          <Panel title="Parallel Slots" kicker="Currently active runs">
            <div className="run-card-grid-board">
              {harnessRuns.length === 0 && activeRuns.length === 0 ? (
                <div className="empty-box">当前没有 live run。</div>
              ) : (
                (harnessRuns.length > 0 ? harnessRuns : activeRuns).map((run) =>
                  "progress" in run ? (
                    <div key={run.run_id} className="live-run-card-wrap">
                      <RunCard
                        run={{
                          campaign_id: run.campaign_id,
                          run_id: run.run_id,
                          manifest_run_id: run.run_id,
                          instance_id: run.instance_id,
                          repo: run.repo,
                          task_class: run.task_class,
                          cohort_id: run.cohort_id,
                          model: run.model,
                          provider: run.provider,
                          personality_mode: run.personality_mode,
                          prompt_style: null,
                          status: run.run_status,
                          grading_status: run.grading_status,
                          run_dir: "",
                          manifest_path: "",
                          latest_updated_at: run.last_event_at ?? null,
                          command_count: run.progress.command_count,
                          tool_count: run.progress.tool_count,
                          patch_file_count: run.progress.patch_event_count,
                          message_metric_count: run.progress.message_count,
                          visible_output_total_tokens_est: run.telemetry.visible_output_total_tokens_est,
                          total_tokens: run.telemetry.total_tokens,
                          anomaly_count: run.mechanism.harness_friction_count,
                          tool_kind_counts: {},
                          tool_name_counts: {},
                          tool_route_counts: {},
                          message_category_counts: {},
                          ignition_shell_search_count: 0,
                          verification_closure_count: run.progress.verification_event_count,
                          personality_fallback_count: run.mechanism.personality_fallback_count,
                          harness_friction_count: run.mechanism.harness_friction_count,
                          latest_attempt: null,
                        }}
                        selected={selectedRunId === run.run_id}
                        onSelect={() => setSelectedRunId(run.run_id)}
                      />
                      <KeyValueGrid
                        columns={4}
                        items={[
                          { label: "Phase", value: run.progress.current_phase, detail: run.run_status, tone: "authority" },
                          { label: "Elapsed", value: formatDurationMs(run.elapsed_ms), detail: run.last_event_at ? formatDate(run.last_event_at) : "—" },
                          { label: "Tools/min", value: run.telemetry.tool_bursts_per_minute.toFixed(1), detail: `${run.progress.tool_count} tools`, tone: "pressure" },
                          { label: "Tokens/min", value: formatCompact(run.telemetry.tokens_per_minute), detail: formatCompact(run.telemetry.total_tokens), tone: "signal" },
                          { label: "Heat", value: run.activity_heat, detail: run.current_focus ?? "—", tone: run.activity_heat === "hot" ? "pressure" : run.activity_heat === "stalled" ? "anomaly" : "signal" },
                          { label: "Msg Category", value: run.mechanism.last_message_category ?? "—", detail: run.mechanism.top_tool_route ?? "—" },
                        ]}
                      />
                      {run.warnings.length ? (
                        <div className="warning-tape">
                          {run.warnings.slice(0, 3).map((warning) => (
                            <span key={warning}>{warning}</span>
                          ))}
                        </div>
                      ) : null}
                    </div>
                  ) : (
                    <RunCard
                      key={run.run_id}
                      run={run}
                      selected={selectedRunId === run.run_id}
                      onSelect={() => setSelectedRunId(run.run_id)}
                    />
                  ),
                )
              )}
            </div>
          </Panel>

          <div className="page-grid page-grid-2">
            <Panel title="Hot Runs Board" kicker="Most active live runs right now">
              {hottestRuns.length ? (
                <div className="run-card-grid-board run-card-grid-2">
                  {hottestRuns.slice(0, 4).map((run) => (
                    <div key={`hot-${run.run_id}`} className="compare-block">
                      <div className="compare-heading">{run.cohort_id}</div>
                      <div className="brief-meta">
                        <span>{run.instance_id}</span>
                        <StatusBadge tone={run.activity_heat === "hot" ? "warning" : run.activity_heat === "stalled" ? "failed" : "running"}>
                          {run.activity_heat}
                        </StatusBadge>
                      </div>
                      <div className="signal-bar-stack">
                        <SignalBar label="visible" value={run.telemetry.visible_output_total_tokens_est} max={Math.max(...hottestRuns.map((row) => row.telemetry.visible_output_total_tokens_est), 1)} tone="signal" />
                        <SignalBar label="tools" value={run.progress.tool_count} max={Math.max(...hottestRuns.map((row) => row.progress.tool_count), 1)} tone="pressure" />
                        <SignalBar label="commands" value={run.progress.command_count} max={Math.max(...hottestRuns.map((row) => row.progress.command_count), 1)} tone="authority" />
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="empty-box">等待 live heat 排行。</div>
              )}
            </Panel>

            <Panel title="Stalled / Warning Board" kicker="Runs that may need operator attention">
              {stalledRuns.length || latestGlobalWarnings.length ? (
                <>
                  {stalledRuns.length ? (
                    <div className="warning-tape warning-tape-block">
                      {stalledRuns.slice(0, 6).map((run) => (
                        <span key={`stalled-${run.run_id}`}>{run.cohort_id} · {run.instance_id} · stalled</span>
                      ))}
                    </div>
                  ) : null}
                  {latestGlobalWarnings.length ? (
                    <div className="evidence-list">
                      {latestGlobalWarnings.slice(0, 8).map((warning) => (
                        <div key={warning}>{warning}</div>
                      ))}
                    </div>
                  ) : null}
                </>
              ) : (
                <div className="empty-box">当前没有 stalled run 或全局警告。</div>
              )}
            </Panel>
          </div>

          <Panel title="Focused Run Spotlight" kicker="Selected live run war-room preview">
            {selectedRun ? (
              <>
                <KeyValueGrid
                  columns={4}
                  items={[
                    { label: "Run", value: selectedRun.run_id, detail: selectedRun.instance_id },
                    { label: "Model / Tone", value: `${selectedRun.model} / ${selectedRun.personality_mode ?? "none"}`, detail: selectedRun.task_class },
                    { label: "Current Phase", value: "progress" in selectedRun ? selectedRun.progress.current_phase : selectedRun.status, detail: "progress" in selectedRun ? selectedRun.activity_heat : selectedRun.grading_status, tone: "authority" },
                    { label: "Status", value: "progress" in selectedRun ? selectedRun.run_status : selectedRun.status, detail: "progress" in selectedRun ? selectedRun.grading_status : selectedRun.grading_status },
                    { label: "Visible", value: formatCompact("progress" in selectedRun ? selectedRun.telemetry.visible_output_total_tokens_est : selectedRun.visible_output_total_tokens_est), detail: `${"progress" in selectedRun ? selectedRun.progress.message_count : selectedRun.message_metric_count} msgs`, tone: "signal" },
                    { label: "Tool / Cmd", value: `${"progress" in selectedRun ? selectedRun.progress.tool_count : selectedRun.tool_count} / ${"progress" in selectedRun ? selectedRun.progress.command_count : selectedRun.command_count}`, detail: `${"progress" in selectedRun ? selectedRun.progress.patch_event_count : selectedRun.patch_file_count} patch`, tone: "pressure" },
                    { label: "Latest Focus", value: "progress" in selectedRun ? selectedRun.current_focus ?? "—" : "—", detail: "progress" in selectedRun ? selectedRun.latest_tool ?? selectedRun.latest_command ?? "—" : "—" },
                    { label: "Operational Warnings", value: selectedRunOperational.data?.operational_warnings.length ?? ("progress" in selectedRun ? selectedRun.warnings.length : 0), detail: selectedRunOperational.data?.operational_warnings?.[0] ?? ("progress" in selectedRun ? selectedRun.warnings[0] ?? "none" : "none"), tone: "anomaly" },
                  ]}
                />
                {selectedRunOperational.data?.operational_warnings?.length ? (
                  <>
                    <div className="panel-divider" />
                    <div className="warning-tape warning-tape-block">
                      {selectedRunOperational.data.operational_warnings.map((warning) => (
                        <span key={warning}>{warning}</span>
                      ))}
                    </div>
                  </>
                ) : null}
                <div className="panel-divider" />
                <div className="page-grid page-grid-4">
                  <MetricCard label="Live Msg" value={selectedRunEventCounts.message} detail={`${selectedRunMessageRows.length} normalized rows`} tone="signal" />
                  <MetricCard label="Live Tool" value={selectedRunEventCounts.tool} detail={`${selectedRunToolRows.length} normalized rows`} tone="pressure" />
                  <MetricCard label="Live Patch" value={selectedRunEventCounts.patch} detail={`${selectedRunPatchRows.length} normalized rows`} tone="verify" />
                  <MetricCard label="Mechanism" value={selectedRunEventCounts.mechanism} detail={`${selectedRunOperational.data?.event_table_counts ? Object.keys(selectedRunOperational.data.event_table_counts).length : 0} event tables`} tone="anomaly" />
                </div>
                <div className="war-room-layout">
                  <div className="war-room-column">
                    <Panel title="Live Message Rail" kicker="Selected run visible output">
                      <EventRail rows={selectedRunEvents.filter((event) => event.type === "run.message.appended")} emptyLabel="等待该 run 的 live message。" />
                    </Panel>
                    <Panel title="Live Attempt Log Tail" kicker={selectedRunAttemptLogArtifact?.name ?? "attempt-log.txt"}>
                      <pre className="artifact-pre artifact-pre-medium">
                        {selectedRunAttemptTail.data?.lines.join("\n") ?? "等待 attempt log…"}
                      </pre>
                    </Panel>
                  </div>
                  <div className="war-room-column">
                    <Panel title="Live Tool / Command Rail" kicker="Concrete tooling cadence">
                      <EventRail rows={selectedRunEvents.filter((event) => event.type === "run.tool.appended" || event.type === "run.command.appended")} emptyLabel="等待该 run 的 tool / command 流。" />
                    </Panel>
                    <Panel title="Live Patch / Mechanism Rail" kicker="Patch chain + personality / skill / token pressure">
                      <EventRail rows={selectedRunEvents.filter((event) => event.type === "run.patch.appended" || event.type === "run.personality.appended" || event.type === "run.mechanism.appended" || event.type === "run.skill.appended" || event.type === "run.token.appended" || event.type === "run.phase.changed" || event.type === "run.focus.changed" || event.type === "run.warning.appended")} emptyLabel="等待 patch / mechanism 流。" />
                    </Panel>
                  </div>
                </div>
              </>
            ) : (
              <div className="empty-box">选择一个活跃 run，就能看到更像 war room 的 live 速览。</div>
            )}
          </Panel>

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
                      <td>
                        <StatusBadge tone={process.status === "running" ? "running" : process.status === "failed" ? "failed" : "neutral"}>
                          {process.status}
                        </StatusBadge>
                      </td>
                      <td>{formatDate(process.started_at)}</td>
                      <td className="mono-cell">{truncateMiddle(process.command.join(" "), 108)}</td>
                      <td className="mono-cell">{truncateMiddle(process.cwd, 42)}</td>
                      <td>
                        <div className="chip-row">
                          <button type="button" className="artifact-chip" onClick={() => setSelectedProcessId(process.id)}>
                            inspect
                          </button>
                          <button
                            type="button"
                            disabled={process.status !== "running" || stopping === process.id}
                            onClick={() => void stop(process.id)}
                          >
                            {stopping === process.id ? "Stopping…" : "Stop"}
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Panel>

          {selectedProcess && selectedProcessDetail.data ? (
            <Panel title="Process Dossier" kicker={selectedProcess.id}>
              <KeyValueGrid
                columns={4}
                items={[
                  { label: "Kind", value: selectedProcessDetail.data.snapshot.kind },
                  { label: "Status", value: selectedProcessDetail.data.snapshot.status, detail: selectedProcessDetail.data.snapshot.exit_code != null ? `exit ${selectedProcessDetail.data.snapshot.exit_code}` : "running" },
                  { label: "Started", value: formatDateFull(selectedProcessDetail.data.snapshot.started_at) },
                  { label: "Last Output", value: formatDateFull(selectedProcessDetail.data.snapshot.last_output_at), detail: selectedProcessDetail.data.snapshot.latest_line_preview ?? "—" },
                  { label: "Stdout", value: selectedProcessDetail.data.snapshot.stdout_line_count, tone: "signal" },
                  { label: "Stderr", value: selectedProcessDetail.data.snapshot.stderr_line_count, tone: "anomaly" },
                  { label: "Total Lines", value: selectedProcessDetail.data.snapshot.total_output_line_count, tone: "pressure" },
                  { label: "CWD", value: truncateMiddle(selectedProcessDetail.data.snapshot.cwd, 72) },
                ]}
              />
              <div className="panel-divider" />
              <div className="mono-note">{selectedProcessDetail.data.snapshot.command.join(" ")}</div>
              <div className="panel-divider" />
              <div className="process-output-ledger">
                {selectedProcessDetail.data.recent_output.length === 0 ? (
                  <div className="empty-box">这个进程还没有输出。</div>
                ) : (
                  selectedProcessDetail.data.recent_output.slice(-120).map((line, index) => (
                    <div key={`${line.timestamp}-${index}`} className={`process-output-row process-output-${line.stream}`}>
                      <span className="process-output-meta">{line.stream} · {formatDateFull(line.timestamp)}</span>
                      <code>{line.line}</code>
                    </div>
                  ))
                )}
              </div>
            </Panel>
          ) : null}

          {processDossiers.length ? (
            <Panel title="Top Process Dossiers" kicker="Most relevant managed processes">
              <div className="page-grid page-grid-2">
                {processDossiers.slice(0, 4).map((dossier) => (
                  <div key={dossier.snapshot.snapshot.id} className="compare-block">
                    <div className="compare-heading">{dossier.kind_group}</div>
                    <div className="brief-meta">
                      <span>{dossier.snapshot.snapshot.id}</span>
                      <StatusBadge tone={dossier.snapshot.snapshot.status === "running" ? "running" : "failed"}>
                        {dossier.snapshot.snapshot.status}
                      </StatusBadge>
                    </div>
                    <div className="metric-grid">
                      <div><span className="metric-label">stdout</span><strong>{dossier.snapshot.snapshot.stdout_line_count}</strong></div>
                      <div><span className="metric-label">stderr</span><strong>{dossier.snapshot.snapshot.stderr_line_count}</strong></div>
                      <div><span className="metric-label">last</span><strong>{formatDate(dossier.snapshot.snapshot.last_output_at)}</strong></div>
                      <div><span className="metric-label">cwd</span><strong>{truncateMiddle(dossier.snapshot.snapshot.cwd, 28)}</strong></div>
                    </div>
                    <pre className="artifact-pre artifact-pre-small">
                      {dossier.snapshot.recent_output.slice(-8).map((line) => `${line.timestamp.slice(11, 19)} ${line.stream} ${line.line}`).join("\n")}
                    </pre>
                  </div>
                ))}
              </div>
            </Panel>
          ) : null}

          <Panel title="Live Console" kicker="Streaming stdout / stderr">
            <div className="console-pane console-pane-tall">
              {recentLines.length === 0 ? (
                <div className="empty-box">等待新的 live console 输出。</div>
              ) : (
                recentLines.map((line) => (
                  <div key={line.id} className={`console-line console-${line.stream}`}>
                    <span>{line.timestamp.slice(11, 19)}</span>
                    <span>{line.stream.toUpperCase()}</span>
                    <span>{line.processId}</span>
                    <span>{line.line}</span>
                  </div>
                ))
              )}
            </div>
          </Panel>
        </div>

        <div className="live-side-column">
          <Panel title="Live Progress Pulse" kicker="Rate + heat + status proxy">
            <KeyValueGrid
              columns={2}
              items={[
                { label: "Messages", value: activeRuns.reduce((sum, run) => sum + run.message_metric_count, 0), tone: "signal" },
                { label: "Tools", value: activeRuns.reduce((sum, run) => sum + run.tool_count, 0), tone: "pressure" },
                { label: "Commands", value: activeRuns.reduce((sum, run) => sum + run.command_count, 0) },
                { label: "Patches", value: activeRuns.reduce((sum, run) => sum + run.patch_file_count, 0), tone: "verify" },
                { label: "Visible Tok", value: formatCompact(activeRuns.reduce((sum, run) => sum + run.visible_output_total_tokens_est, 0)) },
                { label: "Harness Friction", value: activeRuns.reduce((sum, run) => sum + run.harness_friction_count, 0), tone: "anomaly" },
                { label: "Heat Mix", value: Object.entries(heatCounts).map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                { label: "Solver Mix", value: Object.entries(campaignSolverCounts).map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
              ]}
            />
          </Panel>

          <Panel title="Campaign Pulse Rail" kicker="Live focus / latest previews / warnings">
            <KeyValueGrid
              columns={2}
              items={[
                { label: "Active Cohorts", value: activeCohorts.length, detail: activeCohorts.slice(0, 3).join(" · ") || "—" },
                { label: "Active Instances", value: liveInstances.length, detail: liveInstances.slice(0, 2).join(" · ") || "—" },
                { label: "Latest Activity", value: activeCampaignOperational.data?.latest_activity_at ? formatDate(activeCampaignOperational.data.latest_activity_at) : "—" },
                { label: "Dataset / Report", value: `${activeCampaignOperational.data?.latest_datasets.length ?? 0} / ${activeCampaignOperational.data?.latest_reports.length ?? 0}` },
              ]}
            />
            {(latestGlobalFocusSamples.length || focusSamples.length) ? (
              <>
                <div className="panel-divider" />
                <div className="evidence-list">
                  {(latestGlobalFocusSamples.length ? latestGlobalFocusSamples : focusSamples).slice(0, 5).map((focus) => (
                    <div key={focus}>{focus}</div>
                  ))}
                </div>
              </>
            ) : null}
            {(latestGlobalMessagePreviews.length || latestMessagePreviews.length) ? (
              <>
                <div className="panel-divider" />
                <div className="evidence-list">
                  {(latestGlobalMessagePreviews.length ? latestGlobalMessagePreviews : latestMessagePreviews).slice(0, 4).map((preview) => (
                    <div key={preview}>{truncateMiddle(preview, 120)}</div>
                  ))}
                </div>
              </>
            ) : null}
          </Panel>

          <Panel title="Latest Tool / Command Rail" kicker="Structured run events">
            <EventRail rows={recentTools} emptyLabel="等待最新 tool / command 事件。" />
          </Panel>

          <Panel title="Latest Message Rail" kicker="Visible output snippets">
            <EventRail rows={recentMessages} emptyLabel="等待新的可见输出。" />
          </Panel>

          <Panel title="Latest Patch Rail" kicker="Patch chain / apply activity">
            <EventRail rows={recentPatches} emptyLabel="等待 patch 事件。" />
          </Panel>

          <Panel title="Latest Mechanism Rail" kicker="Personality / skill / token pressure">
            <EventRail rows={recentMechanisms} emptyLabel="等待 mechanism 事件。" />
          </Panel>

          <Panel title="Latest Campaign Report Dock" kicker="Read the freshest generated dossier without leaving Live">
            {selectedReportArtifact ? (
              <>
                <div className="artifact-list artifact-list-column artifact-ledger">
                  {activeCampaignReports.map((artifact) => (
                    <button
                      key={artifact.path}
                      className={`artifact-chip${selectedReportArtifact.path === artifact.path ? " artifact-chip-active" : ""}`}
                      onClick={() => setSelectedCampaignReport(artifact.path)}
                    >
                      <span>{artifact.name}</span>
                      <span className="artifact-kind">{artifact.role}</span>
                    </button>
                  ))}
                </div>
                <ArtifactViewer artifact={selectedReportArtifact} />
              </>
            ) : (
              <div className="empty-box">当前 campaign 还没有生成 report。</div>
            )}
          </Panel>
        </div>
      </div>
    </div>
  );
}
