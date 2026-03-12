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
import { StateNotice } from "../components/StateNotice";
import { StatusBadge } from "../components/StatusBadge";
import { api } from "../lib/api";
import { formatCompact, formatDate, formatDateFull, formatDurationMs, truncateMiddle } from "../lib/format";
import {
  readTableRows,
  useActiveRuns,
  useArtifactTail,
  useCampaignDetail,
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
import type { RunIndexEntry } from "../lib/types";

export function LivePage() {
  const stream = useEventStreamStatus();
  const liveOverviewResource = useLiveOverview();
  const processResource = useProcesses();
  const workspaceResource = useWorkspaceIndex();
  const liveRunsResource = useLiveRuns();
  const liveOverview = liveOverviewResource.data;
  const processes = processResource.data;
  const workspace = liveOverview?.workspace ?? workspaceResource.data ?? null;
  const liveRuns = liveRunsResource.data;
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
  const currentCampaignLiveRuns = liveOverview?.current_campaign_live_runs ?? [];
  const otherLiveRuns = liveOverview?.other_live_runs ?? [];
  const activeCampaign = liveOverview?.active_campaign ?? workspace?.campaigns?.find((campaign) => campaign.status === "running") ?? workspace?.campaigns?.[0] ?? null;
  const activeCampaignOperational = useCampaignOperationalSummary(activeCampaign?.campaign_id ?? "");
  const activeCampaignDetail = useCampaignDetail(activeCampaign?.campaign_id ?? "");
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
  const operatorNotices = liveOverview?.operator_notices ?? [];
  const activeCampaignRuns = activeCampaignDetail.data?.runs ?? [];
  const queuedCampaignRuns = activeCampaignRuns.filter((run: RunIndexEntry) => run.status !== "completed").slice(0, 8);
  const isHydrating = (workspaceResource.loading || liveOverviewResource.loading) && !workspace;

  const selectedRun = useMemo(
    () => currentCampaignLiveRuns.find((run) => run.run_id === selectedRunId)
      ?? harnessRuns.find((run) => run.run_id === selectedRunId)
      ?? activeRuns.find((run) => run.run_id === selectedRunId)
      ?? null,
    [activeRuns, currentCampaignLiveRuns, harnessRuns, selectedRunId],
  );
  const selectedLiveRun = useMemo(
    () => (selectedRun && "progress" in selectedRun ? selectedRun : null),
    [selectedRun],
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

      {isHydrating ? (
        <StateNotice
          title="控制台正在水合 live 工作区"
          body="当前页面会先建立 workspace 索引、campaign 摘要和 SSE 总线，再把实时 run / process / artifact 事件挂上来。短暂出现的空值不代表 harness 停止。"
          tone="loading"
        />
      ) : null}

      {operatorNotices.length ? (
        <StateNotice
          title="控制台发现了需要解释的现场状态"
          body={
            <div className="dense-ledger">
              {operatorNotices.map((notice) => (
                <div key={notice}>{notice}</div>
              ))}
            </div>
          }
          tone="warning"
        />
      ) : null}

      {!operatorNotices.length && activeCampaign && !currentCampaignLiveRuns.length && (activeCampaign.active_run_count > 0 || queuedCampaignRuns.length > 0) ? (
        <StateNotice
          title="当前 campaign 还有待推进 run，但 live rail 暂时没有挂上"
          body={`当前主战场 manifest 仍显示 ${activeCampaign.active_run_count} 个 active run；这通常意味着 live snapshot 还没追上，或这些 run 是由外部 launcher / 旧 control plane 启动。下方会优先展示 campaign 队列和最近的 dossier。`}
          tone="info"
        />
      ) : null}

      <div className="live-layout">
        <div className="live-main-column">
          <Panel title="Mission Status Strip" kicker="Active campaign operational picture">
            <KeyValueGrid
              columns={5}
              items={[
                { label: "Campaign", value: activeCampaign?.campaign_id ?? "—", detail: activeCampaign?.experiment_name ?? "等待识别当前主战场" },
                { label: "Active Runs", value: currentCampaignLiveRuns.length || harnessRuns.length || activeRuns.length, detail: `${activeCampaign?.active_run_count ?? 0} campaign active`, tone: "signal" },
                { label: "Completed", value: activeCampaign?.completed_run_count ?? 0, detail: `${activeCampaign?.failed_run_count ?? 0} failed`, tone: "verify" },
                { label: "Visible Tok", value: formatCompact(activeCampaign?.total_visible_output_tokens_est), detail: formatCompact(activeCampaign?.total_tokens), tone: "pressure" },
                { label: "Reports / Datasets", value: `${activeCampaign?.report_count ?? 0} / ${activeCampaign?.dataset_count ?? 0}`, detail: activeCampaign?.status ?? "—" },
                { label: "Live Focus", value: currentCampaignLiveRuns[0]?.current_focus ?? harnessRuns[0]?.current_focus ?? "—", detail: currentCampaignLiveRuns[0]?.activity_heat ?? harnessRuns[0]?.activity_heat ?? "idle", tone: "authority" },
                { label: "Live Msg / Tool", value: `${currentCampaignLiveRuns.reduce((sum, run) => sum + run.progress.message_count, 0) || harnessRuns.reduce((sum, run) => sum + run.progress.message_count, 0)} / ${currentCampaignLiveRuns.reduce((sum, run) => sum + run.progress.tool_count, 0) || harnessRuns.reduce((sum, run) => sum + run.progress.tool_count, 0)}`, detail: `${currentCampaignLiveRuns.reduce((sum, run) => sum + run.progress.command_count, 0) || harnessRuns.reduce((sum, run) => sum + run.progress.command_count, 0)} cmd`, tone: "signal" },
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
            {!currentCampaignLiveRuns.length && otherLiveRuns.length ? (
              <>
                <div className="panel-divider" />
                <StateNotice
                  title="当前主战场暂时没有 live run"
                  body={`控制台检测到 ${otherLiveRuns.length} 个其他 campaign 的历史 live / stalled runs；下面会把它们降级到次级视图，避免和当前实验混在一起。`}
                  tone="info"
                />
              </>
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
              {!runningProcesses.length && currentCampaignLiveRuns.length ? (
                <>
                  <div className="panel-divider" />
                  <StateNotice
                    title="当前没有受管进程，但 live run 仍然存在"
                    body="这通常表示这些 run 不是由当前 control plane 启动，或者进程已经退出但 artifacts 仍在持续被观察。你仍然可以通过 run war room 监视 raw events 和机制链。"
                    tone="info"
                  />
                </>
              ) : null}
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
              {currentCampaignLiveRuns.length === 0 && harnessRuns.length === 0 && activeRuns.length === 0 ? (
                <div className="empty-box">当前没有 live run。</div>
              ) : (
                (currentCampaignLiveRuns.length > 0 ? currentCampaignLiveRuns : harnessRuns.length > 0 ? harnessRuns : activeRuns).map((run) =>
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

          <Panel title="Campaign Queue Ledger" kicker="Current campaign manifests, even when live rails lag behind">
            {queuedCampaignRuns.length ? (
              <div className="run-card-grid-board run-card-grid-2">
                {queuedCampaignRuns.map((run) => (
                  <RunCard
                    key={`queue-${run.run_id}`}
                    run={run}
                    selected={selectedRunId === run.run_id}
                    onSelect={() => setSelectedRunId(run.run_id)}
                    compact
                  />
                ))}
              </div>
            ) : (
              <StateNotice
                title="当前 campaign 队列为空"
                body="如果这里为空，通常表示当前 campaign 已完成，或者 detail 仍在加载。"
                tone="success"
              />
            )}
          </Panel>

          {otherLiveRuns.length ? (
            <Panel title="Historical / Spillover Live Runs" kicker="Still marked running, but not part of the current primary campaign">
              <StateNotice
                title="这些 run 仍然值得看，但不应和当前主战场混在一起"
                body="这里收纳其他 campaign 中仍处于 running / stalled 状态的 run，帮助你识别历史残留、挂起会话或需要人工清理的实验。"
                tone="warning"
              />
              <div className="run-card-grid-board run-card-grid-2">
                {otherLiveRuns.slice(0, 8).map((run) => (
                  <div key={`spillover-${run.run_id}`} className="focus-callout">
                    <strong>{run.instance_id}</strong>
                    <div className="brief-meta">
                      <span>{run.cohort_id}</span>
                      <span>{run.activity_heat}</span>
                    </div>
                    <div className="mono-note">{truncateMiddle(run.current_focus ?? run.latest_message_preview ?? "—", 120)}</div>
                    <Link to={`/runs/${encodeURIComponent(run.run_id)}`} className="artifact-chip">
                      open war room
                    </Link>
                  </div>
                ))}
              </div>
            </Panel>
          ) : null}

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
                <StateNotice
                  title="热度排行还没有形成"
                  body="如果 live snapshot 还没拿到稳定的 token / tool / message 行，热度榜会暂时空缺；这并不一定表示 run 没在推进。"
                  tone="loading"
                />
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
                <StateNotice
                  title="目前没有需要人工介入的全局警告"
                  body="当 stalled run、personality fallback、harness friction 或最新全局 warning 出现时，这里会优先把它们抬出来。"
                  tone="success"
                />
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
                {!selectedRunMessageRows.length && !selectedRunToolRows.length && "progress" in selectedRun && selectedRun.progress.raw_event_count > 0 ? (
                  <>
                    <div className="panel-divider" />
                    <StateNotice
                      title="该 run 已有 raw 事件，但归一化表还没追上"
                      body={`目前已经观察到 ${selectedRun.progress.raw_event_count} 条 raw 事件。war room 会优先显示 raw attempt log / focus / warnings，等 message/tool/patch rows 落盘后再切换到更密的结构化轨道。`}
                      tone="info"
                    />
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
                      {selectedRunEvents.filter((event) => event.type === "run.message.appended").length ? (
                        <EventRail rows={selectedRunEvents.filter((event) => event.type === "run.message.appended")} emptyLabel="等待该 run 的 live message。" />
                      ) : (
                        <StateNotice
                          title="可见输出轨道还没有结构化增量"
                          body={selectedLiveRun?.latest_message_preview ?? "当前还没有最新可见输出预览。"}
                          tone="loading"
                        />
                      )}
                    </Panel>
                    <Panel title="Live Attempt Log Tail" kicker={selectedRunAttemptLogArtifact?.name ?? "attempt-log.txt"}>
                      <pre className="artifact-pre artifact-pre-medium">
                        {selectedRunAttemptTail.data?.lines.join("\n") ?? "等待 attempt log…"}
                      </pre>
                    </Panel>
                  </div>
                  <div className="war-room-column">
                    <Panel title="Live Tool / Command Rail" kicker="Concrete tooling cadence">
                      {selectedRunEvents.filter((event) => event.type === "run.tool.appended" || event.type === "run.command.appended").length ? (
                        <EventRail rows={selectedRunEvents.filter((event) => event.type === "run.tool.appended" || event.type === "run.command.appended")} emptyLabel="等待该 run 的 tool / command 流。" />
                      ) : (
                        <StateNotice
                          title="工具与命令轨道暂时为空"
                          body={selectedLiveRun?.latest_tool ?? selectedLiveRun?.latest_command ?? "该 run 还没把 tool / command 增量落到实时轨道。"}
                          tone="loading"
                        />
                      )}
                    </Panel>
                    <Panel title="Live Patch / Mechanism Rail" kicker="Patch chain + personality / skill / token pressure">
                      {selectedRunEvents.filter((event) => event.type === "run.patch.appended" || event.type === "run.personality.appended" || event.type === "run.mechanism.appended" || event.type === "run.skill.appended" || event.type === "run.token.appended" || event.type === "run.phase.changed" || event.type === "run.focus.changed" || event.type === "run.warning.appended").length ? (
                        <EventRail rows={selectedRunEvents.filter((event) => event.type === "run.patch.appended" || event.type === "run.personality.appended" || event.type === "run.mechanism.appended" || event.type === "run.skill.appended" || event.type === "run.token.appended" || event.type === "run.phase.changed" || event.type === "run.focus.changed" || event.type === "run.warning.appended")} emptyLabel="等待 patch / mechanism 流。" />
                      ) : (
                        <StateNotice
                          title="机制轨道暂时还没收到结构化事件"
                          body={selectedLiveRun?.mechanism.latest_mechanism_event ?? "当前没有新的 personality / skill / token / patch 机制增量。"}
                          tone="loading"
                        />
                      )}
                    </Panel>
                  </div>
                </div>
                {selectedRunOperational.data?.latest_reports.length || selectedRunOperational.data?.latest_datasets.length ? (
                  <>
                    <div className="panel-divider" />
                    <div className="artifact-list artifact-list-column artifact-ledger">
                      {selectedRunOperational.data?.latest_reports.map((artifact) => (
                        <Link key={artifact.path} to="/artifacts" className="artifact-row">
                          <div className="artifact-row-main">
                            <strong>{artifact.name}</strong>
                            <span className="artifact-role">{artifact.role ?? "report"}</span>
                            <span className="artifact-scope">{artifact.scope}</span>
                          </div>
                        </Link>
                      ))}
                      {selectedRunOperational.data?.latest_datasets.map((artifact) => (
                        <Link key={artifact.path} to="/artifacts" className="artifact-row">
                          <div className="artifact-row-main">
                            <strong>{artifact.name}</strong>
                            <span className="artifact-role">{artifact.role ?? "dataset"}</span>
                            <span className="artifact-scope">{artifact.scope}</span>
                          </div>
                        </Link>
                      ))}
                    </div>
                  </>
                ) : null}
              </>
            ) : (
              <div className="empty-box">选择一个活跃 run，就能看到更像 war room 的 live 速览。</div>
            )}
          </Panel>

          <Panel title="Managed Process Matrix" kicker="Launch / stop / inspect">
            {!runningProcesses.length ? (
              <StateNotice
                title="当前没有受管进程"
                body="如果 benchmark 是在当前 control plane 之外启动的，这张表会为空，但上面的 live run 和 artifact 轨道仍然会继续显示真实 harness 现场。"
                tone="info"
              />
            ) : null}
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
                <StateNotice
                  title="控制台暂时没有新的 stdout / stderr"
                  body="如果 run 是外部启动的，console pane 可能为空，但上面的 raw / structured event rails 仍会继续更新。"
                  tone="info"
                />
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
                { label: "Messages", value: currentCampaignLiveRuns.reduce((sum, run) => sum + run.progress.message_count, 0), tone: "signal" },
                { label: "Tools", value: currentCampaignLiveRuns.reduce((sum, run) => sum + run.progress.tool_count, 0), tone: "pressure" },
                { label: "Commands", value: currentCampaignLiveRuns.reduce((sum, run) => sum + run.progress.command_count, 0) },
                { label: "Patches", value: currentCampaignLiveRuns.reduce((sum, run) => sum + run.progress.patch_event_count, 0), tone: "verify" },
                { label: "Visible Tok", value: formatCompact(currentCampaignLiveRuns.reduce((sum, run) => sum + run.telemetry.visible_output_total_tokens_est, 0)) },
                { label: "Harness Friction", value: currentCampaignLiveRuns.reduce((sum, run) => sum + run.mechanism.harness_friction_count, 0), tone: "anomaly" },
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
            {currentCampaignLiveRuns.length ? (
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
              <StateNotice
                title="当前 campaign 还没有生成报告"
                body="当 solver 完成或手动触发 report 后，这里会直接挂上最新的 report.txt、Markdown 专题和数据集。"
                tone="loading"
              />
            )}
          </Panel>
        </div>
      </div>
    </div>
  );
}
