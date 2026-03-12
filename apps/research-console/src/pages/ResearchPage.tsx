import { useEffect, useMemo, useState } from "react";

import { DataTable } from "../components/DataTable";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { api } from "../lib/api";
import { formatCompact } from "../lib/format";
import { useWorkspaceIndex } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";

export function ResearchPage() {
  const { data } = useWorkspaceIndex();
  const campaigns = data?.campaigns ?? [];
  const [campaignId, setCampaignId] = useState("");
  const [claimRows, setClaimRows] = useState<Array<Record<string, string>>>([]);
  const [taskRows, setTaskRows] = useState<Array<Record<string, string>>>([]);
  const [personalityRows, setPersonalityRows] = useState<Array<Record<string, string>>>([]);

  const activeCampaign = useMemo(
    () => campaigns.find((campaign) => campaign.campaign_id === campaignId) ?? campaigns[0] ?? null,
    [campaignId, campaigns],
  );

  useEffect(() => {
    if (!activeCampaign) return;
    setCampaignId(activeCampaign.campaign_id);
    void (async () => {
      const artifacts = await api.campaignDatasets(activeCampaign.campaign_id) as ArtifactDescriptor[];
      const claimFile = artifacts.find((artifact) => artifact.name === "claim_evidence.csv");
      const taskFile = artifacts.find((artifact) => artifact.name === "task_class_summary.csv");
      const personalityFile = artifacts.find((artifact) => artifact.name === "personality_mechanism.csv");
      const [claims, tasks, personality] = await Promise.all([
        claimFile ? api.artifactFile(claimFile.path, "csv") : Promise.resolve(null),
        taskFile ? api.artifactFile(taskFile.path, "csv") : Promise.resolve(null),
        personalityFile ? api.artifactFile(personalityFile.path, "csv") : Promise.resolve(null),
      ]);
      setClaimRows(claims?.payload.kind === "csv" ? claims.payload.rows : []);
      setTaskRows(tasks?.payload.kind === "csv" ? tasks.payload.rows : []);
      setPersonalityRows(personality?.payload.kind === "csv" ? personality.payload.rows : []);
    })();
  }, [activeCampaign?.campaign_id]);

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Research Workbench"
        title="Research"
        description="围绕 hypothesis、claim evidence、task class 与 personality 机制整理研究证据。这里优先服务写 memo、写论文、做配对解释。"
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Hypothesis Surface" value="H1–H6" detail="模型代际 × personality × harness 机制" tone="signal" />
        <MetricCard label="Claim Rows" value={claimRows.length} detail="grounding + codex-unique evidence" tone="pressure" />
        <MetricCard label="Task-Class Rows" value={taskRows.length} detail="benchmark-specific behavior buckets" tone="verify" />
        <MetricCard label="Mechanism Rows" value={formatCompact(personalityRows.length)} detail="personality / instruction / fallback evidence" />
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Research Focus" kicker="Current thesis line">
          <ul className="evidence-list">
            <li>5.4 是否比 5.3-codex 更愿意把状态外显成用户可见语言。</li>
            <li>friendly 与 pragmatic 的差异是语气级，还是会改变工具编排与验证节奏。</li>
            <li>语言输出、tool use、patch chain 与 personality 机制如何一起形成 agent policy。</li>
            <li>哪些差异更像模型因素，哪些差异更像 harness 中介效应。</li>
          </ul>
        </Panel>

        <Panel title="Campaign Selector" kicker="Research target">
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
            <div className="metric-grid">
              <div>
                <span className="metric-label">Benchmark</span>
                <strong>{activeCampaign.benchmark_name}</strong>
              </div>
              <div>
                <span className="metric-label">Sample</span>
                <strong>{activeCampaign.sample_size}</strong>
              </div>
              <div>
                <span className="metric-label">Reports</span>
                <strong>{activeCampaign.report_count}</strong>
              </div>
              <div>
                <span className="metric-label">Datasets</span>
                <strong>{activeCampaign.dataset_count}</strong>
              </div>
            </div>
          ) : null}
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Claim Evidence" kicker="claim_evidence.csv">
          <DataTable rows={claimRows as Array<Record<string, unknown>>} />
        </Panel>
        <Panel title="Task-Class Summary" kicker="task_class_summary.csv">
          <DataTable rows={taskRows as Array<Record<string, unknown>>} />
        </Panel>
      </div>

      <Panel title="Personality Mechanism Surface" kicker="personality_mechanism.csv">
        <DataTable rows={personalityRows as Array<Record<string, unknown>>} />
      </Panel>
    </div>
  );
}
