import { useEffect, useMemo, useState } from "react";

import { api } from "../lib/api";
import { useLiveOverview, useWorkspaceIndex } from "../lib/store";
import { SegmentedTabs } from "./SegmentedTabs";

type ActionKind =
  | "prepare"
  | "bootstrap-local"
  | "warm-cache"
  | "run"
  | "grade"
  | "report"
  | "replay"
  | "inspect-codex";

const actionHelp: Record<ActionKind, string> = {
  prepare: "创建一个新的 campaign manifest，冻结样本、cohort 与实验参数。",
  "bootstrap-local": "预热本地 bench 与 repo cache，降低后续冷启动成本。",
  "warm-cache": "只执行 repo cache 预热，不启动求解。",
  run: "运行 solver，并在完成后自动生成报告与数据集。",
  grade: "调用官方 benchmark grader，并在完成后自动刷新报告。",
  report: "从现有 artifacts 重新构建报告与数据集。",
  replay: "从已保存 artifacts 重建单个 run 的 evidence 视图。",
  "inspect-codex": "检查 Codex runtime / probe / app-server 机制面。",
};

const STORAGE_KEY = "research-console.launcher.v2";
const HISTORY_KEY = "research-console.launcher.history.v1";

type LauncherMode = "quick" | "advanced";
type LaunchHistoryEntry = {
  id: string;
  kind: ActionKind;
  target: string;
  at: string;
  payloadSummary: string;
};

