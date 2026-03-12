import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";

import { DataTable } from "../components/DataTable";
import { KeyValueGrid } from "../components/KeyValueGrid";
import { MetricCard } from "../components/MetricCard";
import { PageIntro } from "../components/PageIntro";
import { Panel } from "../components/Panel";
import { RunCard } from "../components/RunCard";
import { SignalBar } from "../components/SignalBar";
import { StateNotice } from "../components/StateNotice";
import { api } from "../lib/api";
import { formatCompact, humanizeKey } from "../lib/format";
import { useWorkspaceIndex } from "../lib/store";
import type { ArtifactDescriptor } from "../lib/types";

type CsvRow = Record<string, string>;

export function ComparePage() {
  const { data } = useWorkspaceIndex();
  const campaigns = data?.campaigns ?? [];
  const [campaignId, setCampaignId] = useState("");
  const [datasets, setDatasets] = useState<ArtifactDescriptor[]>([]);
  const [pairRows, setPairRows] = useState<CsvRow[]>([]);
  const [campaignRunRows, setCampaignRunRows] = useState<CsvRow[]>([]);
  const [phraseRows, setPhraseRows] = useState<CsvRow[]>([]);
  const [toolRows, setToolRows] = useState<CsvRow[]>([]);
  const [personalityRows, setPersonalityRows] = useState<CsvRow[]>([]);
  const [selectedInstance, setSelectedInstance] = useState("");

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
      const phraseArtifact = nextDatasets.find((artifact) => artifact.name === "model_phrase_deltas.csv");
      const toolArtifact = nextDatasets.find((artifact) => artifact.name === "tool_inventory.csv");
      const personalityArtifact = nextDatasets.find((artifact) => artifact.name === "personality_mechanism.csv");
      const [pairs, runs, phrases, tools, personality] = await Promise.all([
        pairArtifact ? api.artifactFile(pairArtifact.path, "csv") : Promise.resolve(null),
        campaignRunsArtifact ? api.artifactFile(campaignRunsArtifact.path, "csv") : Promise.resolve(null),
        phraseArtifact ? api.artifactFile(phraseArtifact.path, "csv") : Promise.resolve(null),
        toolArtifact ? api.artifactFile(toolArtifact.path, "csv") : Promise.resolve(null),
        personalityArtifact ? api.artifactFile(personalityArtifact.path, "csv") : Promise.resolve(null),
      ]);
      setPairRows(pairs?.payload.kind === "csv" ? pairs.payload.rows : []);
      setCampaignRunRows(runs?.payload.kind === "csv" ? runs.payload.rows : []);
      setPhraseRows(phrases?.payload.kind === "csv" ? phrases.payload.rows : []);
      setToolRows(tools?.payload.kind === "csv" ? tools.payload.rows : []);
      setPersonalityRows(personality?.payload.kind === "csv" ? personality.payload.rows : []);
    })();
  }, [activeCampaign?.campaign_id]);

  const runIndex = data?.runs ?? [];
  const pairHighlights = pairRows.slice(0, 12);
  const deltaLeaderboard = useMemo(() => {
    const numeric = (row: CsvRow, ...keys: string[]) => {
      for (const key of keys) {
        const value = Number(row[key] ?? "");
        if (Number.isFinite(value) && value !== 0) return value;
      }
      return 0;
    };
    const byMagnitude = (keys: string[]) =>
      [...pairRows]
        .map((row) => ({ row, value: Math.abs(numeric(row, ...keys)) }))
        .sort((left, right) => right.value - left.value)
        .slice(0, 4);
    return {
      visible: byMagnitude(["visible_output_total_tokens_est_delta", "visible_delta"]),
      tools: byMagnitude(["tool_count_delta"]),
      bridge: byMagnitude(["bridge_language_score_bps_delta", "bridge_delta"]),
      verify: byMagnitude(["verification_language_score_bps_delta", "verify_delta"]),
    };
  }, [pairRows]);
  const sameTaskGroups = useMemo(() => {
    const byInstance = campaignRunRows.reduce<Record<string, CsvRow[]>>((acc, row) => {
      const key = row.instance_id ?? "unknown";
      acc[key] ??= [];
      acc[key].push(row);
      return acc;
    }, {});
    return Object.entries(byInstance).slice(0, 8);
  }, [campaignRunRows]);
  const activeTaskRows = useMemo(
    () => sameTaskGroups.find(([instanceId]) => instanceId === selectedInstance)?.[1] ?? sameTaskGroups[0]?.[1] ?? [],
    [sameTaskGroups, selectedInstance],
  );
  const activeTaskId = useMemo(
    () => sameTaskGroups.find(([instanceId]) => instanceId === selectedInstance)?.[0] ?? sameTaskGroups[0]?.[0] ?? null,
    [sameTaskGroups, selectedInstance],
  );
  const activeQuadrants = useMemo(() => {
    const order = [
      "gpt-5.3-codex-pragmatic",
      "gpt-5.3-codex-friendly",
      "gpt-5.4-pragmatic",
      "gpt-5.4-friendly",
    ];
    return order.map((cohortId) => ({
      cohortId,
      row: activeTaskRows.find((row) => row.cohort_id === cohortId) ?? null,
      run: activeTaskRows
        .map((row) => runIndex.find((run) => run.run_id === row.run_id))
        .find((run) => run?.cohort_id === cohortId) ?? null,
    }));
  }, [activeTaskRows, runIndex]);

  const cohortCounts = useMemo(() => {
    const acc: Record<string, number> = {};
    for (const row of campaignRunRows) {
      const key = row.cohort_id ?? "unknown";
      acc[key] = (acc[key] ?? 0) + 1;
    }
    return acc;
  }, [campaignRunRows]);

  const toolHighlights = useMemo(() => toolRows.slice(0, 10), [toolRows]);
  const phraseHighlights = useMemo(() => phraseRows.slice(0, 10), [phraseRows]);
  const personalityHighlights = useMemo(() => personalityRows.slice(0, 10), [personalityRows]);
  const selectedTaskSummary = useMemo(() => {
    if (!activeTaskRows.length) return null;
    const visible = activeTaskRows.map((row) => Number(row.visible_output_total_tokens_est ?? 0));
    const tools = activeTaskRows.map((row) => Number(row.tool_count ?? 0));
    const bridge = activeTaskRows.map((row) => Number(row.bridge_language_score_bps ?? 0));
    const verification = activeTaskRows.map((row) => Number(row.verification_language_score_bps ?? 0));
    const maxVisibleIndex = visible.indexOf(Math.max(...visible));
    const maxToolIndex = tools.indexOf(Math.max(...tools));
    const maxBridgeIndex = bridge.indexOf(Math.max(...bridge));
    const maxVerifyIndex = verification.indexOf(Math.max(...verification));
    return {
      maxVisible: activeTaskRows[maxVisibleIndex]?.cohort_id ?? "—",
      maxTool: activeTaskRows[maxToolIndex]?.cohort_id ?? "—",
      maxBridge: activeTaskRows[maxBridgeIndex]?.cohort_id ?? "—",
      maxVerify: activeTaskRows[maxVerifyIndex]?.cohort_id ?? "—",
    };
  }, [activeTaskRows]);
  const selectedTaskMechanismDelta = useMemo(
    () =>
      activeTaskRows
        .map((row) => ({
          cohort: row.cohort_id ?? "unknown",
          bridge: Number(row.bridge_language_score_bps ?? 0),
          verify: Number(row.verification_language_score_bps ?? 0),
          externalization: Number(row.state_externalization_score_bps ?? 0),
          social: Number(row.social_tone_score_bps ?? 0),
        }))
        .sort((left, right) => right.externalization - left.externalization),
    [activeTaskRows],
  );
  const selectedTaskMaxima = useMemo(() => {
    if (!activeTaskRows.length) {
      return {
        visible: 1,
        tools: 1,
        bridge: 1,
        verify: 1,
      };
    }
    return {
      visible: Math.max(...activeTaskRows.map((row) => Number(row.visible_output_total_tokens_est ?? 0)), 1),
      tools: Math.max(...activeTaskRows.map((row) => Number(row.tool_count ?? 0)), 1),
      bridge: Math.max(...activeTaskRows.map((row) => Number(row.bridge_language_score_bps ?? 0)), 1),
      verify: Math.max(...activeTaskRows.map((row) => Number(row.verification_language_score_bps ?? 0)), 1),
    };
  }, [activeTaskRows]);
  const phraseDeltaHighlights = useMemo(() => {
    const grouped = phraseRows.reduce<Record<string, CsvRow[]>>((acc, row) => {
      const key = `${row.left_cohort ?? "left"}→${row.right_cohort ?? "right"}`;
      acc[key] ??= [];
      acc[key].push(row);
      return acc;
    }, {});
    return Object.entries(grouped)
      .map(([key, rows]) => ({ key, rows: rows.slice(0, 6) }))
      .slice(0, 4);
  }, [phraseRows]);

  return (
    <div className="page-grid">
      <PageIntro
        kicker="Pairwise Research Workbench"
        title="Compare"
        description="这里优先回答 2x2 研究问题：谁说得更多、桥接语言谁更强、工具路由和 personality 机制在哪些同题配对上分化。"
      />

      <div className="page-grid page-grid-4">
        <MetricCard label="Pair Rows" value={pairRows.length} detail={`${campaignRunRows.length} run rows`} tone="signal" />
        <MetricCard label="Phrase Deltas" value={phraseRows.length} detail="model / personality lexical shifts" tone="pressure" />
        <MetricCard label="Tool Inventory" value={toolRows.length} detail="concrete tool rows" tone="verify" />
        <MetricCard label="Cohort Surface" value={Object.keys(cohortCounts).length} detail={`${activeCampaign?.sample_size ?? 0} sampled tasks`} />
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Comparison Readiness" kicker="Is this campaign ready for serious 2x2 analysis?">
          {!activeCampaign ? (
            <StateNotice
              title="当前还没有可分析的 experiment"
              body="Compare 页需要一个已经写出 datasets 的 campaign。没有 datasets 时，这里会优先解释缺了哪一层。"
              tone="loading"
            />
          ) : null}
          <KeyValueGrid
            columns={4}
            items={[
              { label: "Campaign Status", value: activeCampaign?.status ?? "—", detail: activeCampaign?.experiment_name ?? "—" },
              { label: "Datasets", value: datasets.length, detail: datasets[0]?.name ?? "none yet", tone: datasets.length ? "verify" : "anomaly" },
              { label: "Pair Rows", value: pairRows.length, detail: pairRows.length ? "pair deltas present" : "pair deltas not written yet", tone: pairRows.length ? "signal" : "pressure" },
              { label: "Run Rows", value: campaignRunRows.length, detail: sameTaskGroups.length ? `${sameTaskGroups.length} same-task groups` : "no same-task groups yet" },
              { label: "Phrase Rows", value: phraseRows.length, detail: phraseRows[0]?.phrase ?? phraseRows[0]?.lemma ?? "—" },
              { label: "Tool Rows", value: toolRows.length, detail: toolRows[0]?.tool_name ?? "—" },
              { label: "Mechanism Rows", value: personalityRows.length, detail: personalityRows[0]?.personality_effective ?? "—" },
              { label: "Selected Task", value: activeTaskId ?? "—", detail: selectedTaskSummary ? `${selectedTaskSummary.maxVisible} most verbose` : "pick a same-task row" },
            ]}
          />
        </Panel>

        <Panel title="Delta Leaderboards" kicker="Largest divergences worth reading first">
          {!pairRows.length ? (
            <StateNotice
              title="delta leaderboard 还没形成"
              body="一旦 `model_pair_deltas.csv` 和 `campaign_runs.csv` 写出，这里会自动挑出最值得读的同题差异。"
              tone="loading"
            />
          ) : null}
          <div className="page-grid page-grid-2">
            {([
              ["Visible", deltaLeaderboard.visible],
              ["Tools", deltaLeaderboard.tools],
              ["Bridge", deltaLeaderboard.bridge],
              ["Verify", deltaLeaderboard.verify],
            ] as Array<[string, Array<{ row: CsvRow; value: number }>]>).map(([label, rows]) => (
              <div key={label} className="compare-block">
                <div className="compare-heading">{label} delta</div>
                <div className="cluster-list">
                  {rows.map(({ row, value }, index) => (
                    <div key={`${label}-${index}`} className="event-kv">
                      <span>{row.instance_id ?? "instance"} · {row.left_cohort ?? "left"} → {row.right_cohort ?? "right"}</span>
                      <strong>{value || "—"}</strong>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </Panel>
      </div>

      <Panel title="Comparison Target" kicker="Select experiment campaign">
        {!activeCampaign ? (
          <StateNotice
            title="还没有可比较的实验 campaign"
            body="Compare 页依赖 campaign 级 datasets。等一次完整的 run 自动生成 datasets 后，这里会变成 2x2 研究工作台。"
            tone="loading"
          />
        ) : null}
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
          <KeyValueGrid
            columns={5}
            items={[
              { label: "Benchmark", value: activeCampaign.benchmark_name, detail: activeCampaign.stage_name ?? "—" },
              { label: "Sample", value: activeCampaign.sample_size },
              { label: "Cohorts", value: activeCampaign.cohort_count },
              { label: "Visible Output", value: formatCompact(activeCampaign.total_visible_output_tokens_est), tone: "signal" },
              { label: "Tool Calls", value: formatCompact(activeCampaign.total_tool_calls), tone: "pressure" },
              { label: "Reports", value: activeCampaign.report_count, detail: activeCampaign.report_paths[0] ?? "—" },
              { label: "Datasets", value: activeCampaign.dataset_count, detail: activeCampaign.dataset_paths[0] ?? "—" },
              { label: "Active", value: activeCampaign.active_run_count, detail: `${activeCampaign.completed_run_count} completed` },
              { label: "Tokens", value: formatCompact(activeCampaign.total_tokens), detail: `${activeCampaign.selected_instances} instances` },
              { label: "Status", value: activeCampaign.status },
            ]}
          />
        ) : null}
      </Panel>

      <div className="page-grid page-grid-2">
        <Panel title="2x2 Quadrant Board" kicker="Cohort counts inside the selected campaign">
          {!Object.keys(cohortCounts).length ? (
            <StateNotice
              title="cohort 象限暂时为空"
              body="当前 campaign 还没有足够的 run row 写进 datasets，或者这批 run 还在进行中。"
              tone="info"
            />
          ) : null}
          <div className="quadrant-board">
            {Object.entries(cohortCounts).map(([cohort, count]) => (
              <div key={cohort} className="quadrant-cell">
                <div className="quadrant-kicker">{cohort}</div>
                <strong>{count}</strong>
                <span>{humanizeKey(cohort)}</span>
              </div>
            ))}
          </div>
        </Panel>

        <Panel title="Pair Delta Highlights" kicker="model_pair_deltas.csv">
          {!pairHighlights.length ? (
            <StateNotice
              title="pair delta 还没有形成"
              body="等 `model_pair_deltas.csv` 生成后，这里会直接列出 5.4 vs 5.3-codex、friendly vs pragmatic 的关键差分。"
              tone="loading"
            />
          ) : null}
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
                  <div><span className="metric-label">Visible Δ</span><strong>{row.visible_output_total_tokens_est_delta ?? "—"}</strong></div>
                  <div><span className="metric-label">Tool Δ</span><strong>{row.tool_count_delta ?? "—"}</strong></div>
                  <div><span className="metric-label">Cmd Δ</span><strong>{row.command_count_delta ?? "—"}</strong></div>
                  <div><span className="metric-label">Narration Δ</span><strong>{row.micro_narrated_tool_burst_count_delta ?? "—"}</strong></div>
                </div>
              </div>
            ))}
          </div>
        </Panel>
      </div>

      <Panel title="Selected Task Reading Frame" kicker="What this one same-task 2x2 actually suggests">
        {!activeTaskRows.length ? (
          <StateNotice
            title="请先选择一个有 cohort coverage 的 task"
            body="当 `campaign_runs.csv` 里出现同题多 cohort 行后，这里会自动提炼出这道题最值得看的解释角度。"
            tone="info"
          />
        ) : null}
        {activeTaskRows.length ? (
          <div className="page-grid page-grid-2">
            <div className="focus-grid">
              <div className="focus-note">
                <span className="metric-label">Most verbose</span>
                <strong>{selectedTaskSummary?.maxVisible ?? "—"}</strong>
              </div>
              <div className="focus-note">
                <span className="metric-label">Most tool-dense</span>
                <strong>{selectedTaskSummary?.maxTool ?? "—"}</strong>
              </div>
              <div className="focus-note">
                <span className="metric-label">Strongest bridge</span>
                <strong>{selectedTaskSummary?.maxBridge ?? "—"}</strong>
              </div>
              <div className="focus-note">
                <span className="metric-label">Strongest verify</span>
                <strong>{selectedTaskSummary?.maxVerify ?? "—"}</strong>
              </div>
            </div>
            <ul className="evidence-list">
              <li>先看四格里谁的 `visible output` 上升，同时 `tool count` 也上升，而不是只变长。</li>
              <li>再看 `bridge / verification framing`，判断是单纯更会说，还是更会把动作组织成外显策略。</li>
              <li>最后看 `social tone` 与 mechanism rows，判断 `friendly` 差异更像 tone-shaping 还是 policy-shaping。</li>
            </ul>
          </div>
        ) : null}
      </Panel>

      <Panel title="2x2 Same-task Board" kicker="Select one task and inspect all four quadrants">
        {!sameTaskGroups.length ? (
          <StateNotice
            title="还没有同题四格样本"
            body="这个区域只在同一个 instance 被多个 cohort 配对跑过后才会亮起来。"
            tone="loading"
          />
        ) : null}
        <div className="filter-row filter-row-wide">
          <select value={activeTaskId ?? ""} onChange={(event) => setSelectedInstance(event.target.value)}>
            {sameTaskGroups.map(([instanceId, rows]) => (
              <option key={instanceId} value={instanceId}>
                {instanceId} / {rows.length} cohorts
              </option>
            ))}
          </select>
        </div>
        <div className="quadrant-board quadrant-board-2x2">
          {activeQuadrants.map(({ cohortId, row, run }) => (
            <div key={cohortId} className="quadrant-cell quadrant-cell-detailed">
              <div className="quadrant-kicker">{cohortId}</div>
              <strong>{row?.visible_output_total_tokens_est ?? "—"}</strong>
              <span>visible tokens</span>
              <div className="quadrant-meta">
                <span>tools {row?.tool_count ?? "—"}</span>
                <span>commands {row?.command_count ?? "—"}</span>
              </div>
              <div className="quadrant-meta">
                <span>bridge {row?.bridge_language_score_bps ?? "—"}</span>
                <span>verify {row?.verification_language_score_bps ?? "—"}</span>
              </div>
              {row ? (
                <div className="signal-bar-stack">
                  <SignalBar
                    label="Visible"
                    value={Number(row.visible_output_total_tokens_est ?? 0)}
                    max={selectedTaskMaxima.visible}
                    tone="signal"
                  />
                  <SignalBar
                    label="Tools"
                    value={Number(row.tool_count ?? 0)}
                    max={selectedTaskMaxima.tools}
                    tone="pressure"
                  />
                  <SignalBar
                    label="Bridge"
                    value={Number(row.bridge_language_score_bps ?? 0)}
                    max={selectedTaskMaxima.bridge}
                    tone="verify"
                    detail="bps"
                  />
                  <SignalBar
                    label="Verify"
                    value={Number(row.verification_language_score_bps ?? 0)}
                    max={selectedTaskMaxima.verify}
                    tone="authority"
                    detail="bps"
                  />
                </div>
              ) : null}
              {run ? <RunCard run={run} compact /> : <div className="empty-box">no run card</div>}
              {run ? (
                <div className="chip-row">
                  <Link className="artifact-chip" to={`/runs/${run.run_id}`}>
                    war room
                  </Link>
                </div>
              ) : null}
            </div>
          ))}
        </div>
        {selectedTaskSummary ? (
          <div className="focus-grid">
            <div className="focus-note">
              <span className="metric-label">Most verbose</span>
              <strong>{selectedTaskSummary.maxVisible}</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Most tool-dense</span>
              <strong>{selectedTaskSummary.maxTool}</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Strongest bridge language</span>
              <strong>{selectedTaskSummary.maxBridge}</strong>
            </div>
            <div className="focus-note">
              <span className="metric-label">Strongest verification framing</span>
              <strong>{selectedTaskSummary.maxVerify}</strong>
            </div>
          </div>
        ) : null}
      </Panel>

      <Panel title="Same-task 2x2 Matrix" kicker="Drill down by instance">
        {!sameTaskGroups.length ? (
          <StateNotice
            title="矩阵暂时为空"
            body="当 campaign 还没生成 `campaign_runs.csv` 或样本不足以形成配对时，这里会保持空态。"
            tone="info"
          />
        ) : null}
        <div className="same-task-groups">
          {sameTaskGroups.map(([instanceId, rows]) => {
            const cards = rows
              .map((row) => runIndex.find((run) => run.run_id === row.run_id))
              .filter((run): run is NonNullable<typeof runIndex[number]> => Boolean(run));
            return (
              <div key={instanceId} className="same-task-group">
                <div className="same-task-head">
                  <strong>{instanceId}</strong>
                  <span>{rows.length} cohort rows</span>
                </div>
                <div className="run-card-grid-board run-card-grid-2">
                  {cards.map((run) => (
                    <RunCard key={run.run_id} run={run} compact />
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </Panel>

      <div className="page-grid page-grid-2">
        <Panel title="2x2 Signal Board" kicker="Selected task normalized against per-task maxima">
          <div className="signal-board-grid">
            {activeQuadrants.map(({ cohortId, row }) => (
              <div key={`${cohortId}-signal`} className="compare-block">
                <div className="compare-heading">{cohortId}</div>
                {row ? (
                  <div className="signal-bar-stack">
                    <SignalBar
                      label="Visible output"
                      value={Number(row.visible_output_total_tokens_est ?? 0)}
                      max={selectedTaskMaxima.visible}
                      tone="signal"
                    />
                    <SignalBar
                      label="Tool count"
                      value={Number(row.tool_count ?? 0)}
                      max={selectedTaskMaxima.tools}
                      tone="pressure"
                    />
                    <SignalBar
                      label="Bridge language"
                      value={Number(row.bridge_language_score_bps ?? 0)}
                      max={selectedTaskMaxima.bridge}
                      tone="verify"
                      detail="bps"
                    />
                    <SignalBar
                      label="Verification framing"
                      value={Number(row.verification_language_score_bps ?? 0)}
                      max={selectedTaskMaxima.verify}
                      tone="authority"
                      detail="bps"
                    />
                  </div>
                ) : (
                  <div className="empty-box">该 quadrant 暂无数据。</div>
                )}
              </div>
            ))}
          </div>
        </Panel>

        <Panel title="Phrase Delta Surface" kicker="model_phrase_deltas.csv">
          <DataTable rows={phraseHighlights as Array<Record<string, unknown>>} compact />
        </Panel>
        <Panel title="Tool Inventory Surface" kicker="tool_inventory.csv">
          <DataTable rows={toolHighlights as Array<Record<string, unknown>>} compact />
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Mechanism Delta Board" kicker="Selected task mechanism surface">
          {selectedTaskMechanismDelta.length ? (
            <div className="signal-board-grid">
              {selectedTaskMechanismDelta.map((row) => (
                <div key={row.cohort} className="compare-block">
                  <div className="compare-heading">{row.cohort}</div>
                  <div className="signal-bar-stack">
                    <SignalBar label="Bridge" value={row.bridge} max={10_000} tone="signal" detail="bps" />
                    <SignalBar label="Verify" value={row.verify} max={10_000} tone="verify" detail="bps" />
                    <SignalBar label="Externalize" value={row.externalization} max={10_000} tone="pressure" detail="bps" />
                    <SignalBar label="Social" value={row.social} max={10_000} tone="authority" detail="bps" />
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="empty-box">等待机制对比数据。</div>
          )}
        </Panel>
        <Panel title="Route / Burst Delta Board" kicker="How tooling style changes by quadrant">
          <div className="compare-stack">
            {activeTaskRows.slice(0, 4).map((row) => (
              <div key={`route-${row.cohort_id}`} className="compare-block">
                <div className="compare-heading">{row.cohort_id ?? "cohort"}</div>
                <div className="metric-grid">
                  <div><span className="metric-label">Tool</span><strong>{row.tool_count ?? "—"}</strong></div>
                  <div><span className="metric-label">Cmd</span><strong>{row.command_count ?? "—"}</strong></div>
                  <div><span className="metric-label">Micro-narr.</span><strong>{row.micro_narrated_tool_burst_count ?? "—"}</strong></div>
                  <div><span className="metric-label">Silent burst</span><strong>{row.silent_tool_burst_count ?? "—"}</strong></div>
                </div>
              </div>
            ))}
          </div>
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Phrase Delta Clusters" kicker="Top lexical shifts by comparison direction">
          <div className="compare-stack">
            {phraseDeltaHighlights.map((cluster) => (
              <div key={cluster.key} className="compare-block">
                <div className="compare-heading">{cluster.key}</div>
                <div className="brief-meta">
                  <span>{cluster.rows.length} phrase rows</span>
                </div>
                <div className="cluster-list">
                  {cluster.rows.map((row, index) => (
                    <div key={`${cluster.key}-${index}`} className="event-kv">
                      <span>{row.phrase ?? row.lemma ?? "term"}</span>
                      <strong>{row.delta ?? row.count_delta ?? "—"}</strong>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </Panel>
        <Panel title="Mechanism Reading Guide" kicker="What to look for in 2x2 policy shifts">
          <ul className="evidence-list">
            <li>同题四格里，哪些 quadrant 提升了 bridge / verification language，而不是纯 social tone。</li>
            <li>哪些 cohort 的工具增长来自 shell burst，哪些来自 patch / MCP / routed tools。</li>
            <li>personality fallback 或 model-native personality preserve 是否和语言风格变化同向。</li>
            <li>如果 `pragmatic` 仍然高 verbosity，要看它是否更偏 task-state narration 而不是礼貌包装。</li>
          </ul>
        </Panel>
      </div>

      <div className="page-grid page-grid-2">
        <Panel title="Mechanism Delta Surface" kicker="personality_mechanism.csv">
          <DataTable rows={personalityHighlights as Array<Record<string, unknown>>} compact />
        </Panel>
        <Panel title="2x2 Reading Guide" kicker="What to ask of this matrix">
          <ul className="evidence-list">
            <li>谁的可见输出更多，但又不是纯礼貌包装？</li>
            <li>谁的 bridge / verification / state externalization 更高？</li>
            <li>谁的 tool burst 更密，谁更喜欢 silent burst？</li>
            <li>friendly 和 pragmatic 的差异更像 tone，还是 policy-shaping？</li>
            <li>哪些 mechanism signals 只在某些 cohort 中显著上升？</li>
          </ul>
        </Panel>
      </div>

      <Panel title="Pairwise Dataset" kicker="Full model_pair_deltas.csv">
        <DataTable rows={pairRows as Array<Record<string, unknown>>} />
      </Panel>
    </div>
  );
}
