import { useEffect, useMemo, useState } from "react";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { StatusBadge } from "../components/StatusBadge";
import { api } from "../lib/api";
import { formatCompact, formatDate, formatNumber, truncateMiddle } from "../lib/format";
import { useWorkspaceIndex } from "../lib/store";
import type { ArtifactDescriptor, CampaignListItem } from "../lib/types";

export function CampaignsPage() {
  const { data, loading, error } = useWorkspaceIndex();
  const campaigns = data?.campaigns ?? [];
  const [selectedCampaignId, setSelectedCampaignId] = useState<string>("");
  const [reports, setReports] = useState<ArtifactDescriptor[]>([]);
  const [datasets, setDatasets] = useState<ArtifactDescriptor[]>([]);
  const [selectedArtifact, setSelectedArtifact] = useState<ArtifactDescriptor | null>(null);
  const [artifactMode, setArtifactMode] = useState("reports");

  const activeCampaign = useMemo(() => {
    return campaigns.find((campaign) => campaign.campaign_id === selectedCampaignId) ?? campaigns[0] ?? null;
  }, [campaigns, selectedCampaignId]);

  useEffect(() => {
    if (!activeCampaign) return;
    setSelectedCampaignId(activeCampaign.campaign_id);
    void (async () => {
      const [nextReports, nextDatasets] = await Promise.all([
        api.campaignReports(activeCampaign.campaign_id) as Promise<ArtifactDescriptor[]>,
        api.campaignDatasets(activeCampaign.campaign_id) as Promise<ArtifactDescriptor[]>,
      ]);
      setReports(nextReports);
      setDatasets(nextDatasets);
      const first = artifactMode === "datasets" ? nextDatasets[0] : nextReports[0];
      setSelectedArtifact(first ?? nextReports[0] ?? nextDatasets[0] ?? null);
    })();
  }, [activeCampaign?.campaign_id, artifactMode]);

  const activeArtifacts = artifactMode === "datasets" ? datasets : reports;

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Campaign Control Room"
        title="Campaigns"
        description="查看所有实验批次、cohort 规模、运行状态、报告数量与数据集产物。这里是整个研究系统的运行总账。"
      />

      <div className="page-grid page-grid-3">
        <MetricCard
          label="Indexed Campaigns"
          value={formatNumber(data?.summary.campaign_count)}
          detail={`${formatNumber(data?.summary.run_count)} runs total`}
          tone="signal"
        />
        <MetricCard
          label="Live Campaigns"
          value={formatNumber(data?.summary.active_run_count)}
          detail={`${formatCompact(data?.summary.total_tool_calls)} tool calls observed`}
          tone="pressure"
        />
        <MetricCard
          label="Signal Volume"
          value={formatCompact(data?.summary.total_tokens)}
          detail={`${formatCompact(data?.summary.total_visible_output_tokens_est)} visible tokens`}
          tone="verify"
        />
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Campaign Ledger" kicker="Experiments / cohorts / artifact surface">
          {loading ? <div className="empty-box">加载 workspace index…</div> : null}
          {error ? <div className="empty-box">{error}</div> : null}
          <div className="table-wrap">
            <table className="ledger-table">
              <thead>
                <tr>
                  <th>Campaign</th>
                  <th>Status</th>
                  <th>Benchmark</th>
                  <th>Sample</th>
                  <th>Cohorts</th>
                  <th>Reports</th>
                  <th>Datasets</th>
                  <th>Created</th>
                </tr>
              </thead>
              <tbody>
                {campaigns.map((campaign) => (
                  <tr key={campaign.campaign_id} onClick={() => setSelectedCampaignId(campaign.campaign_id)}>
                    <td>
                      <div className="cell-stack">
                        <strong>{campaign.experiment_name}</strong>
                        <span className="mono-note">{truncateMiddle(campaign.campaign_id, 52)}</span>
                      </div>
                    </td>
                    <td><StatusBadge tone={statusTone(campaign.status)}>{campaign.status}</StatusBadge></td>
                    <td>
                      <div className="cell-stack">
                        <span>{campaign.benchmark_name}</span>
                        <span className="mono-note">{campaign.stage_name ?? "—"}</span>
                      </div>
                    </td>
                    <td>{campaign.sample_size}</td>
                    <td>{campaign.cohort_count}</td>
                    <td>{campaign.report_count}</td>
                    <td>{campaign.dataset_count}</td>
                    <td>{formatDate(campaign.created_at)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Panel>

        <Panel
          title="Campaign Dossier"
          kicker={activeCampaign?.campaign_id ?? "Select a campaign"}
          actions={
            <SegmentedTabs
              items={[
                { value: "reports", label: "Reports" },
                { value: "datasets", label: "Datasets" },
              ]}
              value={artifactMode}
              onChange={(value) => setArtifactMode(value)}
            />
          }
        >
          {activeCampaign ? <CampaignDossier campaign={activeCampaign} /> : <div className="empty-box">选择一个 campaign。</div>}
          <div className="artifact-browser">
            <div className="artifact-list artifact-list-column">
              {activeArtifacts.map((artifact) => (
                <button
                  key={artifact.path}
                  className={`artifact-chip${selectedArtifact?.path === artifact.path ? " artifact-chip-active" : ""}`}
                  onClick={() => setSelectedArtifact(artifact)}
                >
                  <span>{artifact.name}</span>
                  <span className="artifact-kind">{artifact.kind}</span>
                </button>
              ))}
            </div>
            <ArtifactViewer artifact={selectedArtifact} />
          </div>
        </Panel>
      </div>
    </div>
  );
}

function CampaignDossier({ campaign }: { campaign: CampaignListItem }) {
  return (
    <div className="metric-grid metric-grid-4">
      <MetricCard label="Sample Size" value={campaign.sample_size} detail={`${campaign.selected_instances} selected instances`} />
      <MetricCard label="Cohorts" value={campaign.cohort_count} detail={`parallel ${campaign.max_parallel_runs}`} />
      <MetricCard label="Tool Calls" value={formatCompact(campaign.total_tool_calls)} detail={`${formatCompact(campaign.total_commands)} commands`} tone="signal" />
      <MetricCard label="Visible Output" value={formatCompact(campaign.total_visible_output_tokens_est)} detail={`${formatCompact(campaign.total_tokens)} total tokens`} tone="pressure" />
      <MetricCard label="Completed" value={campaign.completed_run_count} detail={`failed ${campaign.failed_run_count}`} tone="verify" />
      <MetricCard label="Active" value={campaign.active_run_count} detail={campaign.path} tone="neutral" />
    </div>
  );
}

function statusTone(status: string) {
  if (status.includes("running")) return "running";
  if (status.includes("graded") || status.includes("completed")) return "completed";
  if (status.includes("failed") || status.includes("error")) return "failed";
  return "neutral";
}
