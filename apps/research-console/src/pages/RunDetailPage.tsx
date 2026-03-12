import { useMemo, useState } from "react";
import { useParams } from "react-router-dom";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { DataTable } from "../components/DataTable";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { StatusBadge } from "../components/StatusBadge";
import { TimelineRail } from "../components/TimelineRail";
import { formatCompact, formatDate, percentFromBps, summarizeMap, truncateMiddle } from "../lib/format";
import { readTableRows, useArtifactTail, useRunDetail } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";

const detailTabs = [
  { value: "timeline", label: "Timeline" },
  { value: "messages", label: "Messages" },
  { value: "tools", label: "Tools" },
  { value: "commands", label: "Commands" },
  { value: "mechanisms", label: "Mechanisms" },
  { value: "diff", label: "Diff" },
  { value: "artifacts", label: "Artifacts" },
];

export function RunDetailPage() {
  const { runId = "" } = useParams();
  const { data, loading, error } = useRunDetail(runId);
  const [detailTab, setDetailTab] = useState("timeline");
  const [selectedArtifact, setSelectedArtifact] = useState<ArtifactDescriptor | null>(null);
  const [logPath, setLogPath] = useState<string | null>(null);

  const run = data?.run;
  const runSummary = data?.run_summary ?? {};
  const probeSummary = data?.probe_summary ?? {};
  const messageRows = readTableRows(data ?? null, "messageMetrics");
  const toolRows = readTableRows(data ?? null, "toolEvents");
  const commandRows = readTableRows(data ?? null, "commandEvents");
  const patchRows = readTableRows(data ?? null, "patchChain");
  const personalityRows = readTableRows(data ?? null, "personalityEvents");
  const skillRows = readTableRows(data ?? null, "skillMechanism");
  const couplingRows = readTableRows(data ?? null, "verbosityToolCoupling");
  const tail = useArtifactTail(logPath, 120, Boolean(logPath));

  const availableArtifacts = useMemo(() => data?.attempt_artifacts ?? [], [data?.attempt_artifacts]);

  const topTools = useMemo(() => summarizeMap(run?.tool_name_counts, 5), [run?.tool_name_counts]);
  const topRoutes = useMemo(() => summarizeMap(run?.tool_route_counts, 5), [run?.tool_route_counts]);
  const topCategories = useMemo(() => summarizeMap(run?.message_category_counts, 5), [run?.message_category_counts]);
  const patchPreview = data?.previews.patchDiff ?? "";
  const attemptLog = data?.previews.attemptLog ?? "";
  const runEvidence = data?.previews.runEvidence ?? "";

  const openTail = (artifact: ArtifactDescriptor) => {
    setLogPath(artifact.path);
    setSelectedArtifact(artifact);
  };

  return (
    <div className="page-grid">
      <PageIntro
        kicker={run?.cohort_id ?? runId}
        title={run?.instance_id ?? "Run Detail"}
        description={
          run
            ? `围绕 ${run.model} × ${run.personality_mode ?? "none"} 的单题战情室。这里统一展示用户可见输出、工具链路、patch 机制、personality 机制与完整 artifact 面。`
            : "加载 run detail…"
        }
      />

      {loading ? <div className="empty-box">正在读取 run detail bundle…</div> : null}
      {error ? <div className="empty-box">{error}</div> : null}

      {run ? (
        <>
          <div className="page-grid page-grid-4">
            <MetricCard label="Status" value={<StatusBadge tone={run.status === "completed" ? "completed" : run.status === "running" ? "running" : "failed"}>{run.status}</StatusBadge>} detail={run.grading_status} />
            <MetricCard label="Visible Output" value={formatCompact(run.visible_output_total_tokens_est)} detail={`${run.message_metric_count} message metrics`} tone="signal" />
            <MetricCard label="Tool / Command" value={`${run.tool_count} / ${run.command_count}`} detail={`${run.patch_file_count} patch files`} tone="pressure" />
            <MetricCard label="Verification / Friction" value={`${run.verification_closure_count} / ${run.harness_friction_count}`} detail={`${run.personality_fallback_count} personality fallback`} tone="verify" />
          </div>

          <div className="run-overview-grid">
            <Panel title="Run Overview" kicker={run.run_id}>
              <div className="run-overview">
                <div><span className="metric-label">Instance</span><strong>{run.instance_id}</strong></div>
                <div><span className="metric-label">Repo</span><strong>{run.repo}</strong></div>
                <div><span className="metric-label">Model</span><strong>{run.model}</strong></div>
                <div><span className="metric-label">Personality</span><strong>{run.personality_mode ?? "-"}</strong></div>
                <div><span className="metric-label">Task Class</span><strong>{run.task_class}</strong></div>
                <div><span className="metric-label">Prompt Style</span><strong>{run.prompt_style ?? "-"}</strong></div>
                <div><span className="metric-label">Updated</span><strong>{formatDate(run.latest_updated_at)}</strong></div>
                <div><span className="metric-label">Tokens</span><strong>{formatCompact(run.total_tokens)}</strong></div>
              </div>
            </Panel>

            <Panel title="Mechanism Snapshot" kicker="Tool / route / discourse top lines">
              <div className="summary-stack">
                <SummaryList title="Top Tools" items={topTools.map(([name, count]) => `${name} × ${count}`)} />
                <SummaryList title="Top Routes" items={topRoutes.map(([name, count]) => `${name} × ${count}`)} />
                <SummaryList title="Top Message Categories" items={topCategories.map(([name, count]) => `${name} × ${count}`)} />
              </div>
            </Panel>
          </div>

          <Panel
            title="Run War Room"
            kicker="Timeline / messages / tools / commands / mechanisms / diff / artifacts"
            actions={<SegmentedTabs items={detailTabs} value={detailTab} onChange={setDetailTab} />}
          >
            {detailTab === "timeline" ? (
              <TimelineRail rows={data?.timeline ?? []} emptyLabel="当前没有结构化时间线。" />
            ) : null}

            {detailTab === "messages" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Message Metrics" kicker="Visible output / discourse / tone">
                  <DataTable rows={messageRows} />
                </Panel>
                <Panel title="Verbosity Coupling" kicker="Commentary x tool interaction">
                  <DataTable rows={couplingRows} />
                </Panel>
              </div>
            ) : null}

            {detailTab === "tools" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Tool Events" kicker="Concrete Codex tools / routes / timings">
                  <DataTable rows={toolRows} />
                </Panel>
                <Panel title="Patch Chain" kicker="Patch approvals / failures / chain evolution">
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
              </div>
            ) : null}

            {detailTab === "diff" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Patch Diff Preview" kicker="patch.diff">
                  <pre className="artifact-pre artifact-pre-tall">{patchPreview || "没有 patch diff 预览。"}</pre>
                </Panel>
                <Panel title="Attempt / Evidence Preview" kicker="attempt-log + run-evidence">
                  <div className="stacked-previews">
                    <pre className="artifact-pre artifact-pre-medium">{attemptLog || "没有 attempt log。"}</pre>
                    <pre className="artifact-pre artifact-pre-medium">{runEvidence || "没有 run evidence。"}</pre>
                  </div>
                </Panel>
              </div>
            ) : null}

            {detailTab === "artifacts" ? (
              <div className="page-grid page-grid-2">
                <Panel title="Attempt Artifacts" kicker="Latest attempt inventory">
                  <div className="artifact-list artifact-list-column">
                    {availableArtifacts.map((artifact) => (
                      <button
                        key={artifact.path}
                        className={`artifact-chip${selectedArtifact?.path === artifact.path ? " artifact-chip-active" : ""}`}
                        onClick={() => openTail(artifact)}
                      >
                        <span>{artifact.name}</span>
                        <span className="artifact-kind">{artifact.kind}</span>
                      </button>
                    ))}
                  </div>
                </Panel>
                <Panel title="Artifact Inspector" kicker={selectedArtifact?.name ?? "Choose artifact"}>
                  {selectedArtifact ? (
                    <>
                      <ArtifactViewer artifact={selectedArtifact} />
                      {logPath ? (
                        <div className="tail-box">
                          <div className="section-label">Live Tail</div>
                          <pre className="artifact-pre artifact-pre-medium">
                            {tail.data?.lines.join("\n") ?? "加载 tail…"}
                          </pre>
                        </div>
                      ) : null}
                    </>
                  ) : (
                    <div className="empty-box">选择一个 artifact 查看。</div>
                  )}
                </Panel>
              </div>
            ) : null}
          </Panel>
        </>
      ) : null}
    </div>
  );
}

function SummaryList({ title, items }: { title: string; items: string[] }) {
  return (
    <div className="summary-list">
      <div className="section-label">{title}</div>
      {items.length === 0 ? (
        <div className="empty-box">—</div>
      ) : (
        <ul className="evidence-list">
          {items.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
