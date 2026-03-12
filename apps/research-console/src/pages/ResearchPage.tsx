import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { ArtifactViewer } from "../components/ArtifactViewer";
import { DataTable } from "../components/DataTable";
import { EventRail } from "../components/EventRail";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { StateNotice } from "../components/StateNotice";
import { api } from "../lib/api";
import { formatCompact } from "../lib/format";
import { useRecentEventTypes, useWorkspaceIndex } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";

export function ResearchPage() {
  const { data } = useWorkspaceIndex();
  const campaigns = data?.campaigns ?? [];
  const [campaignId, setCampaignId] = useState("");
  const [claimRows, setClaimRows] = useState<Array<Record<string, string>>>([]);
  const [taskRows, setTaskRows] = useState<Array<Record<string, string>>>([]);
  const [personalityRows, setPersonalityRows] = useState<Array<Record<string, string>>>([]);
  const [skillRows, setSkillRows] = useState<Array<Record<string, string>>>([]);
  const [hypothesisRows, setHypothesisRows] = useState<Array<Record<string, unknown>>>([]);
  const [selectedReference, setSelectedReference] = useState<string | null>(null);
  const mechanismEvents = useRecentEventTypes(["run.personality.appended", "run.skill.appended", "run.token.appended"], 16);
  const references: ArtifactDescriptor[] = [
    {
      name: "model-personality-study.md",
      path: "/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/model-personality-study.md",
      kind: "human_readable_dossier",
      exists: true,
      role: "methods",
      scope: "repo",
      format: "markdown",
      previewable: true,
    },
    {
      name: "codex-observability-contract.md",
      path: "/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/codex-observability-contract.md",
      kind: "human_readable_dossier",
      exists: true,
      role: "methods",
      scope: "repo",
      format: "markdown",
      previewable: true,
    },
    {
      name: "probe-taxonomy.md",
      path: "/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md",
      kind: "human_readable_dossier",
      exists: true,
      role: "methods",
      scope: "repo",
      format: "markdown",
      previewable: true,
    },
    {
      name: "model-behavior-v1.json",
      path: "/Users/kevinlin/Downloads/CodexPlusClaw/studies/hypotheses/model-behavior-v1.json",
      kind: "hypothesis_catalog",
      exists: true,
      role: "hypotheses",
      scope: "repo",
      format: "json",
      previewable: true,
    },
  ];
  const activeReference = references.find((artifact) => artifact.path === selectedReference) ?? references[0];
  const evidenceStatus = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const row of claimRows) {
      const label = row.evidence_label ?? row.status ?? "unknown";
      counts[label] = (counts[label] ?? 0) + 1;
    }
    return counts;
  }, [claimRows]);
  const taskLens = useMemo(() => taskRows.slice(0, 8), [taskRows]);
  const hypothesisFocus = useMemo(() => (hypothesisRows as Array<Record<string, unknown>>).slice(0, 4), [hypothesisRows]);
  const mechanismPulse = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const event of mechanismEvents) {
      counts[event.type] = (counts[event.type] ?? 0) + 1;
    }
    return counts;
  }, [mechanismEvents]);
  const evidenceGapRows = useMemo(
    () =>
      [
        ["claim rows", claimRows.length],
        ["task rows", taskRows.length],
        ["personality rows", personalityRows.length],
        ["skill rows", skillRows.length],
        ["mechanism events", mechanismEvents.length],
      ].map(([label, count]) => ({
        label,
        count,
        status: Number(count) > 0 ? "present" : "missing",
      })),
    [claimRows.length, taskRows.length, personalityRows.length, skillRows.length, mechanismEvents.length],
  );

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
      const skillFile = artifacts.find((artifact) => artifact.name === "skill_mechanism.csv");
      const [claims, tasks, personality, skill] = await Promise.all([
        claimFile ? api.artifactFile(claimFile.path, "csv") : Promise.resolve(null),
        taskFile ? api.artifactFile(taskFile.path, "csv") : Promise.resolve(null),
        personalityFile ? api.artifactFile(personalityFile.path, "csv") : Promise.resolve(null),
        skillFile ? api.artifactFile(skillFile.path, "csv") : Promise.resolve(null),
      ]);
      setClaimRows(claims?.payload.kind === "csv" ? claims.payload.rows : []);
      setTaskRows(tasks?.payload.kind === "csv" ? tasks.payload.rows : []);
      setPersonalityRows(personality?.payload.kind === "csv" ? personality.payload.rows : []);
      setSkillRows(skill?.payload.kind === "csv" ? skill.payload.rows : []);
    })();
  }, [activeCampaign?.campaign_id]);

  useEffect(() => {
    void (async () => {
      const response = await api.artifactFile("/Users/kevinlin/Downloads/CodexPlusClaw/studies/hypotheses/model-behavior-v1.json");
      if (response.payload.kind !== "text") {
        setHypothesisRows([]);
        return;
      }
      try {
        const decoded = JSON.parse(response.payload.content) as { hypotheses?: Array<Record<string, unknown>> };
        setHypothesisRows(decoded.hypotheses ?? []);
      } catch {
        setHypothesisRows([]);
      }
    })();
  }, []);

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Research Workbench"
        title="Research"
        description="围绕 hypothesis、claim evidence、task class、personality / instruction / skill 机制来解释实验。这里优先服务写 memo、写 paper outline、做机制对比。"
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Hypothesis Surface" value="H1–H6" detail="model × personality × harness mechanism" tone="signal" />
        <MetricCard label="Claim Rows" value={claimRows.length} detail="grounding + codex-unique evidence" tone="pressure" />
        <MetricCard label="Task-Class Rows" value={taskRows.length} detail="bootstrap / verification / patch / compaction" tone="verify" />
        <MetricCard label="Mechanism Rows" value={formatCompact(personalityRows.length + skillRows.length)} detail="personality + skill mechanism tables" />
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Research Surface Health" kicker="Do we have enough evidence to make disciplined claims?">
          <KeyValueGrid
            columns={4}
            items={[
              { label: "Campaign", value: activeCampaign?.campaign_id ?? "—", detail: activeCampaign?.experiment_name ?? "no active campaign" },
              { label: "Claim Surface", value: claimRows.length, detail: claimRows.length ? "claim_evidence loaded" : "claim evidence missing", tone: claimRows.length ? "verify" : "anomaly" },
              { label: "Task Surface", value: taskRows.length, detail: taskRows.length ? "task lens available" : "task lens missing", tone: taskRows.length ? "signal" : "pressure" },
              { label: "Mechanism Surface", value: personalityRows.length + skillRows.length, detail: mechanismEvents.length ? `${mechanismEvents.length} live mechanism events` : "no live mechanism event", tone: mechanismEvents.length ? "pressure" : "neutral" },
              { label: "Reports / Datasets", value: `${activeCampaign?.report_count ?? 0} / ${activeCampaign?.dataset_count ?? 0}`, detail: activeCampaign?.status ?? "—" },
              { label: "Visible / Total", value: `${formatCompact(activeCampaign?.total_visible_output_tokens_est)} / ${formatCompact(activeCampaign?.total_tokens)}` },
              { label: "Evidence Status Mix", value: Object.entries(evidenceStatus).map(([label, count]) => `${label}×${count}`).join(" · ") || "—" },
              { label: "Method Refs", value: references.length, detail: activeReference.name },
            ]}
          />
        </Panel>

        <Panel title="Evidence Gap Ledger" kicker="What is still missing or weak in the current research surface">
          <div className="focus-grid">
            {evidenceGapRows.map((row) => (
              <div key={row.label} className="focus-note">
                <span className="metric-label">{row.label}</span>
                <strong>{row.count}</strong>
                <span className="mono-note">{row.status}</span>
              </div>
            ))}
          </div>
          <div className="panel-divider" />
          <ul className="evidence-list">
            <li>如果 claim rows 为空，当前页更像 methods hub，而不是 evidence board。</li>
            <li>如果 mechanism rows 很低，要谨慎解释 personality / skill 结论，优先回到 observed artifacts。</li>
            <li>如果 task rows 很低，不要把当前结论过度泛化成跨 task-class 结论。</li>
          </ul>
        </Panel>
      </div>

      <Panel title="Research Navigation" kicker="Jump directly into the current evidence loop">
        {!activeCampaign ? (
          <StateNotice
            title="研究导航尚未绑定到可用 campaign"
            body="当前 workspace 里还没有能驱动研究页的 campaign / datasets；这页会在 run 自动产出 datasets 后显著充实。"
            tone="loading"
          />
        ) : null}
        <div className="chip-row">
          {activeCampaign ? (
            <>
              <Link className="artifact-chip" to="/campaigns">
                active campaign desk
              </Link>
              <Link className="artifact-chip" to="/compare">
                compare workbench
              </Link>
              <Link className="artifact-chip" to="/artifacts">
                evidence archive
              </Link>
            </>
          ) : null}
        </div>
        {activeCampaign ? (
          <KeyValueGrid
            columns={4}
            items={[
              { label: "Campaign", value: activeCampaign.campaign_id, detail: activeCampaign.experiment_name },
              { label: "Benchmark", value: activeCampaign.benchmark_name, detail: activeCampaign.stage_name ?? "—" },
              { label: "Reports / Datasets", value: `${activeCampaign.report_count} / ${activeCampaign.dataset_count}`, detail: activeCampaign.status || "—" },
              { label: "Visible / Total", value: `${formatCompact(activeCampaign.total_visible_output_tokens_est)} / ${formatCompact(activeCampaign.total_tokens)}` },
            ]}
          />
        ) : null}
      </Panel>

      <Panel title="Evidence Status Board" kicker="Current hypothesis / claim readout">
        {!Object.keys(evidenceStatus).length ? (
          <StateNotice
            title="claim evidence 还没有载入"
            body="Research 页需要 `claim_evidence.csv` 和相关 datasets；如果 benchmark 还没跑完，这里会先显示方法层框架。"
            tone="info"
          />
        ) : null}
        <KeyValueGrid
          columns={5}
          items={Object.entries(evidenceStatus).slice(0, 10).map(([label, count]) => ({
            label,
            value: count,
            detail: "claim rows",
            tone: label.includes("consistent") ? "signal" : label.includes("mixed") ? "pressure" : label.includes("against") ? "anomaly" : "neutral",
          }))}
        />
      </Panel>

      <div className="page-grid page-grid-2">
        <Panel title="Hypothesis Command Deck" kicker="What to read first before writing claims">
          <div className="focus-grid">
            <div className="focus-note">
              <span className="metric-label">Most important question</span>
              <strong>谁更会把任务状态外显出来</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Mechanism axis</span>
              <strong>personality × instruction × tool mediation</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Evidence discipline</span>
              <strong>observed first, inferred second</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Current campaign</span>
              <strong>{activeCampaign?.campaign_id ?? "—"}</strong>
            </div>
          </div>
          <div className="panel-divider" />
          <ul className="evidence-list">
            <li>先看 `claim_evidence.csv` 与 hypothesis ledger，再回到 Compare 页找同题四格的具体证据。</li>
            <li>如果 mechanism rows 很薄，不要急着写“personality 改变了 policy”，先写 evidence-limited。</li>
            <li>优先把 tool / verification / bridge language 的联合变化当作主证据，而不是单看字数。</li>
          </ul>
        </Panel>

        <Panel title="Hypothesis Spotlight" kicker="What this bench is actively trying to prove or falsify">
          {!hypothesisFocus.length ? (
            <StateNotice
              title="hypothesis catalog 暂时不可用"
              body="如果研究假设文件还没加载成功，这里会先保持空态，不影响其他机制与 evidence 面板。"
              tone="warning"
            />
          ) : null}
          <div className="focus-grid">
            {hypothesisFocus.map((row, index) => (
              <div key={`${row.id ?? index}`} className="insight-card">
                <div className="section-label">{String(row.id ?? `H${index + 1}`)}</div>
                <strong>{String(row.text ?? "Untitled hypothesis")}</strong>
                <p>{String((row.preferred_task_classes as string[] | undefined)?.join(" · ") ?? "No preferred task class listed")}</p>
              </div>
            ))}
          </div>
        </Panel>
        <Panel title="Mechanism Focus" kicker="Current reading frame for model vs harness">
          <ul className="evidence-list">
            <li>先读 observed，再读 inferred，最后才允许读 estimated。</li>
            <li>优先看 personality 是否改变 bridge / verification / state externalization。</li>
            <li>再看 tool route 与 command density 是否一起上升，而不是只看字数。</li>
            <li>最后才讨论 style；先判断 policy-shaping，再判断 tone-shaping。</li>
          </ul>
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Current Research Focus" kicker="Active thesis line">
          {!taskLens.length ? (
            <StateNotice
              title="task-class lens 还没有 campaign 数据"
              body="等 task-class summary 生成后，这里会逐类展示 bootstrap-heavy / verification-heavy / patch-heavy 等任务的行为差异。"
              tone="loading"
            />
          ) : null}
          <ul className="evidence-list">
            <li>5.4 是否比 5.3-codex 更愿意把状态外显成用户可见语言。</li>
            <li>friendly 与 pragmatic 的差异是语气级，还是会改变工具编排与验证节奏。</li>
            <li>语言输出、tool use、patch chain 与 personality 机制如何一起形成 agent policy。</li>
            <li>哪些差异更像模型因素，哪些差异更像 harness 中介效应。</li>
          </ul>
        </Panel>
        <Panel title="Methods Lens" kicker="Observability / evidence discipline">
          <KeyValueGrid
            columns={2}
            items={[
              { label: "Observed", value: "Codex raw events / process / artifacts", tone: "signal" },
              { label: "Inferred", value: "tool coupling / style / mechanism summaries", tone: "pressure" },
              { label: "Estimated", value: "visible token estimates / lexical density", tone: "verify" },
              { label: "Scope", value: activeCampaign?.benchmark_name ?? "—" },
            ]}
          />
          <div className="panel-divider" />
          <div className="focus-grid">
            {Object.entries(mechanismPulse).slice(0, 6).map(([label, count]) => (
              <div key={label} className="focus-note">
                <span className="metric-label">{label}</span>
                <strong>{count}</strong>
              </div>
            ))}
          </div>
          <div className="panel-divider" />
          <div className="artifact-list artifact-list-column artifact-ledger">
            {references.map((artifact) => (
              <button
                key={artifact.path}
                className={`artifact-chip${activeReference.path === artifact.path ? " artifact-chip-active" : ""}`}
                onClick={() => setSelectedReference(artifact.path)}
              >
                <span>{artifact.name}</span>
                <span className="artifact-kind">{artifact.role}</span>
              </button>
            ))}
          </div>
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Mechanism Pulse Board" kicker="Live mechanism categories currently surfacing">
          {!Object.keys(mechanismPulse).length ? (
            <StateNotice
              title="live mechanism pulse 还没有形成"
              body="如果没有 personality / skill / token / mechanism 增量事件，这里会先保持空态，不代表研究面失效。"
              tone="loading"
            />
          ) : (
            <div className="focus-grid">
              {Object.entries(mechanismPulse).map(([label, count]) => (
                <div key={label} className="focus-note">
                  <span className="metric-label">{label}</span>
                  <strong>{count}</strong>
                </div>
              ))}
            </div>
          )}
        </Panel>
        <Panel title="Reference Dock" kicker="Methods / observability / probe discipline">
          <div className="artifact-list artifact-list-column artifact-ledger">
            {references.map((artifact) => (
              <button
                key={artifact.path}
                className={`artifact-row-button${activeReference.path === artifact.path ? " artifact-row-button-active" : ""}`}
                onClick={() => setSelectedReference(artifact.path)}
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
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Personality / Mechanism Reading Guide" kicker="How to interpret tone vs policy-shaping">
          <ul className="evidence-list">
            <li>先看 `requested vs effective personality`，再看 fallback，不要跳过机制生效层。</li>
            <li>如果 social tone 上升，但 bridge / verification / externalization 不变，更像 tone-shaping。</li>
            <li>如果工具密度、bridge language、verification framing 一起变化，更像 policy-shaping。</li>
            <li>如果 skill / instruction rows 稀薄，优先把结论写成 evidence-limited，而不是 mechanism-proven。</li>
          </ul>
        </Panel>
        <Panel title="Observability Contract Highlights" kicker="What this console can and cannot really know">
          <div className="focus-grid">
            <div className="focus-note">
              <span className="metric-label">Observed</span>
              <strong>Raw events / probes / processes</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Inferred</span>
              <strong>bridge / coupling / skill incidence</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Estimated</span>
              <strong>visible token estimates / lexical density</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Not observable</span>
              <strong>full hidden CoT / uninstrumented internal branches</strong>
            </div>
          </div>
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Evidence Status" kicker="Current hypothesis support surface">
          <KeyValueGrid
            columns={3}
            items={[
              { label: "Evidence Consistent", value: evidenceStatus.evidence_consistent ?? 0, tone: "verify" },
              { label: "Evidence Mixed", value: evidenceStatus.evidence_mixed ?? 0, tone: "pressure" },
              { label: "Inconclusive", value: evidenceStatus.evidence_inconclusive ?? 0, tone: "anomaly" },
              { label: "Against", value: evidenceStatus.evidence_against ?? 0, tone: "anomaly" },
              { label: "Not Observable", value: evidenceStatus.not_observable_with_current_probes ?? 0 },
              { label: "Campaign", value: activeCampaign?.campaign_id ?? "—" },
            ]}
          />
        </Panel>
        <Panel title="Task-class Lens" kicker="Fast read on where behavior changes show up">
          <DataTable rows={taskLens as Array<Record<string, unknown>>} compact />
        </Panel>
      </div>

      <div className="page-grid research-grid">
        <Panel title="Hypothesis Ledger" kicker="studies/hypotheses/model-behavior-v1.json">
          <DataTable rows={hypothesisRows as Array<Record<string, unknown>>} compact />
        </Panel>
        <Panel title="Hypothesis / Claim Evidence" kicker="claim_evidence.csv">
          <DataTable rows={claimRows as Array<Record<string, unknown>>} />
        </Panel>
        <Panel title="Task-class Lens" kicker="task_class_summary.csv">
          <DataTable rows={taskRows as Array<Record<string, unknown>>} />
        </Panel>
        <Panel title="Personality Mechanism Lens" kicker="personality_mechanism.csv">
          <DataTable rows={personalityRows as Array<Record<string, unknown>>} compact />
        </Panel>
        <Panel title="Skill / Mechanism Lens" kicker="skill_mechanism.csv + live mechanism stream">
          <DataTable rows={skillRows as Array<Record<string, unknown>>} compact />
          <div className="panel-divider" />
          <EventRail rows={mechanismEvents} emptyLabel="等待更多机制事件。" />
        </Panel>
        <Panel title="Observability Lens" kicker="Observed / inferred / estimated boundaries">
          <ul className="evidence-list">
            <li>Observed: raw agent events, diagnostics, Codex study probes, process registry and artifact append events.</li>
            <li>Inferred: language coupling, skill incidence, mechanism summaries, bridge language, state externalization.</li>
            <li>Estimated: visible token totals, lexical densities, some timing-derived efficiency rates.</li>
            <li>Research writing should cite the artifact path and classification layer together.</li>
          </ul>
        </Panel>
      </div>

      <Panel title="Methods Appendix Dock" kicker={activeReference.name}>
        <ArtifactViewer artifact={activeReference} />
      </Panel>
    </div>
  );
}
