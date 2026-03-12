import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useParams } from "react-router-dom";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { DataTable } from "../components/DataTable";
import { EventRail } from "../components/EventRail";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { SignalBar } from "../components/SignalBar";
import { StateNotice } from "../components/StateNotice";
import { StatusBadge } from "../components/StatusBadge";
import { TimelineRail } from "../components/TimelineRail";
import { api } from "../lib/api";
import { detectToneFromStatus, formatCompact, formatDate, percentFromBps, summarizeMap, truncateMiddle } from "../lib/format";
import { readTableRows, useArtifactTail, useEventStreamStatus, useLiveRun, useRunDetail, useRunEventBuckets, useRunOperationalSummary } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";

const detailTabs = [
  { value: "war-room", label: "War Room" },
  { value: "timeline", label: "Timeline" },
  { value: "messages", label: "Messages" },
  { value: "tools", label: "Tools" },
  { value: "commands", label: "Commands" },
  { value: "mechanisms", label: "Mechanisms" },
  { value: "evidence", label: "Evidence Dock" },
];

export function RunDetailPage() {
  const { runId = "" } = useParams();
  const stream = useEventStreamStatus();
  const { data, loading, error } = useRunDetail(runId);
  const [detailTab, setDetailTab] = useState("war-room");
  const [selectedArtifactPath, setSelectedArtifactPath] = useState<string | null>(null);
  const [actionBusy, setActionBusy] = useState<string | null>(null);
  const [actionResult, setActionResult] = useState<string>("");

  const run = data?.run;
  const liveRun = useLiveRun(runId);
  const messageRows = readTableRows(data ?? null, "messageMetrics");
  const toolRows = readTableRows(data ?? null, "toolEvents");
  const commandRows = readTableRows(data ?? null, "commandEvents");
  const patchRows = readTableRows(data ?? null, "patchChain");
  const personalityRows = readTableRows(data ?? null, "personalityEvents");
  const skillRows = readTableRows(data ?? null, "skillMechanism");
  const couplingRows = readTableRows(data ?? null, "verbosityToolCoupling");
  const turnRows = readTableRows(data ?? null, "turnMetrics");
  const availableArtifacts = useMemo(() => data?.attempt_artifacts ?? [], [data?.attempt_artifacts]);
  const selectedArtifact = availableArtifacts.find((artifact) => artifact.path === selectedArtifactPath) ?? availableArtifacts[0] ?? null;
  const tail = useArtifactTail(selectedArtifact?.path ?? null, 120, Boolean(selectedArtifact));
  const recentLiveEvents = useRunEventBuckets(runId, [
    "run.updated",
    "run.timeline.appended",
    "run.message.appended",
    "run.tool.appended",
    "run.patch.appended",
    "run.command.appended",
    "run.personality.appended",
    "run.mechanism.appended",
    "run.skill.appended",
    "run.token.appended",
    "run.phase.changed",
    "run.focus.changed",
    "run.warning.appended",
  ], 18);
  const liveSnapshot = liveRun.data ?? data?.live_snapshot ?? null;
  const runOperational = useRunOperationalSummary(runId);

  const topTools = useMemo(() => summarizeMap(run?.tool_name_counts, 5), [run?.tool_name_counts]);
  const topRoutes = useMemo(() => summarizeMap(run?.tool_route_counts, 5), [run?.tool_route_counts]);
  const topCategories = useMemo(() => summarizeMap(run?.message_category_counts, 5), [run?.message_category_counts]);

  const patchPreview = data?.previews.patchDiff ?? "";
  const attemptLog = data?.previews.attemptLog ?? "";
  const runEvidence = data?.previews.runEvidence ?? "";
  const mechanismRows = [...personalityRows.slice(0, 8), ...skillRows.slice(0, 8)];
  const messageMechanismSummary = useMemo(() => {
    if (!messageRows.length) return null;
    const avg = (field: string) => {
      const values = messageRows
        .map((row) => Number(row[field] ?? 0))
        .filter((value) => Number.isFinite(value));
      if (!values.length) return 0;
      return Math.round(values.reduce((sum, value) => sum + value, 0) / values.length);
    };
    return {
      bridge: avg("bridge_language_score_bps"),
      verification: avg("verification_language_score_bps"),
      externalization: avg("state_externalization_score_bps"),
      collaboration: avg("collaboration_tone_score_bps"),
    };
  }, [messageRows]);
  const turnStripRows = useMemo(
    () => turnRows.slice(0, 12).map((row, index) => ({
      label: `turn ${row.turn_id ?? row.turnId ?? index + 1}`,
      input: Number(row.input_tokens ?? row.inputTokens ?? 0),
      output: Number(row.output_tokens ?? row.outputTokens ?? 0),
      total: Number(row.total_tokens ?? row.totalTokens ?? 0),
    })),
    [turnRows],
  );
  const turnMaxTotal = useMemo(
    () => Math.max(...turnStripRows.map((row) => row.total), 1),
    [turnStripRows],
  );
  const liveEventCounts = useMemo(() => ({
    message: recentLiveEvents.filter((event) => event.type === "run.message.appended").length,
    tool: recentLiveEvents.filter((event) => event.type === "run.tool.appended" || event.type === "run.command.appended").length,
    patch: recentLiveEvents.filter((event) => event.type === "run.patch.appended").length,
    mechanism: recentLiveEvents.filter((event) => !["run.message.appended", "run.tool.appended", "run.command.appended", "run.patch.appended"].includes(event.type)).length,
  }), [recentLiveEvents]);
  const runReadiness = useMemo(() => {
    const warnings = runOperational.data?.operational_warnings.length ?? liveSnapshot?.warnings.length ?? 0;
    const rawEventCount = liveSnapshot?.progress.raw_event_count ?? 0;
    const normalizedRows =
      messageRows.length +
      toolRows.length +
      patchRows.length +
      personalityRows.length +
      skillRows.length +
      turnRows.length;
    return {
      warnings,
      rawEventCount,
      normalizedRows,
    };
  }, [
    liveSnapshot?.progress.raw_event_count,
    liveSnapshot?.warnings.length,
    messageRows.length,
    personalityRows.length,
    patchRows.length,
    runOperational.data?.operational_warnings.length,
    skillRows.length,
    toolRows.length,
    turnRows.length,
  ]);
  const eventTableHighlights = useMemo(
    () => Object.entries(runOperational.data?.event_table_counts ?? {}).slice(0, 8),
    [runOperational.data?.event_table_counts],
  );
  const artifactTypeHighlights = useMemo(
    () => Object.entries(runOperational.data?.artifact_type_counts ?? {}).slice(0, 8),
    [runOperational.data?.artifact_type_counts],
  );

  const triggerRunAction = async (kind: "report" | "replay") => {
    if (!run) return;
    setActionBusy(kind);
    setActionResult("");
    try {
      const launched = await api.action(kind, { campaign_dir: run.run_dir });
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
        kicker={run?.cohort_id ?? runId}
        title={run?.instance_id ?? "Run Detail"}
        description={
          run
            ? `围绕 ${run.model} × ${run.personality_mode ?? "none"} 的单题战情室。这里统一展示用户可见输出、具体工具路由、patch 链路、personality 与 skill 机制。`
            : "加载 run detail…"
        }
        actions={run ? (
          <div className="chip-row">
            <Link to="/compare" className="artifact-chip">compare</Link>
            <Link to="/artifacts" className="artifact-chip">artifacts</Link>
            <button type="button" className="artifact-chip" disabled={actionBusy !== null} onClick={() => void triggerRunAction("report")}>
              {actionBusy === "report" ? "report…" : "rebuild report"}
            </button>
            <button type="button" className="artifact-chip" disabled={actionBusy !== null} onClick={() => void triggerRunAction("replay")}>
              {actionBusy === "replay" ? "replay…" : "replay"}
            </button>
            {actionResult ? <span className="mono-note">{actionResult}</span> : null}
          </div>
        ) : null}
      />

      {loading ? <div className="empty-box">正在读取 run detail bundle…</div> : null}
      {error ? <div className="empty-box">{error}</div> : null}

      {run ? (
        <>
          <div className="page-grid page-grid-4">
            <MetricCard label="Status" value={<StatusBadge tone={detectToneFromStatus(run.status)}>{run.status}</StatusBadge>} detail={run.grading_status} />
            <MetricCard label="Visible Output" value={formatCompact(run.visible_output_total_tokens_est)} detail={`${run.message_metric_count} message rows`} tone="signal" />
            <MetricCard label="Tool / Command" value={`${run.tool_count} / ${run.command_count}`} detail={`${run.patch_file_count} patch files`} tone="pressure" />
            <MetricCard label="Verification / Friction" value={`${run.verification_closure_count} / ${run.harness_friction_count}`} detail={`${run.personality_fallback_count} personality fallback`} tone="verify" />
          </div>

          <Panel title="Run Readiness Dossier" kicker="How complete is this war room right now?">
            <KeyValueGrid
              columns={4}
              items={[
                { label: "Raw Event Surface", value: runReadiness.rawEventCount, detail: "live raw / probe event rows", tone: runReadiness.rawEventCount ? "signal" : "neutral" },
                { label: "Normalized Surface", value: runReadiness.normalizedRows, detail: "message / tool / patch / mechanism tables", tone: runReadiness.normalizedRows ? "verify" : "pressure" },
                { label: "Warnings", value: runReadiness.warnings, detail: runOperational.data?.operational_warnings?.[0] ?? liveSnapshot?.warnings?.[0] ?? "none", tone: runReadiness.warnings ? "anomaly" : "neutral" },
                { label: "Attempt Artifacts", value: availableArtifacts.length, detail: selectedArtifact?.name ?? "—" },
                { label: "Latest Report", value: runOperational.data?.latest_reports[0]?.name ?? data?.reports[0]?.name ?? "—" },
                { label: "Latest Dataset", value: runOperational.data?.latest_datasets[0]?.name ?? data?.datasets[0]?.name ?? "—" },
                { label: "Current Focus", value: liveSnapshot?.current_focus ?? runOperational.data?.latest_focus ?? "—" },
                { label: "Latest Message", value: truncateMiddle(liveSnapshot?.latest_message_preview ?? runOperational.data?.latest_message_preview ?? "—", 92) },
              ]}
            />
          </Panel>

          {liveSnapshot ? (
            <div className="run-header-band">
              <KeyValueGrid
                columns={5}
                items={[
                  { label: "Stream", value: stream.status, detail: stream.lastEventAt ? formatDate(stream.lastEventAt) : "no live event yet", tone: stream.status === "connected" ? "signal" : stream.status === "degraded" ? "anomaly" : "neutral" },
                  { label: "Current Phase", value: liveSnapshot.progress.current_phase, detail: liveSnapshot.run_status, tone: "authority" },
                  { label: "Elapsed", value: liveSnapshot.elapsed_ms ? `${Math.round(liveSnapshot.elapsed_ms / 1000)}s` : "—", detail: formatDate(liveSnapshot.last_event_at) },
                  { label: "Live Tokens/min", value: formatCompact(liveSnapshot.telemetry.tokens_per_minute), detail: `${formatCompact(liveSnapshot.telemetry.total_tokens)} total`, tone: "signal" },
                  { label: "Live Tools/min", value: liveSnapshot.telemetry.tool_bursts_per_minute.toFixed(1), detail: `${liveSnapshot.progress.tool_count} total`, tone: "pressure" },
                  { label: "Live Msg/min", value: liveSnapshot.telemetry.messages_per_minute.toFixed(1), detail: `${liveSnapshot.progress.message_count} total`, tone: "verify" },
                  { label: "Activity Heat", value: liveSnapshot.activity_heat, detail: liveSnapshot.current_focus ?? "—", tone: liveSnapshot.activity_heat === "hot" ? "pressure" : liveSnapshot.activity_heat === "stalled" ? "anomaly" : "signal" },
                  { label: "Msg Category", value: liveSnapshot.mechanism.last_message_category ?? "—", detail: liveSnapshot.mechanism.top_tool_route ?? "—" },
                  { label: "Active Skills", value: liveSnapshot.mechanism.active_skill_names.join(" · ") || "—", detail: `${liveSnapshot.mechanism.skill_inferred_count} inferred` },
                  { label: "Artifact Rows", value: liveSnapshot.progress.artifact_row_count, detail: `${liveSnapshot.progress.raw_event_count} raw events` },
                  { label: "Flow Ratio", value: `${liveSnapshot.telemetry.visible_tokens_per_message.toFixed(1)} tok/msg`, detail: `${liveSnapshot.telemetry.tool_calls_per_message.toFixed(2)} tool/msg` },
                  { label: "Latest Message", value: truncateMiddle(liveSnapshot.latest_message_preview ?? "—", 84) },
                  { label: "Latest Tool", value: liveSnapshot.latest_tool ?? "—" },
                  { label: "Latest Patch", value: liveSnapshot.latest_patch ?? "—" },
                  { label: "Latest Command", value: truncateMiddle(liveSnapshot.latest_command ?? "—", 72) },
                  { label: "Latest Mechanism", value: liveSnapshot.mechanism.latest_mechanism_event ?? "—" },
                ]}
              />
              {liveSnapshot.warnings.length ? (
                <>
                  <div className="panel-divider" />
                  <div className="warning-tape warning-tape-block">
                    {liveSnapshot.warnings.map((warning) => (
                      <span key={warning}>{warning}</span>
                    ))}
                  </div>
                </>
              ) : null}
            </div>
          ) : null}

          {runOperational.data ? (
            <Panel title="Operational Snapshot" kicker="Run observer / artifact index / live health">
              <KeyValueGrid
                columns={5}
                items={[
                  { label: "Attempt Artifacts", value: runOperational.data.attempt_artifact_count, detail: `${runOperational.data.latest_reports.length} reports / ${runOperational.data.latest_datasets.length} datasets` },
                  { label: "Artifact Types", value: artifactTypeHighlights.length, detail: artifactTypeHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                  { label: "Event Tables", value: eventTableHighlights.length, detail: eventTableHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                  { label: "Operational Warnings", value: runOperational.data.operational_warnings.length, detail: runOperational.data.operational_warnings[0] ?? "none", tone: "anomaly" },
                  { label: "Latest Reports", value: runOperational.data.latest_reports.length, detail: runOperational.data.latest_reports[0]?.name ?? "—" },
                ]}
              />
              {runOperational.data.operational_warnings.length ? (
                <>
                  <div className="panel-divider" />
                  <div className="warning-tape warning-tape-block">
                    {runOperational.data.operational_warnings.map((warning) => (
                      <span key={warning}>{warning}</span>
                    ))}
                  </div>
                </>
              ) : null}
            </Panel>
          ) : null}

          <div className="page-grid page-grid-4">
            <MetricCard label="Live Message Rail" value={liveEventCounts.message} detail="recent appended message events" tone="signal" />
            <MetricCard label="Live Tool Rail" value={liveEventCounts.tool} detail="recent tool / command events" tone="pressure" />
            <MetricCard label="Live Patch Rail" value={liveEventCounts.patch} detail="recent patch chain updates" tone="verify" />
            <MetricCard label="Live Mechanism Rail" value={liveEventCounts.mechanism} detail="personality / skill / token / compaction" tone="anomaly" />
          </div>

          {messageMechanismSummary ? (
            <div className="page-grid page-grid-4">
              <MetricCard label="Bridge Avg" value={percentFromBps(messageMechanismSummary.bridge)} detail="message-level bridge language" tone="signal" />
              <MetricCard label="Verification Avg" value={percentFromBps(messageMechanismSummary.verification)} detail="verification framing intensity" tone="verify" />
              <MetricCard label="Externalization Avg" value={percentFromBps(messageMechanismSummary.externalization)} detail="state verbalization surface" tone="pressure" />
              <MetricCard label="Collaboration Avg" value={percentFromBps(messageMechanismSummary.collaboration)} detail="collaboration / warmth tone" />
            </div>
          ) : null}

          <div className="run-header-band">
            <KeyValueGrid
              columns={5}
              items={[
                { label: "Model", value: run.model },
                { label: "Personality", value: run.personality_mode ?? "none" },
                { label: "Task Class", value: run.task_class },
                { label: "Prompt Style", value: run.prompt_style ?? "—" },
                { label: "Updated", value: formatDate(run.latest_updated_at) },
                { label: "Repo", value: run.repo },
                { label: "Visible Tok", value: formatCompact(run.visible_output_total_tokens_est), tone: "signal" },
                { label: "Total Tok", value: formatCompact(run.total_tokens) },
                { label: "Top Tools", value: topTools.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                { label: "Top Routes", value: topRoutes.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
              ]}
            />
          </div>

          <Panel
            title="Run War Room"
            kicker="Live telemetry / message stream / tool rail / mechanism rail / evidence dock"
            actions={<SegmentedTabs items={detailTabs} value={detailTab} onChange={setDetailTab} />}
          >
            {detailTab === "war-room" ? (
              <div className="war-room-layout">
                <div className="war-room-column">
                  <Panel title="Message Stream" kicker="Visible output + discourse surface">
                    {!messageRows.length ? (
                      <StateNotice
                        tone={recentLiveEvents.some((event) => event.type === "run.message.appended") ? "warning" : "info"}
                        title={recentLiveEvents.some((event) => event.type === "run.message.appended") ? "Live message 已经开始流动" : "结构化 message 还未落盘"}
                        body={
                          recentLiveEvents.some((event) => event.type === "run.message.appended")
                            ? "这通常表示 Codex 已经在输出用户可见内容，但 message-metrics 这一层还在等待归一化 rows 或 attempt 文件继续写入。先看下面的 live rail。"
                            : "如果 run 刚启动，这是正常的。随着消息、tool 和 patch 继续增长，这里会自动切换成完整的 discourse / tone / bridge 分析表。"
                        }
                      />
                    ) : null}
                    <DataTable rows={messageRows.slice(0, 40)} compact />
                    <div className="panel-divider" />
                    <EventRail rows={recentLiveEvents.filter((event) => event.type === "run.message.appended")} emptyLabel="等待 live message 流。" />
                  </Panel>
                  <Panel title="Timeline Rail" kicker="Structured chronology">
                    <TimelineRail rows={data?.timeline ?? []} emptyLabel="当前没有结构化时间线。" />
                  </Panel>
                </div>
                <div className="war-room-column">
                  <Panel title="Tool Rail" kicker="Concrete Codex tools / routes / timing">
                    {!toolRows.length ? (
                      <StateNotice
                        tone={recentLiveEvents.some((event) => event.type === "run.tool.appended" || event.type === "run.command.appended") ? "warning" : "info"}
                        title={recentLiveEvents.some((event) => event.type === "run.tool.appended" || event.type === "run.command.appended") ? "Live tool / command 已出现" : "结构化工具画像尚未完成"}
                        body={
                          recentLiveEvents.some((event) => event.type === "run.tool.appended" || event.type === "run.command.appended")
                            ? "raw event 已经表明 Codex 在调用工具或执行命令；上面的结构化工具画像表还在等待 tool-events / command-events 等归一化文件补齐。"
                            : "如果当前 run 仍处在早期阅读、推理或等待阶段，这里会暂时为空；一旦 shell、patch、MCP 或 dynamic tool 事件落盘，这个 rail 会自动变得详细。"
                        }
                      />
                    ) : null}
                    <DataTable rows={toolRows.slice(0, 36)} compact />
                    <div className="panel-divider" />
                    <EventRail rows={recentLiveEvents.filter((event) => event.type === "run.tool.appended" || event.type === "run.command.appended")} emptyLabel="等待 live tool / command 流。" />
                  </Panel>
                  <Panel title="Patch Rail" kicker="Patch chain / approvals / diff evolution">
                    {!patchRows.length ? (
                      <StateNotice
                        tone={recentLiveEvents.some((event) => event.type === "run.patch.appended") ? "warning" : "info"}
                        title={recentLiveEvents.some((event) => event.type === "run.patch.appended") ? "Patch 事件已经出现" : "还没有 patch chain"}
                        body={
                          recentLiveEvents.some((event) => event.type === "run.patch.appended")
                            ? "当前 patch 事件已经在 live rail 中出现，但 patch-chain 或 diff 预览还在等待文件链条补齐。下面的 rail 往往会比结构化 patch 表更早更新。"
                            : "如果 run 还在搜索、定位或复现问题，这里为空是正常的。真正进入 edit / apply_patch / diff 之后，这里会变成 patch chain 轨道。"
                        }
                      />
                    ) : null}
                    <DataTable rows={patchRows.slice(0, 24)} compact />
                    <pre className="artifact-pre artifact-pre-medium">{patchPreview || "没有 patch diff 预览。"}</pre>
                    <div className="panel-divider" />
                    <EventRail rows={recentLiveEvents.filter((event) => event.type === "run.patch.appended")} emptyLabel="等待 live patch 流。" />
                  </Panel>
                </div>
                <div className="war-room-column">
                <Panel title="Mechanism Rail" kicker="Personality / skill / instruction / token pressure">
                    <KeyValueGrid
                      columns={2}
                      items={[
                        { label: "Personality Fallback", value: run.personality_fallback_count, tone: run.personality_fallback_count ? "anomaly" : "neutral" },
                        { label: "Harness Friction", value: run.harness_friction_count, tone: run.harness_friction_count ? "pressure" : "neutral" },
                        { label: "Ignition Shell Search", value: run.ignition_shell_search_count },
                        { label: "Verification Closures", value: run.verification_closure_count, tone: "verify" },
                        { label: "Top Categories", value: topCategories.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                        { label: "Turn Rows", value: turnRows.length },
                        { label: "Instruction Layers", value: liveSnapshot?.mechanism.instruction_layers_active.join(" · ") || "—" },
                        { label: "Skill Inferred", value: liveSnapshot?.mechanism.skill_inferred_count ?? 0 },
                      ]}
                    />
                    <div className="panel-divider" />
                    <DataTable rows={mechanismRows as Array<Record<string, unknown>>} compact />
                    <div className="panel-divider" />
                    <EventRail rows={recentLiveEvents.filter((event) => event.type !== "run.message.appended" && event.type !== "run.tool.appended" && event.type !== "run.command.appended" && event.type !== "run.patch.appended")} emptyLabel="等待更多 live 机制事件。" />
                  </Panel>
                  <Panel title="Operational Tables" kicker="Artifact types / event table counts / dossier readiness">
                    <KeyValueGrid
                      columns={2}
                      items={[
                        { label: "Artifact Types", value: artifactTypeHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                        { label: "Event Tables", value: eventTableHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                        { label: "Latest Report", value: runOperational.data?.latest_reports[0]?.name ?? "—" },
                        { label: "Latest Dataset", value: runOperational.data?.latest_datasets[0]?.name ?? "—" },
                        { label: "Current Focus", value: runOperational.data?.latest_focus ?? liveSnapshot?.current_focus ?? "—" },
                        { label: "Live Warnings", value: runOperational.data?.live_warning_count ?? liveSnapshot?.warnings.length ?? 0, detail: runOperational.data?.operational_warnings?.[0] ?? liveSnapshot?.warnings?.[0] ?? "none" },
                        { label: "Latest Message", value: truncateMiddle(runOperational.data?.latest_message_preview ?? liveSnapshot?.latest_message_preview ?? "—", 72) },
                        { label: "Latest Command", value: truncateMiddle(runOperational.data?.latest_command ?? liveSnapshot?.latest_command ?? "—", 72) },
                      ]}
                    />
                    {runOperational.data?.operational_warnings?.length ? (
                      <>
                        <div className="panel-divider" />
                        <div className="warning-tape warning-tape-block">
                          {runOperational.data.operational_warnings.map((warning) => (
                            <span key={warning}>{warning}</span>
                          ))}
                        </div>
                      </>
                    ) : null}
                  </Panel>
                </div>
              </div>
            ) : null}

            {detailTab === "timeline" ? <TimelineRail rows={data?.timeline ?? []} emptyLabel="当前没有结构化时间线。" /> : null}

            {detailTab === "messages" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Message Metrics" kicker="Visible output / discourse / tone">
                  {!messageRows.length ? (
                    <StateNotice
                      tone="warning"
                      title="message 指标表暂时为空"
                      body="这不一定意味着 run 没有输出。请结合上面的 live message rail 与 attempt-log 一起看；如果 raw agent events 正在增长，这里通常只是 message-metrics 归一化稍慢。"
                    />
                  ) : null}
                  {messageMechanismSummary ? (
                    <>
                      <div className="signal-board-grid signal-board-grid-compact">
                        <SignalBar label="Bridge" value={messageMechanismSummary.bridge} max={10_000} tone="signal" detail="bps" />
                        <SignalBar label="Verification" value={messageMechanismSummary.verification} max={10_000} tone="verify" detail="bps" />
                        <SignalBar label="Externalization" value={messageMechanismSummary.externalization} max={10_000} tone="pressure" detail="bps" />
                        <SignalBar label="Collaboration" value={messageMechanismSummary.collaboration} max={10_000} tone="authority" detail="bps" />
                      </div>
                      <div className="panel-divider" />
                    </>
                  ) : null}
                  <DataTable rows={messageRows} />
                </Panel>
                <Panel title="Verbosity Coupling" kicker="Commentary × tool interaction">
                  <DataTable rows={couplingRows} />
                </Panel>
              </div>
            ) : null}

            {detailTab === "tools" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Tool Events" kicker="Concrete Codex tools / routes / timings">
                  {!toolRows.length ? (
                    <StateNotice
                      tone="warning"
                      title="tool 表暂时为空"
                      body="如果下方或 war room 的 live rail 已经出现 command / tool 事件，这里只是归一化数据还没追上。优先结合 live rail、attempt-log 与 raw-agent-events 来判断 run 是否真的卡住。"
                    />
                  ) : null}
                  <DataTable rows={toolRows} />
                </Panel>
                <Panel title="Patch Chain" kicker="Patch approvals / failures / chain evolution">
                  {!patchRows.length ? (
                    <StateNotice
                      tone="info"
                      title="patch chain 尚未形成"
                      body="这通常意味着当前仍在搜索、定位或验证阶段。只要命令、message 或 tool 事件还在前进，就不代表 run 已经失活。"
                    />
                  ) : null}
                  <DataTable rows={patchRows} />
                </Panel>
              </div>
            ) : null}

            {detailTab === "commands" ? (
              <Panel title="Command Ledger" kicker="Shell / exec chronology">
                <DataTable rows={commandRows} />
              </Panel>
            ) : null}

            {detailTab === "mechanisms" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Personality Mechanism" kicker="Requested / effective / fallback">
                  <DataTable rows={personalityRows} />
                </Panel>
                <Panel title="Skill Mechanism" kicker="Skill catalog / inferred use / triggers">
                  <DataTable rows={skillRows} />
                </Panel>
                <Panel title="Mechanism Event Rail" kicker="phase / focus / warnings / live pressure">
                  <EventRail rows={recentLiveEvents.filter((event) => event.type !== "run.message.appended" && event.type !== "run.tool.appended" && event.type !== "run.command.appended" && event.type !== "run.patch.appended")} emptyLabel="等待机制事件。" />
                </Panel>
                <Panel title="Operational Dossier" kicker="Run observer / live snapshot / artifact readiness">
                  <KeyValueGrid
                    columns={2}
                    items={[
                      { label: "Current Phase", value: runOperational.data?.current_phase ?? liveSnapshot?.progress.current_phase ?? "—", detail: runOperational.data?.latest_focus ?? liveSnapshot?.current_focus ?? "—", tone: "authority" },
                      { label: "Live Warnings", value: runOperational.data?.live_warning_count ?? liveSnapshot?.warnings.length ?? 0, detail: runOperational.data?.operational_warnings?.[0] ?? "none", tone: "anomaly" },
                      { label: "Latest Tool", value: runOperational.data?.latest_tool ?? liveSnapshot?.latest_tool ?? "—" },
                      { label: "Latest Patch", value: runOperational.data?.latest_patch ?? liveSnapshot?.latest_patch ?? "—" },
                      { label: "Event Tables", value: eventTableHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                      { label: "Artifact Types", value: artifactTypeHighlights.map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                    ]}
                  />
                </Panel>
                <Panel title="Token / Turn Strip" kicker="Turn-by-turn token pressure">
                  {turnStripRows.length ? (
                    <div className="signal-bar-stack">
                      {turnStripRows.map((row) => (
                        <SignalBar
                          key={row.label}
                          label={row.label}
                          value={row.total}
                          max={turnMaxTotal}
                          tone="pressure"
                          detail={`in ${row.input} / out ${row.output}`}
                        />
                      ))}
                    </div>
                  ) : (
                    <div className="empty-box">还没有 turn 级 token 行。</div>
                  )}
                </Panel>
              </div>
            ) : null}

            {detailTab === "evidence" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Evidence Dock" kicker="run-evidence / attempt-log / raw / normalized">
                  <div className="artifact-list artifact-list-column artifact-ledger">
                    {availableArtifacts.map((artifact) => (
                      <button
                        key={artifact.path}
                        className={`artifact-row-button${selectedArtifact?.path === artifact.path ? " artifact-row-button-active" : ""}`}
                        onClick={() => setSelectedArtifactPath(artifact.path)}
                      >
                        <div className="artifact-row-main">
                          <strong>{artifact.name}</strong>
                          <span className="artifact-role">{artifact.role ?? artifact.kind}</span>
                          <span className="artifact-scope">{artifact.scope ?? artifact.format ?? "—"}</span>
                        </div>
                      </button>
                    ))}
                  </div>
                </Panel>
                <Panel title="Artifact Inspector" kicker={selectedArtifact?.name ?? "Choose artifact"}>
                  {selectedArtifact ? (
                    <>
                  <ArtifactViewer artifact={selectedArtifact} />
                      <KeyValueGrid
                        columns={2}
                        items={[
                          { label: "Role", value: selectedArtifact.role ?? "—", detail: selectedArtifact.scope ?? "—" },
                          { label: "Format", value: selectedArtifact.format ?? "—", detail: selectedArtifact.previewable ? "previewable" : "opaque" },
                          { label: "Rows / Lines", value: formatCompact(selectedArtifact.row_count), detail: formatCompact(selectedArtifact.line_count) },
                          { label: "Bytes", value: formatCompact(selectedArtifact.size_bytes), detail: selectedArtifact.updated_at ? formatDate(selectedArtifact.updated_at) : "—" },
                        ]}
                      />
                      <div className="tail-box">
                        <div className="section-label">Live Tail</div>
                        <pre className="artifact-pre artifact-pre-medium">
                          {tail.data?.lines.join("\n") ?? "加载 tail…"}
                        </pre>
                      </div>
                      <div className="panel-divider" />
                      <KeyValueGrid
                        columns={2}
                        items={[
                          {
                            label: "Truth Layer",
                            value:
                              selectedArtifact.role?.includes("raw_truth")
                                ? "observed"
                                : selectedArtifact.role?.includes("derived")
                                  ? "derived"
                                  : selectedArtifact.role?.includes("human_readable")
                                    ? "dossier"
                                    : "other",
                            detail: selectedArtifact.role ?? "—",
                          },
                          { label: "Scope", value: selectedArtifact.scope ?? "—", detail: selectedArtifact.kind },
                          { label: "Format", value: selectedArtifact.format ?? "—", detail: selectedArtifact.previewable ? "previewable" : "opaque" },
                          { label: "Path", value: truncateMiddle(selectedArtifact.path, 92) },
                        ]}
                      />
                    </>
                  ) : (
                    <div className="empty-box">选择一个 artifact 查看。</div>
                  )}
                </Panel>
                <Panel title="Attempt Log Preview" kicker="attempt-log.txt">
                  <pre className="artifact-pre artifact-pre-medium">{attemptLog || "没有 attempt log。"}</pre>
                </Panel>
                <Panel title="Run Evidence Preview" kicker="run-evidence.txt">
                  <pre className="artifact-pre artifact-pre-medium">{runEvidence || "没有 run evidence。"}</pre>
                </Panel>
              </div>
            ) : null}
          </Panel>
        </>
      ) : null}
    </div>
  );
}
