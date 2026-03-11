# 快速上手

## 目标

在本地跑起一个可复现的 Codex 研究 campaign，并最终得到一套人类可读的证据包。

## 前置条件

- macOS 主机
- 已安装 Rust toolchain
- 已安装 Python，用于 benchmark 特定的评分流程
- `repos/codex` 下已经存在 vendored Codex
- 你要研究的 Codex runtime 所需的本地 auth / config 已经可用

## 标准工作流

以下命令都从 [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench) 目录执行。

### 1. 准备一个 campaign

```bash
cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../artifacts \
  --preset-path ../studies/task-presets/swebench-v1.json \
  --stage behavior-pilot \
  --seed my-study
```

这一步会写出：

- `campaign-manifest.json`
- `selected-dataset.json`
- `codex-architecture-map.json`
- campaign 局部拷贝的 claim catalog
- `model-catalog-snapshot.json`
- `experiment-lock.json`

如果 preset 含有多 cohort 定义，那么这里会一次性展开出多个 `model × personality` 运行组；同一个 `instance_id` 会在不同 cohort 下形成配对样本。

### 2. 预热本地资源

```bash
cargo run -p codex-bench-cli -- bootstrap-local \
  --campaign-dir ../artifacts/<campaign-id>
```

这是推荐的预处理步骤，因为它会：

- 构建本地 bench 二进制
- 确保 benchmark 所需资源下载到仓库文件系统内部
- 尽可能为选中的任务预热共享 git object cache

### 3. 运行 benchmark

```bash
cargo run -p codex-bench-cli -- run ../artifacts/<campaign-id>
```

运行过程中，可以重点查看：

- `artifacts/<campaign-id>/runs/<instance>/manifest.json`
- `artifacts/<campaign-id>/runs/<instance>/attempt-01/`

### 4. 进行评分

以 SWE-bench 为例：

```bash
cargo run -p codex-bench-cli -- grade ../artifacts/<campaign-id> \
  --command 'python -m swebench.harness.run_evaluation --predictions_path {predictions}'
```

### 5. 生成 evidence dossier

```bash
cargo run -p codex-bench-cli -- report ../artifacts/<campaign-id>
```

核心输出文件：

- `artifacts/<campaign-id>/reports/report.txt`
- `artifacts/<campaign-id>/reports/model-comparison.md`
- `artifacts/<campaign-id>/reports/verbosity-analysis.md`
- `artifacts/<campaign-id>/reports/tool-language-coupling.md`
- `artifacts/<campaign-id>/runs/<instance>/attempt-01/run-evidence.txt`
- `artifacts/<campaign-id>/runs/<instance>/attempt-01/attempt-log.txt`
- `artifacts/<campaign-id>/datasets/*.csv`

## 一个完成的运行应该怎么看

如果你想用最短路径理解一题的运行情况，建议顺序是：

1. `manifest.json`
2. `run-summary.json`
3. `run-evidence.txt`
4. `attempt-log.txt`

如果你需要全量机器级别细节，则看：

1. `raw-agent-events.jsonl`
2. `codex-probe-events.jsonl`
3. `probe-events.jsonl`
4. `message-metrics.jsonl`
5. `verbosity-tool-coupling.jsonl`
6. `turn-metrics.jsonl`
7. `tool-events.jsonl`
8. `command-events.jsonl`

## 策略说明

- benchmark run 只走本地路径
- benchmark run 只研究 Codex，不经过 OpenClaw
- 被评测的 benchmark run 中，web search 被显式禁用
- 如果出现被禁止的 web-search 事件，该 run 会被视为无效

## 故障排查

如果 campaign 看起来卡住了：

- 用 `ps` 看顶层 run 进程
- 查看当前活跃 run 的 manifest
- 查看 `raw-agent-events.jsonl` 是否还在增长
- 查看 `codex-probe-events.jsonl` 是否还在增长

如果 report 缺少较新的 artifact：

- 重新运行一次 `report`
- 当前 reporting 路径支持在条件满足时，从旧的 raw artifact 回填新的派生产物