export function ActionLauncher() {
  const workspace = useWorkspaceIndex();
  const liveOverview = useLiveOverview();
  const hydratedWorkspace = liveOverview.data?.workspace ?? workspace.data ?? null;
  const [mode, setMode] = useState<LauncherMode>("quick");
  const [kind, setKind] = useState<ActionKind>("run");
  const [campaignDir, setCampaignDir] = useState("");
  const [campaignRoot, setCampaignRoot] = useState("");
  const [presetPath, setPresetPath] = useState("");
  const [sampleSize, setSampleSize] = useState("4");
  const [seed, setSeed] = useState("");
  const [stage, setStage] = useState("architecture-validation");
  const [model, setModel] = useState("");
  const [personality, setPersonality] = useState("");
  const [promptStyle, setPromptStyle] = useState("");
  const [experimentName, setExperimentName] = useState("");
  const [gradeCommand, setGradeCommand] = useState("");
  const [maxParallelRuns, setMaxParallelRuns] = useState("2");
  const [perRepoPrepareParallelism, setPerRepoPrepareParallelism] = useState("1");
  const [result, setResult] = useState("");
  const [busy, setBusy] = useState(false);
  const [history, setHistory] = useState<LaunchHistoryEntry[]>([]);

  useEffect(() => {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw) as Partial<Record<string, string>>;
      setCampaignDir(saved.campaignDir ?? "");
      setCampaignRoot(saved.campaignRoot ?? "");
      setPresetPath(saved.presetPath ?? "");
      setSampleSize(saved.sampleSize ?? "4");
      setSeed(saved.seed ?? "");
      setStage(saved.stage ?? "architecture-validation");
      setModel(saved.model ?? "");
      setPersonality(saved.personality ?? "");
      setPromptStyle(saved.promptStyle ?? "");
      setExperimentName(saved.experimentName ?? "");
      setGradeCommand(saved.gradeCommand ?? "");
      setMaxParallelRuns(saved.maxParallelRuns ?? "2");
      setPerRepoPrepareParallelism(saved.perRepoPrepareParallelism ?? "1");
    } catch {
      // ignore broken storage
    }
    try {
      const rawHistory = window.localStorage.getItem(HISTORY_KEY);
      if (!rawHistory) return;
      const savedHistory = JSON.parse(rawHistory) as LaunchHistoryEntry[];
      setHistory(savedHistory.slice(0, 8));
    } catch {
      // ignore broken storage
    }
  }, []);

  useEffect(() => {
    const snapshot = {
      campaignDir,
      campaignRoot,
      presetPath,
      sampleSize,
      seed,
      stage,
      model,
      personality,
      promptStyle,
      experimentName,
      gradeCommand,
      maxParallelRuns,
      perRepoPrepareParallelism,
    };
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
  }, [
    campaignDir,
    campaignRoot,
    experimentName,
    gradeCommand,
    model,
    maxParallelRuns,
    personality,
    perRepoPrepareParallelism,
    presetPath,
    promptStyle,
    sampleSize,
    seed,
    stage,
  ]);

  const isPrepare = kind === "prepare";
  const submitLabel = useMemo(() => `Launch ${kind}`, [kind]);
  const recentCampaigns = (hydratedWorkspace?.campaigns ?? []).slice(0, 4);
  const activeCampaign = liveOverview.data?.active_campaign
    ?? hydratedWorkspace?.campaigns.find((campaign) => campaign.status.includes("running"))
    ?? hydratedWorkspace?.campaigns?.[0]
    ?? null;
  const validationNotes = useMemo(() => {
    const notes: string[] = [];
    if (isPrepare) {
      if (!campaignRoot.trim()) notes.push("prepare 需要 campaign root。");
      if (!presetPath.trim()) notes.push("没有 preset path 时会依赖默认 preset。");
      if (!seed.trim()) notes.push("未设置 seed，会降低对比实验可复现性。");
      if (!maxParallelRuns.trim()) notes.push("未设置并行度，会回退到 preset 默认值。");
    } else if (!campaignDir.trim()) {
      notes.push(`${kind} 最好指向一个明确的 campaign/run 目录。`);
    }
    if ((kind === "run" || kind === "grade") && recentCampaigns.length === 0) {
      notes.push("当前 workspace 里没有最近 campaign，先 prepare 会更稳。");
    }
    return notes;
  }, [campaignDir, campaignRoot, isPrepare, kind, maxParallelRuns, presetPath, recentCampaigns.length, seed]);
  const recommendedActions = useMemo(() => {
    const actions: Array<{ label: string; detail: string; apply: () => void }> = [];
    if (!hydratedWorkspace?.campaigns.length) {
      actions.push({
        label: "先 prepare 一个新实验",
        detail: "当前工作区里还没有可直接操作的 campaign。",
        apply: () => {
          setKind("prepare");
          if (!campaignRoot) setCampaignRoot("artifacts");
        },
      });
    }
    if (activeCampaign && activeCampaign.status.includes("running")) {
      actions.push({
        label: "盯当前主战场",
        detail: `${activeCampaign.experiment_name} 正在运行，可直接绑定 active campaign。`,
        apply: () => {
          setCampaignDir(activeCampaign.path);
          setKind("report");
        },
      });
    }
    if (activeCampaign && activeCampaign.completed_run_count > 0 && activeCampaign.report_count === 0) {
      actions.push({
        label: "重建 campaign 报告",
        detail: "已经有 completed runs，但报告还没写出来。",
        apply: () => {
          setCampaignDir(activeCampaign.path);
          setKind("report");
        },
      });
    }
    if (activeCampaign && activeCampaign.completed_run_count > 0 && activeCampaign.status === "run_completed") {
      actions.push({
        label: "推进 grading",
        detail: "solver 已经跑过，适合进入 grade 阶段。",
        apply: () => {
          setCampaignDir(activeCampaign.path);
          setKind("grade");
        },
      });
    }
    return actions.slice(0, 3);
  }, [activeCampaign, campaignRoot, hydratedWorkspace?.campaigns.length]);
  const contextSummary = useMemo(() => {
    const summary = hydratedWorkspace?.summary;
    if (!summary) return null;
    return [
      { label: "Campaigns", value: String(summary.campaign_count) },
      { label: "Active", value: String(summary.active_run_count) },
      { label: "Visible Tok", value: String(summary.total_visible_output_tokens_est) },
      { label: "Tools", value: String(summary.total_tool_calls) },
    ];
  }, [hydratedWorkspace?.summary]);

  const quickActions: ActionKind[] = ["prepare", "bootstrap-local", "run", "grade", "report", "replay"];
  const presetShortcuts = [
    {
      label: "5题 2x2 pilot",
      apply() {
        setPresetPath("studies/task-presets/swebench-v1.json");
        setStage("pilot");
        setSampleSize("5");
        setExperimentName("5题 2x2 personality pilot");
      },
    },
    {
      label: "5题 2x2 并行实验",
      apply() {
        setPresetPath("studies/task-presets/swebench-v1.json");
        setStage("behavior-pilot");
        setSampleSize("5");
        setMaxParallelRuns("2");
        setPerRepoPrepareParallelism("1");
        setExperimentName("5题 2x2 behavior pilot");
      },
    },
    {
      label: "单题 2x2",
      apply() {
        setPresetPath("studies/task-presets/swebench-v1.json");
        setStage("architecture-validation");
        setSampleSize("1");
        setExperimentName("单题 2x2 research sanity");
      },
    },
    {
      label: "SWE-bench 标准批次",
      apply() {
        setPresetPath("studies/task-presets/swebench-v1.json");
        setStage("evidence-batch");
        setSampleSize("10");
        setExperimentName("SWE-bench evidence batch");
      },
    },
  ];

  const cloneCampaignConfig = (campaignPath: string) => {
    setCampaignDir(campaignPath);
    setCampaignRoot("artifacts");
    setPresetPath("studies/task-presets/swebench-v1.json");
    setStage("behavior-pilot");
    setExperimentName(`rerun ${campaignPath.split("/").pop() ?? "campaign"}`);
    setKind("run");
  };

  const rememberLaunch = (kind: ActionKind, target: string, payloadSummary: string) => {
    const entry: LaunchHistoryEntry = {
      id: `${Date.now()}-${kind}`,
      kind,
      target,
      at: new Date().toISOString(),
      payloadSummary,
    };
    const next = [entry, ...history].slice(0, 8);
    setHistory(next);
    window.localStorage.setItem(HISTORY_KEY, JSON.stringify(next));
  };

  const submit = async () => {
    setBusy(true);
    try {
      if (kind === "prepare") {
        const body: Record<string, unknown> = {
          campaign_root: campaignRoot,
        };
        if (presetPath) body.preset_path = presetPath;
        if (sampleSize) body.sample_size = Number(sampleSize);
        if (seed) body.seed = seed;
        if (stage) body.stage = stage;
        if (model) body.model = model;
        if (personality) body.personality = personality;
        if (promptStyle) body.prompt_style = promptStyle;
        if (experimentName) body.experiment_name = experimentName;
        if (maxParallelRuns) body.max_parallel_runs = Number(maxParallelRuns);
        if (perRepoPrepareParallelism) body.per_repo_prepare_parallelism = Number(perRepoPrepareParallelism);
        const launched = await api.action(kind, body);
        setResult(`${launched.kind} -> ${launched.process_id}`);
        rememberLaunch(kind, campaignRoot, [presetPath || "default preset", stage || "no stage", `n=${sampleSize || "?"}`].join(" · "));
        return;
      }

      const body: Record<string, unknown> = {};
      if (campaignDir) body.campaign_dir = campaignDir;
      if (gradeCommand) body.command = gradeCommand;
      const launched = await api.action(kind, body);
      setResult(`${launched.kind} -> ${launched.process_id}`);
      rememberLaunch(kind, campaignDir, gradeCommand || "default flow");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="action-console">
      <div className="action-head">
        <div>
          <div className="section-label">Console Launchpad</div>
          <div className="mono-note">受管动作、研究 preset 与运行参数都会在这里汇总。</div>
        </div>
        <SegmentedTabs
          items={[
            { value: "quick", label: "Quick" },
            { value: "advanced", label: "Advanced" },
          ]}
          value={mode}
          onChange={(value) => setMode(value as LauncherMode)}
        />
      </div>

      {mode === "quick" ? (
        <div className="quick-action-grid">
          {quickActions.map((item) => (
            <button
              key={item}
              type="button"
              className={`quick-action-button${kind === item ? " quick-action-active" : ""}`}
              onClick={() => setKind(item)}
            >
              <span>{item}</span>
              <small>{actionHelp[item]}</small>
            </button>
          ))}
        </div>
      ) : null}

      {mode === "advanced" ? (
        <div className="launcher-presets">
          <div className="section-label">Preset Shortcuts</div>
          <div className="chip-row">
            {presetShortcuts.map((preset) => (
              <button key={preset.label} type="button" className="artifact-chip" onClick={preset.apply}>
                {preset.label}
              </button>
            ))}
          </div>
        </div>
      ) : null}

      <div className="launcher-context">
        <div className="section-label">Workspace Context</div>
        {contextSummary ? (
          <div className="metric-grid metric-grid-compact">
            {contextSummary.map((item) => (
              <div key={item.label}>
                <span className="metric-label">{item.label}</span>
                <strong>{item.value}</strong>
              </div>
            ))}
          </div>
        ) : (
          <div className="mono-note">等待 workspace / live overview 水合…</div>
        )}
      </div>

      {activeCampaign ? (
        <div className="launcher-presets">
          <div className="section-label">Context Quick Actions</div>
          <div className="chip-row">
            <button type="button" className="artifact-chip" onClick={() => setCampaignDir(activeCampaign.path)}>
              use active campaign
            </button>
            <button type="button" className="artifact-chip" onClick={() => cloneCampaignConfig(activeCampaign.path)}>
              clone active config
            </button>
            <button
              type="button"
              className="artifact-chip"
              onClick={() => {
                setKind("grade");
                setCampaignDir(activeCampaign.path);
              }}
            >
              grade active
            </button>
            <button
              type="button"
              className="artifact-chip"
              onClick={() => {
                setKind("report");
                setCampaignDir(activeCampaign.path);
              }}
            >
              rebuild active report
            </button>
          </div>
        </div>
      ) : null}

      {recommendedActions.length ? (
        <div className="launcher-presets">
          <div className="section-label">Recommended Next Step</div>
          <div className="artifact-list artifact-list-column artifact-ledger">
            {recommendedActions.map((action) => (
              <button key={action.label} type="button" className="artifact-row-button" onClick={action.apply}>
                <div className="artifact-row-main">
                  <strong>{action.label}</strong>
                  <span className="artifact-role">operator recommendation</span>
                  <span className="artifact-scope">{action.detail}</span>
                </div>
              </button>
            ))}
          </div>
        </div>
      ) : null}

      {recentCampaigns.length ? (
        <div className="launcher-presets">
          <div className="section-label">Recent Campaign Shortcuts</div>
          <div className="chip-row">
            {recentCampaigns.map((campaign) => (
              <button
                key={campaign.campaign_id}
                type="button"
                className="artifact-chip"
                onClick={() => {
                  setCampaignDir(campaign.path);
                  if (!isPrepare) setKind("run");
                }}
                title={campaign.path}
              >
                {campaign.campaign_id}
              </button>
            ))}
          </div>
        </div>
      ) : null}

      {history.length ? (
        <div className="launcher-presets">
          <div className="section-label">Recent Launches</div>
          <div className="chip-row">
            {history.map((entry) => (
              <button
                key={entry.id}
                type="button"
                className="artifact-chip"
                title={`${entry.payloadSummary} · ${entry.at}`}
                onClick={() => {
                  setKind(entry.kind);
                  setCampaignDir(entry.target);
                }}
              >
                {entry.kind} · {entry.target.split("/").pop() ?? entry.target}
              </button>
            ))}
          </div>
        </div>
      ) : null}

      {liveOverview.data?.active_campaign_summary ? (
        <div className="launcher-presets">
          <div className="section-label">Active Campaign Snapshot</div>
          <div className="metric-grid metric-grid-compact">
            <div>
              <span className="metric-label">Active Live</span>
              <strong>{liveOverview.data.active_campaign_summary.active_live_runs.length}</strong>
            </div>
            <div>
              <span className="metric-label">Live Msg</span>
              <strong>{liveOverview.data.active_campaign_summary.live_message_count}</strong>
            </div>
            <div>
              <span className="metric-label">Live Tool</span>
              <strong>{liveOverview.data.active_campaign_summary.live_tool_count}</strong>
            </div>
            <div>
              <span className="metric-label">Warnings</span>
              <strong>{liveOverview.data.active_campaign_summary.active_warning_count}</strong>
            </div>
          </div>
          {liveOverview.data.operator_notices?.length ? (
            <div className="mono-note">
              {liveOverview.data.operator_notices[0]}
            </div>
          ) : null}
        </div>
      ) : null}

      <div className="field-grid">
        <label className="field-stack">
          <span className="metric-label">Action</span>
          <select value={kind} onChange={(event) => setKind(event.target.value as ActionKind)}>
            {Object.keys(actionHelp).map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>
        {isPrepare ? (
          <>
            <label className="field-stack">
              <span className="metric-label">Campaign Root</span>
              <input
                name="campaign_root"
                autoComplete="off"
                placeholder="artifacts"
                value={campaignRoot}
                onChange={(event) => setCampaignRoot(event.target.value)}
              />
            </label>
            <label className="field-stack field-span-2">
              <span className="metric-label">Preset Path</span>
              <input
                name="preset_path"
                autoComplete="off"
                placeholder="studies/task-presets/swebench-v1.json"
                value={presetPath}
                onChange={(event) => setPresetPath(event.target.value)}
              />
            </label>
            <label className="field-stack">
              <span className="metric-label">Sample Size</span>
              <input
                name="sample_size"
                inputMode="numeric"
                value={sampleSize}
                onChange={(event) => setSampleSize(event.target.value)}
              />
            </label>
            <label className="field-stack">
              <span className="metric-label">Seed</span>
              <input
                name="seed"
                autoComplete="off"
                placeholder="research-pilot"
                value={seed}
                onChange={(event) => setSeed(event.target.value)}
              />
            </label>
            <label className="field-stack">
              <span className="metric-label">Stage</span>
              <input
                name="stage"
                autoComplete="off"
                value={stage}
                onChange={(event) => setStage(event.target.value)}
              />
            </label>
            <label className="field-stack">
              <span className="metric-label">Max Parallel Runs</span>
              <input
                name="max_parallel_runs"
                inputMode="numeric"
                value={maxParallelRuns}
                onChange={(event) => setMaxParallelRuns(event.target.value)}
              />
            </label>
            <label className="field-stack">
              <span className="metric-label">Per-repo Prepare Parallelism</span>
              <input
                name="per_repo_prepare_parallelism"
                inputMode="numeric"
                value={perRepoPrepareParallelism}
                onChange={(event) => setPerRepoPrepareParallelism(event.target.value)}
              />
            </label>
            <label className="field-stack">
              <span className="metric-label">Model Override</span>
              <input
                name="model"
                autoComplete="off"
                placeholder="gpt-5.4"
                value={model}
                onChange={(event) => setModel(event.target.value)}
              />
            </label>
            <label className="field-stack">
              <span className="metric-label">Personality</span>
              <select value={personality} onChange={(event) => setPersonality(event.target.value)}>
                <option value="">preset default</option>
                <option value="friendly">friendly</option>
                <option value="pragmatic">pragmatic</option>
                <option value="none">none</option>
              </select>
            </label>
            <label className="field-stack">
              <span className="metric-label">Prompt Style</span>
              <input
                name="prompt_style"
                autoComplete="off"
                placeholder="terse_engineer"
                value={promptStyle}
                onChange={(event) => setPromptStyle(event.target.value)}
              />
            </label>
            <label className="field-stack field-span-2">
              <span className="metric-label">Experiment Name</span>
              <input
                name="experiment_name"
                autoComplete="off"
                placeholder="Codex 模型人格化行为对比研究"
                value={experimentName}
                onChange={(event) => setExperimentName(event.target.value)}
              />
            </label>
          </>
        ) : (
          <>
            <label className="field-stack field-span-2">
              <span className="metric-label">Campaign / Run Dir</span>
              <input
                name="campaign_dir"
                autoComplete="off"
                placeholder="artifacts/..."
                value={campaignDir}
                onChange={(event) => setCampaignDir(event.target.value)}
              />
            </label>
            {kind === "grade" ? (
              <label className="field-stack field-span-2">
                <span className="metric-label">Grade Command</span>
                <input
                  name="grade_command"
                  autoComplete="off"
                  placeholder="custom grader command (optional)"
                  value={gradeCommand}
                  onChange={(event) => setGradeCommand(event.target.value)}
                />
              </label>
            ) : null}
          </>
        )}
      </div>
      {validationNotes.length ? (
        <div className="launcher-warnings">
          <div className="section-label">Launch Notes</div>
          <ul className="evidence-list">
            {validationNotes.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        </div>
      ) : null}
      <div className="action-footer">
        <p className="action-help">{actionHelp[kind]}</p>
        <div className="action-controls">
          <button type="button" onClick={() => void submit()} disabled={busy}>
            {busy ? "Launching…" : submitLabel}
          </button>
          {result ? <span className="action-result">{result}</span> : null}
        </div>
      </div>
    </div>
  );
}
