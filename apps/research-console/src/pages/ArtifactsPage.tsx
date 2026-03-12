import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { EventRail } from "../components/EventRail";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { StateNotice } from "../components/StateNotice";
import { formatCompact } from "../lib/format";
import { useArtifactTail, useCampaignDetail, useCampaignOperationalSummary, useCampaignSelection, useRecentEventTypes, useRunDetail, useRunOperationalSummary, useWorkspaceIndex } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";

type Scope = "campaign" | "run";
type Mode = "reports" | "datasets";

export function ArtifactsPage() {
  const { data } = useWorkspaceIndex();
  const campaigns = data?.campaigns ?? [];
  const runs = data?.runs ?? [];
  const [campaignId, setCampaignId] = useState<string>("");
  const [runId, setRunId] = useState<string>("");
  const [scope, setScope] = useState<Scope>("campaign");
  const [mode, setMode] = useState<Mode>("reports");
  const [selectedArtifactPath, setSelectedArtifactPath] = useState<string | null>(null);
  const activeCampaign = useCampaignSelection(campaigns, campaignId);
  const campaignDetail = useCampaignDetail(activeCampaign?.campaign_id ?? "");
  const campaignOperational = useCampaignOperationalSummary(activeCampaign?.campaign_id ?? "");
  const runDetail = useRunDetail(runId);
  const runOperational = useRunOperationalSummary(runId);
  const recentArtifactEvents = useRecentEventTypes(["artifact.updated", "campaign.artifact.updated", "workspace.updated"], 16);

  useEffect(() => {
    if (!activeCampaign) return;
    setCampaignId(activeCampaign.campaign_id);
  }, [activeCampaign?.campaign_id]);

  useEffect(() => {
    if (activeCampaign && !runId) {
      const firstRun = runs.find((run) => run.campaign_id === activeCampaign.campaign_id);
      if (firstRun) setRunId(firstRun.run_id);
    }
  }, [activeCampaign?.campaign_id, runId, runs]);

  const campaignArtifacts = useMemo(() => {
    if (!campaignDetail.data) return [] as ArtifactDescriptor[];
    if (mode === "datasets") return campaignDetail.data.datasets;
    return campaignDetail.data.reports;
  }, [campaignDetail.data, mode]);

  const runArtifacts = runDetail.data?.attempt_artifacts ?? [];
  const activeArtifacts = scope === "run" ? runArtifacts : campaignArtifacts;
  const artifactGroups = useMemo(() => {
    return activeArtifacts.reduce<Record<string, ArtifactDescriptor[]>>((acc, artifact) => {
      const key = artifact.role ?? artifact.kind ?? "artifact";
      acc[key] ??= [];
      acc[key].push(artifact);
      return acc;
    }, {});
  }, [activeArtifacts]);
  const artifactRoleCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const artifact of activeArtifacts) {
      const key = artifact.role ?? artifact.kind ?? "artifact";
      counts[key] = (counts[key] ?? 0) + 1;
    }
    return counts;
  }, [activeArtifacts]);
  const artifactFormatCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const artifact of activeArtifacts) {
      const key = artifact.format ?? "unknown";
      counts[key] = (counts[key] ?? 0) + 1;
    }
    return counts;
  }, [activeArtifacts]);
  const roleFamilies = useMemo(() => {
    const counts = {
      raw_truth: 0,
      derived_summary: 0,
      derived_evidence: 0,
      human_readable_dossier: 0,
      other: 0,
    };
    for (const artifact of activeArtifacts) {
      const role = artifact.role ?? "";
      if (role.includes("raw_truth")) counts.raw_truth += 1;
      else if (role.includes("derived_summary")) counts.derived_summary += 1;
      else if (role.includes("derived_evidence")) counts.derived_evidence += 1;
      else if (role.includes("human_readable_dossier")) counts.human_readable_dossier += 1;
      else counts.other += 1;
    }
    return counts;
  }, [activeArtifacts]);
  const selectedArtifact = activeArtifacts.find((artifact) => artifact.path === selectedArtifactPath) ?? activeArtifacts[0] ?? null;
  const tail = useArtifactTail(selectedArtifact?.path ?? null, 90, Boolean(selectedArtifact));

  useEffect(() => {
    setSelectedArtifactPath(activeArtifacts[0]?.path ?? null);
  }, [scope, mode, runId, campaignId, activeArtifacts]);

  const campaignRuns = useMemo(
    () => runs.filter((run) => run.campaign_id === activeCampaign?.campaign_id),
    [activeCampaign?.campaign_id, runs],
  );

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Evidence Archive"
        title="Artifacts"
        description="这里是证据档案室。你可以在 campaign 级看 reports / datasets，也可以切到单个 run 看 attempt 级 artifact、日志、raw / normalized 文件。"
      />

      <Panel
        title="Artifact Scope"
        kicker="Campaign-level reports / datasets or run-level attempt artifacts"
        actions={
          <SegmentedTabs
            items={[
              { value: "campaign", label: "Campaign" },
              { value: "run", label: "Run" },
            ]}
            value={scope}
            onChange={(value) => setScope(value as Scope)}
          />
        }
      >
        {!activeCampaign ? (
          <StateNotice
            title="当前还没有可浏览的 campaign"
            body="Artifacts 页会在 workspace 索引和 campaign 产物建立后自动充实。短暂空白不代表 bench 没有在跑。"
            tone="loading"
          />
        ) : null}
        <div className="filter-row filter-row-wide">
          <select value={activeCampaign?.campaign_id ?? ""} onChange={(event) => setCampaignId(event.target.value)}>
            {campaigns.map((campaign) => (
              <option key={campaign.campaign_id} value={campaign.campaign_id}>
                {campaign.experiment_name} / {campaign.campaign_id}
              </option>
            ))}
          </select>
          {scope === "campaign" ? (
            <select value={mode} onChange={(event) => setMode(event.target.value as Mode)}>
              <option value="reports">reports</option>
              <option value="datasets">datasets</option>
            </select>
          ) : (
            <select value={runId} onChange={(event) => setRunId(event.target.value)}>
              {campaignRuns.map((run) => (
                <option key={run.run_id} value={run.run_id}>
                  {run.instance_id} / {run.cohort_id}
                </option>
              ))}
            </select>
          )}
        </div>

        <KeyValueGrid
          columns={4}
          items={[
            { label: "Scope", value: scope },
            { label: "Campaign", value: activeCampaign?.experiment_name ?? "—" },
            { label: "Artifacts", value: activeArtifacts.length },
            { label: "Selected", value: selectedArtifact?.name ?? "—" },
            { label: "Rows", value: formatCompact(selectedArtifact?.row_count), detail: formatCompact(selectedArtifact?.line_count) },
            { label: "Bytes", value: formatCompact(selectedArtifact?.size_bytes), detail: selectedArtifact?.format ?? "—" },
            { label: "Role", value: selectedArtifact?.role ?? "—", detail: selectedArtifact?.scope ?? "—" },
            { label: "Preview", value: selectedArtifact?.previewable ? "yes" : "no" },
                { label: "Run", value: scope === "run" ? runDetail.data?.run.instance_id ?? "—" : "campaign scope" },
                { label: "Cohort", value: scope === "run" ? runDetail.data?.run.cohort_id ?? "—" : activeCampaign?.cohort_count ?? "—" },
                { label: "Status", value: scope === "run" ? runDetail.data?.run.status ?? "—" : activeCampaign?.status ?? "—" },
                { label: "Dataset/Report", value: scope === "campaign" ? mode : selectedArtifact?.kind ?? "artifact" },
              ]}
            />
            <div className="chip-row">
              {scope === "run" && runDetail.data ? (
                <Link className="artifact-chip" to={`/runs/${runDetail.data.run.run_id}`}>
                  open war room
                </Link>
              ) : null}
              {activeCampaign ? (
                <Link className="artifact-chip" to="/campaigns">
                  campaign desk
                </Link>
              ) : null}
              <Link className="artifact-chip" to="/research">
                research desk
              </Link>
            </div>
          </Panel>

      <div className="page-grid page-grid-2">
        <Panel title="Archive Pulse" kicker="Latest artifact / dataset / report movement">
          {!recentArtifactEvents.length ? (
            <StateNotice
              title="artifact pulse 还很安静"
              body="如果 run 还在前期搜索阶段，新的 report / dataset / artifact append 事件会比较少。"
              tone="loading"
            />
          ) : (
            <EventRail rows={recentArtifactEvents} emptyLabel="等待 artifact pulse。" />
          )}
        </Panel>

        {scope === "campaign" ? (
          <Panel title="Campaign Operational Dossier" kicker="Latest reports / datasets / live status">
            <KeyValueGrid
              columns={4}
              items={[
                { label: "Active Live Runs", value: campaignOperational.data?.active_live_runs.length ?? 0, detail: `${campaignOperational.data?.active_process_count ?? 0} processes`, tone: "signal" },
                { label: "Visible / Total Tok", value: `${formatCompact(campaignOperational.data?.live_visible_output_total_tokens_est)} / ${formatCompact(campaignOperational.data?.live_total_tokens)}` },
                { label: "Warnings", value: campaignOperational.data?.active_warning_count ?? 0, detail: campaignOperational.data?.operational_warnings[0] ?? "none", tone: "anomaly" },
                { label: "Heat", value: Object.entries(campaignOperational.data?.heat_counts ?? {}).map(([name, count]) => `${name}×${count}`).join(" · ") || "—" },
                { label: "Latest Reports", value: campaignOperational.data?.latest_reports.length ?? 0, detail: campaignOperational.data?.latest_reports[0]?.name ?? "—" },
                { label: "Latest Datasets", value: campaignOperational.data?.latest_datasets.length ?? 0, detail: campaignOperational.data?.latest_datasets[0]?.name ?? "—" },
                { label: "Focus Samples", value: campaignOperational.data?.focus_samples.slice(0, 2).join(" · ") || "—" },
                { label: "Message Previews", value: campaignOperational.data?.latest_message_previews.slice(0, 1)[0] ?? "—" },
              ]}
            />
            {campaignOperational.data?.operational_warnings.length ? (
              <>
                <div className="panel-divider" />
                <div className="warning-tape warning-tape-block">
                  {campaignOperational.data.operational_warnings.map((warning) => (
                    <span key={warning}>{warning}</span>
                  ))}
                </div>
              </>
            ) : null}
          </Panel>
        ) : null}

        {scope === "run" ? (
          <Panel title="Run Operational Dossier" kicker="Artifact readiness / live phase / warnings">
            <KeyValueGrid
              columns={4}
              items={[
                { label: "Current Phase", value: runOperational.data?.current_phase ?? "—", detail: runOperational.data?.latest_focus ?? "—", tone: "authority" },
                { label: "Live Warnings", value: runOperational.data?.live_warning_count ?? 0, detail: runOperational.data?.operational_warnings[0] ?? "none", tone: "anomaly" },
                { label: "Latest Tool", value: runOperational.data?.latest_tool ?? "—" },
                { label: "Latest Patch", value: runOperational.data?.latest_patch ?? "—" },
                { label: "Latest Message", value: runOperational.data?.latest_message_preview ?? "—" },
                { label: "Latest Command", value: runOperational.data?.latest_command ?? "—" },
                { label: "Reports / Datasets", value: `${runOperational.data?.latest_reports.length ?? 0} / ${runOperational.data?.latest_datasets.length ?? 0}` },
                { label: "Attempt Artifacts", value: runOperational.data?.attempt_artifact_count ?? 0 },
              ]}
            />
            {runOperational.data?.operational_warnings.length ? (
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
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Archive Reading Guide" kicker="How to navigate this evidence archive">
          <ul className="evidence-list">
            <li>`reports` 适合快速理解结论与 narrative；`datasets` 适合做系统对比和画图。</li>
            <li>`raw_truth` 优先级最高；`derived_summary / derived_evidence` 要结合 observability contract 一起读。</li>
            <li>run scope 下优先看 `run-evidence.txt`、`attempt-log.txt`、`raw-agent-events.jsonl`、`codex-probe-events.jsonl`。</li>
            <li>campaign scope 下优先看 `report.txt`、`model-comparison.md`、`tool-language-coupling.md` 与 `datasets/*.csv`。</li>
          </ul>
        </Panel>

        <Panel title="Selected Artifact Dossier" kicker="Why this file matters in the research workflow">
          {selectedArtifact ? (
            <KeyValueGrid
              columns={2}
              items={[
                { label: "Name", value: selectedArtifact.name, detail: selectedArtifact.path },
                { label: "Role", value: selectedArtifact.role ?? "—", detail: selectedArtifact.kind },
                { label: "Format", value: selectedArtifact.format ?? "—", detail: selectedArtifact.previewable ? "previewable" : "opaque" },
                { label: "Scope", value: selectedArtifact.scope ?? "—" },
                { label: "Truth Layer", value: selectedArtifact.role?.includes("raw_truth") ? "observed" : selectedArtifact.role?.includes("derived") ? "derived" : selectedArtifact.role?.includes("human_readable") ? "dossier" : "other" },
                { label: "Rows / Lines", value: `${formatCompact(selectedArtifact.row_count)} / ${formatCompact(selectedArtifact.line_count)}` },
                { label: "Bytes", value: formatCompact(selectedArtifact.size_bytes), detail: selectedArtifact.updated_at ?? "—" },
                { label: "Preview", value: selectedArtifact.previewable ? "yes" : "no" },
              ]}
            />
          ) : (
            <StateNotice
              title="先选择一个 artifact"
              body="选中文件后，这里会说明它在整个研究流水线里的角色，以及应该把它当作 observed、derived 还是 dossier 来读。"
              tone="info"
            />
          )}
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Artifact Classification Summary" kicker="Role / format / previewability">
          {!activeArtifacts.length ? (
            <StateNotice
              title="当前 scope 下还没有 artifact"
              body={scope === "campaign"
                ? "campaign 级 reports / datasets 会在 run 或 report 完成后出现。"
                : "run 级 attempt artifact 会在该题进入真实求解后逐步落盘。"}
              tone="info"
            />
          ) : null}
          <div className="artifact-summary-grid">
            <div className="focus-note">
              <span className="metric-label">Raw Truth</span>
              <strong>{roleFamilies.raw_truth}</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Derived Summary</span>
              <strong>{roleFamilies.derived_summary}</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Derived Evidence</span>
              <strong>{roleFamilies.derived_evidence}</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Human Dossier</span>
              <strong>{roleFamilies.human_readable_dossier}</strong>
            </div>
            {Object.entries(artifactRoleCounts).map(([role, count]) => (
              <div key={role} className="focus-note">
                <span className="metric-label">{role}</span>
                <strong>{count}</strong>
              </div>
            ))}
            {Object.entries(artifactFormatCounts).map(([format, count]) => (
              <div key={format} className="focus-note">
                <span className="metric-label">{format}</span>
                <strong>{count}</strong>
              </div>
            ))}
            <div className="focus-note">
              <span className="metric-label">Previewable</span>
              <strong>{activeArtifacts.filter((artifact) => artifact.previewable).length}</strong>
            </div>
          </div>
        </Panel>

        <Panel title="Reading Order" kicker="How to read this archive like a research evidence stack">
          <div className="focus-grid">
            <div className="focus-note">
              <span className="metric-label">1</span>
              <strong>campaign report / model comparison</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">2</span>
              <strong>run-evidence / attempt-log</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">3</span>
              <strong>datasets csv</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">4</span>
              <strong>raw truth jsonl</strong>
            </div>
          </div>
          <div className="panel-divider" />
          <ul className="evidence-list">
            <li>先读 dossier，再读 derived evidence，最后才回 raw truth 校正。</li>
            <li>如果你要写结论，先确认 artifact 的 truth level 和 observability layer。</li>
            <li>run scope 适合解释个例，campaign scope 适合做 paired / aggregate 比较。</li>
          </ul>
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Artifact Inventory" kicker={scope === "campaign" ? "campaign reports / datasets" : "run attempt files"}>
          {!Object.keys(artifactGroups).length ? (
            <StateNotice
              title="artifact inventory 暂时为空"
              body="当前 scope 还没有可索引的 artifact；等 report、dataset 或 attempt 文件落盘后，这里会自动刷新。"
              tone="loading"
            />
          ) : null}
          <div className="artifact-group-stack">
            {Object.entries(artifactGroups).map(([group, rows]) => (
              <div key={group} className="artifact-group">
                <div className="section-label">{group}</div>
                <div className="artifact-list artifact-list-column artifact-ledger">
                  {rows.map((artifact) => (
                    <button
                      key={artifact.path}
                      className={`artifact-chip${selectedArtifact?.path === artifact.path ? " artifact-chip-active" : ""}`}
                      onClick={() => setSelectedArtifactPath(artifact.path)}
                    >
                      <span>{artifact.name}</span>
                      <span className="artifact-kind">{artifact.role ?? artifact.kind}</span>
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </Panel>

        <Panel title="Artifact Preview" kicker={selectedArtifact?.name ?? "Choose artifact"}>
          <ArtifactViewer artifact={selectedArtifact} />
          {selectedArtifact ? (
            <>
              <div className="panel-divider" />
              <KeyValueGrid
                columns={2}
                items={[
                  { label: "Role", value: selectedArtifact.role ?? "—", detail: selectedArtifact.scope ?? "—" },
                  { label: "Format", value: selectedArtifact.format ?? "—", detail: selectedArtifact.previewable ? "previewable" : "opaque" },
                  { label: "Rows / Lines", value: formatCompact(selectedArtifact.row_count), detail: formatCompact(selectedArtifact.line_count) },
                  { label: "Bytes", value: formatCompact(selectedArtifact.size_bytes), detail: selectedArtifact.updated_at ?? "—" },
                  {
                    label: "Truth Level",
                    value:
                      selectedArtifact.role?.includes("raw_truth") ? "observed" :
                      selectedArtifact.role?.includes("derived") ? "derived" :
                      selectedArtifact.role?.includes("human_readable") ? "dossier" :
                      "other",
                    detail: selectedArtifact.role ?? "—",
                  },
                  { label: "Path", value: selectedArtifact.path },
                ]}
              />
              <div className="tail-box">
                <div className="section-label">Artifact Tail</div>
                <pre className="artifact-pre artifact-pre-medium">
                  {tail.data?.lines.join("\n") ?? "加载 tail…"}
                </pre>
              </div>
            </>
          ) : null}
        </Panel>
      </div>
    </div>
  );
}
