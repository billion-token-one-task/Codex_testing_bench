import { useMemo, useState } from "react";

import { api } from "../lib/api";

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

export function ActionLauncher() {
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
  const [result, setResult] = useState("");
  const [busy, setBusy] = useState(false);

  const isPrepare = kind === "prepare";
  const submitLabel = useMemo(() => `Launch ${kind}`, [kind]);

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
        const launched = await api.action(kind, body);
        setResult(`${launched.kind} -> ${launched.process_id}`);
        return;
      }

      const body: Record<string, unknown> = {};
      if (campaignDir) body.campaign_dir = campaignDir;
      if (gradeCommand) body.command = gradeCommand;
      const launched = await api.action(kind, body);
      setResult(`${launched.kind} -> ${launched.process_id}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="action-console">
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
            <label className="field-stack">
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
            <label className="field-stack">
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
