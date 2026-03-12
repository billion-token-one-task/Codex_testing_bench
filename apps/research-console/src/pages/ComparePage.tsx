import { useEffect, useMemo, useState } from "react";

import { DataTable } from "../components/DataTable";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { StatusBadge } from "../components/StatusBadge";
import { formatCompact } from "../lib/format";
import { useWorkspaceIndex } from "../lib/store";
import { api } from "../lib/api";
import type { ArtifactDescriptor } from "../lib/types";

export function ComparePage() {
  const { data } = useWorkspaceIndex();
  const campaigns = data?.campaigns ?? [];
  const [campaignId, setCampaignId] = useState("");
  const [datasets, setDatasets] = useState<ArtifactDescriptor[]>([]);
  const [pairRows, setPairRows] = useState<Array<Record<string, string>>>([]);
  const [campaignRunRows, setCampaignRunRows] = useState<Array<Record<string, string>>>([]);
  const [messageRows, setMessageRows] = useState<Array<Record<string, string>>>([]);

  const activeCampaign = useMemo(
    () => campaigns.find((campaign) => campaign.campaign_id === campaignId) ?? campaigns[0] ?? null,
    [campaignId, campaigns],
  );

  useEffect(() => {
    if (!activeCampaign) return;
    setCampaignId(activeCampaign.campaign_id);
    void (async () => {
      const nextDatasets = await api.campaignDatasets(activeCampaign.campaign_id) as ArtifactDescriptor[];
      setDatasets(nextDatasets);
      const pairArtifact = nextDatasets.find((artifact) => artifact.name === "model_pair_deltas.csv");
      const campaignRunsArtifact = nextDatasets.find((artifact) => artifact.name === "campaign_runs.csv");
      const messageArtifact = nextDatasets.find((artifact) => artifact.name === "message_style.csv");
      const [pairs, runs, messages] = await Promise.all([
        pairArtifact ? api.artifactFile(pairArtifact.path, "csv") : Promise.resolve(null),
        campaignRunsArtifact ? api.artifactFile(campaignRunsArtifact.path, "csv") : Promise.resolve(null),
        messageArtifact ? api.artifactFile(messageArtifact.path, "csv") : Promise.resolve(null),
      ]);
      setPairRows(pairs?.payload.kind === "csv" ? pairs.payload.rows : []);
      setCampaignRunRows(runs?.payload.kind === "csv" ? runs.payload.rows : []);
      setMessageRows(messages?.payload.kind === "csv" ? messages.payload.rows : []);
    })();
  }, [activeCampaign?.campaign_id]);

  const pairHighlights = useMemo(() => pairRows.slice(0, 8), [pairRows]);
  const socialToneMax = useMemo(() => {
    return messageRows.reduce((max, row) => {
      const value = Number(row.social_tone_ratio_bps ?? "0");
      return Number.isFinite(value) ? Math.max(max, value) : max;
    }, 0);
  }, [messageRows]);

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Pairwise Research Workbench"
        title="Compare"
        description="专门用于看同题四格、cohort pair 差分、verbosity 与 tool coupling 的比较证据。这里优先服务模型代际与 personality 机制研究。"
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Pair Rows" value={pairRows.length} detail={`${campaignRunRows.length} run rows`} tone="signal" />
        <MetricCard label="Message Rows" value={formatCompact(messageRows.length)} detail="style + discourse evidence" tone="pressure" />
        <MetricCard label="Latest Campaign" value={activeCampaign?.cohort_count ?? 0} detail={`${activeCampaign?.sample_size ?? 0} paired tasks`} tone="verify" />
        <MetricCard label="Social Tone Peak" value={socialToneMax ? `${(socialToneMax / 100).toFixed(1)}%` : "—"} detail="max message-level social tone score" />
      </div>

      <Panel title="Comparison Target" kicker="Select campaign">
        <div className="filter-row">
          <select value={activeCampaign?.campaign_id ?? ""} onChange={(event) => setCampaignId(event.target.value)}>
            {campaigns.map((campaign) => (
              <option key={campaign.campaign_id} value={campaign.campaign_id}>
                {campaign.experiment_name} / {campaign.campaign_id}
              </option>
            ))}
          </select>
        </div>
        {activeCampaign ? (
          <div className="metric-grid metric-grid-4">
            <MetricCard label="Benchmark" value={activeCampaign.benchmark_name} detail={activeCampaign.stage_name ?? "—"} />
            <MetricCard label="Reports" value={activeCampaign.report_count} detail={`${activeCampaign.dataset_count} datasets`} />
            <MetricCard label="Tool Calls" value={formatCompact(activeCampaign.total_tool_calls)} detail={`${formatCompact(activeCampaign.total_commands)} commands`} tone="signal" />
            <MetricCard label="Visible Output" value={formatCompact(activeCampaign.total_visible_output_tokens_est)} detail={`${formatCompact(activeCampaign.total_tokens)} total tokens`} tone="pressure" />
          </div>
        ) : null}
      </Panel>

      <div className="page-grid page-grid-2">
        <Panel title="Cohort Pair Highlights" kicker="Top pairwise deltas">
          <div className="compare-stack">
            {pairHighlights.map((row, index) => (
              <div key={`${row.instance_id}-${index}`} className="compare-block">
                <div className="compare-heading">{row.instance_id ?? "instance"}</div>
                <div className="brief-meta">
                  <span>{row.left_cohort ?? "left"}</span>
                  <span>vs</span>
                  <span>{row.right_cohort ?? "right"}</span>
                </div>
                <div className="metric-grid">
                  <div>
                    <span className="metric-label">Visible Δ</span>
                    <strong>{row.visible_output_total_tokens_est_delta ?? "—"}</strong>
                  </div>
                  <div>
                    <span className="metric-label">Tool Δ</span>
                    <strong>{row.tool_count_delta ?? "—"}</strong>
                  </div>
                  <div>
                    <span className="metric-label">Command Δ</span>
                    <strong>{row.command_count_delta ?? "—"}</strong>
                  </div>
                  <div>
                    <span className="metric-label">Narration Δ</span>
                    <strong>{row.micro_narrated_tool_burst_count_delta ?? "—"}</strong>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </Panel>

        <Panel title="2x2 Run Matrix" kicker="Same-task cohort grid">
          <div className="table-wrap">
            <table className="ledger-table">
              <thead>
                <tr>
                  <th>Instance</th>
                  <th>Cohort</th>
                  <th>Status</th>
                  <th>Visible Output</th>
                  <th>Tools</th>
                  <th>Messages</th>
                </tr>
              </thead>
              <tbody>
                {campaignRunRows.slice(0, 40).map((row, index) => (
                  <tr key={`${row.run_id}-${index}`}>
                    <td>{row.instance_id}</td>
                    <td>{row.cohort_id}</td>
                    <td><StatusBadge tone={row.status === "completed" ? "completed" : row.status === "running" ? "running" : "failed"}>{row.status}</StatusBadge></td>
                    <td>{row.visible_output_total_tokens_est}</td>
                    <td>{row.tool_count}</td>
                    <td>{row.message_metric_count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Panel>
      </div>

      <Panel title="Pairwise Dataset" kicker="model_pair_deltas.csv">
        <DataTable rows={pairRows as Array<Record<string, unknown>>} />
      </Panel>
    </div>
  );
}
