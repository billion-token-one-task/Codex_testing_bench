import { useEffect, useMemo, useState } from "react";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { SegmentedTabs } from "../components/SegmentedTabs";
import { useWorkspaceIndex } from "../lib/store";
import { api } from "../lib/api";
import type { ArtifactDescriptor } from "../lib/types";

export function ArtifactsPage() {
  const { data } = useWorkspaceIndex();
  const [campaignId, setCampaignId] = useState<string>("");
  const [mode, setMode] = useState("reports");
  const [artifacts, setArtifacts] = useState<ArtifactDescriptor[]>([]);
  const [selectedArtifact, setSelectedArtifact] = useState<ArtifactDescriptor | null>(null);
  const campaigns = data?.campaigns ?? [];

  const activeCampaign = useMemo(
    () => campaigns.find((campaign) => campaign.campaign_id === campaignId) ?? campaigns[0] ?? null,
    [campaignId, campaigns],
  );

  useEffect(() => {
    if (!activeCampaign) return;
    setCampaignId(activeCampaign.campaign_id);
    void (async () => {
      const rows = mode === "datasets"
        ? await api.campaignDatasets(activeCampaign.campaign_id) as ArtifactDescriptor[]
        : await api.campaignReports(activeCampaign.campaign_id) as ArtifactDescriptor[];
      setArtifacts(rows);
      setSelectedArtifact(rows[0] ?? null);
    })();
  }, [activeCampaign?.campaign_id, mode]);

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Artifact Browser"
        title="Artifacts"
        description="直接浏览 campaign 级 reports、datasets 和文本证据。适合快速核对 markdown、CSV、JSONL 与 patch diff。"
      />

      <div className="page-grid page-grid-2">
        <Panel
          title="Artifact Inventory"
          kicker={activeCampaign?.campaign_id ?? "Select campaign"}
          actions={
            <SegmentedTabs
              items={[
                { value: "reports", label: "Reports" },
                { value: "datasets", label: "Datasets" },
              ]}
              value={mode}
              onChange={setMode}
            />
          }
        >
          <div className="filter-row">
            <select value={activeCampaign?.campaign_id ?? ""} onChange={(event) => setCampaignId(event.target.value)}>
              {campaigns.map((campaign) => (
                <option key={campaign.campaign_id} value={campaign.campaign_id}>
                  {campaign.experiment_name} / {campaign.campaign_id}
                </option>
              ))}
            </select>
          </div>
          <div className="artifact-list artifact-list-column">
            {artifacts.map((artifact) => (
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
        </Panel>

        <Panel title="Artifact Preview" kicker={selectedArtifact?.name ?? "Choose artifact"}>
          <ArtifactViewer artifact={selectedArtifact} />
        </Panel>
      </div>
    </div>
  );
}
