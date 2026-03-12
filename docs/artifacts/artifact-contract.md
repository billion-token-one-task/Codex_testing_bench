# Artifact 契约

## 目标

定义每个 campaign 和每个 run 应该产出什么，哪些文件适合公开放在 GitHub 上，哪些只应保留在本地。

同时，这份文档现在也定义：

- 哪些 artifact 是 `raw truth`
- 哪些 artifact 是 `derived evidence`
- 哪些 artifact 是 `derived summary`
- 哪些 artifact 是给研究者直接阅读的 `human-readable dossier`

这一层的分类应与：

- [Codex 可观测面契约](/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/codex-observability-contract.md)
- [codex-observability-map.json](/Users/kevinlin/Downloads/CodexPlusClaw/studies/observability/codex-observability-map.json)

保持一致。

## Campaign 级别产物

期望存在的 campaign 级文件：

- `campaign-manifest.json`
- `selected-dataset.json`
- `codex-architecture-map.json`
- `grounding-claims.json`
- `codex-unique-claims.json`
- `model-catalog-snapshot.json`
- `experiment-lock.json`
- `benchmark-research-profile.json`
- `studies/hypotheses/model-behavior-v1.json` 的 campaign 引用
- `predictions.jsonl`
- `grader.json`
- `reports/report.txt`
- `reports/model-comparison.md`
- `reports/verbosity-analysis.md`
- `reports/tool-language-coupling.md`
- `reports/linguistic-profile.md`
- `reports/phrase-and-tone-analysis.md`
- `reports/bridge-language-analysis.md`
- `reports/tool-inventory.md`
- `reports/tool-route-analysis.md`
- `reports/personality-analysis.md`
- `reports/personality-mechanism-analysis.md`
- `reports/patch-mechanism-analysis.md`
- `reports/skill-mechanism-analysis.md`
- `reports/instruction-stratification-analysis.md`
- `reports/cohort-pair-analysis.md`
- `datasets/*.csv`

这些文件回答的是：

- 跑了什么
- 选中了哪些任务
- 观察的是 Codex 哪些子系统
- 哪些 claim 在研究范围内
- 汇总后的证据是什么样子

研究控制台当前会直接消费这些 campaign 级文件：

- `reports/*.md`
- `reports/report.txt`
- `datasets/*.csv`
- `campaign-manifest.json`

## 单次运行 / 单次尝试产物

期望存在的 attempt 级文件：

- `prompt.txt`
- `environment-plan.json`
- `raw-agent-events.jsonl`
- `raw-diagnostics.jsonl`
- `codex-probe-events.jsonl`
- `lifecycle-events.jsonl`
- `token-snapshots.jsonl`
- `turn-metrics.jsonl`
- `message-metrics.jsonl`
- `command-events.jsonl`
- `tool-events.jsonl`
- `skill-events.jsonl`
- `personality-events.jsonl`
- `skill-mechanism.jsonl`
- `verbosity-tool-coupling.jsonl`
- `patch-events.jsonl`
- `patch-chain.jsonl`
- `anomalies.jsonl`
- `patch.diff`
- `run-summary.json`
- `probe-events.jsonl`

研究控制台当前直接可视化的重点文件包括：

- `message-metrics.jsonl`
- `tool-events.jsonl`
- `patch-chain.jsonl`
- `personality-events.jsonl`
- `skill-mechanism.jsonl`
- `verbosity-tool-coupling.jsonl`
- `probe-summary.json`
- `claim-evidence.json`
- `run-evidence.txt`
- `attempt-log.txt`
- `replay.json`

目前重点 CSV 包括：

- `campaign_runs.csv`
- `claim_evidence.csv`
- `model_pair_deltas.csv`
- `task_class_summary.csv`
- `turn_metrics.csv`
- `message_metrics.csv`
- `message_lexical_summary.csv`
- `message_discourse_summary.csv`
- `message_style.csv`
- `cohort_lexical_summary.csv`
- `model_phrase_deltas.csv`
- `personality_phrase_deltas.csv`
- `tool_usage.csv`
- `tool_inventory.csv`
- `tool_route_summary.csv`
- `tool_by_turn.csv`
- `verbosity_tool_coupling.csv`
- `personality_mechanism.csv`
- `patch_chain.csv`
- `skill_mechanism.csv`

当前与“模型是否更愿意说、说了什么、这些话和工具怎么耦合”最直接相关的文件是：

- `message-metrics.jsonl`
  - 每条可见输出的长度、词汇、话语功能、语气分数
- `message_style.csv`
  - 适合直接做 cohort 对比的 message-level 风格数据集
- `tool_inventory.csv`
  - 具体工具级别的调用画像
- `tool_by_turn.csv`
  - turn 级语言-工具耦合基础表
- `verbosity_tool_coupling.csv`
  - 研究 `talk_then_act` / `silent_tool_burst` / `micro_narrated_tool_burst` 的主表
- `personality_mechanism.csv`
  - requested/effective personality、fallback、model-native 指令保留情况
- `patch_chain.csv`
  - patch begin/end、patch 审批、patch 失败与时序链路
- `skill_mechanism.csv`
  - skill catalog 与权限相关机制事件

## Artifact 角色分类

### Raw truth

这些文件最接近 Codex 原生输出，是 ground truth：

- `raw-agent-events.jsonl`
- `raw-diagnostics.jsonl`
- `codex-probe-events.jsonl`
- `patch.diff`

### Derived evidence

这些文件来自 bench 后处理，但仍然紧贴原始事件：

- `lifecycle-events.jsonl`
- `token-snapshots.jsonl`
- `turn-metrics.jsonl`
- `message-metrics.jsonl`
- `command-events.jsonl`
- `tool-events.jsonl`
- `skill-events.jsonl`
- `personality-events.jsonl`
- `skill-mechanism.jsonl`
- `patch-events.jsonl`
- `patch-chain.jsonl`
- `grade-events.jsonl`
- `anomalies.jsonl`
- `verbosity-tool-coupling.jsonl`
- `probe-events.jsonl`

注意：

- `message-metrics.jsonl` 中长度、句子、段落、代码块、词频等大多属于 `observed` 或 `estimated`
- discourse category、bridge language、state externalization、tool-grounded commentary 等属于 `inferred`
- tool route 的粗粒度（shell / apply_patch / MCP / dynamic_tool）通常是 `observed`
- 更细粒度的 mediation tax 仍应视为 `inferred`

### Derived summary

这些文件用于 campaign 级归纳与报告：

- `probe-summary.json`
- `claim-evidence.json`
- `run-summary.json`

### Human-readable dossier

这些文件是研究者第一眼应该打开的文件：

- `run-evidence.txt`
- `attempt-log.txt`
- `replay.json`
- `reports/*.md`
- `reports/report.txt`
- `datasets/*.csv`

## 人类阅读的优先顺序

如果一个研究者在做单次 run 的排查，建议顺序是：

1. `run-summary.json`
2. `run-evidence.txt`
3. `attempt-log.txt`
4. `probe-summary.json`

raw JSONL 仍然是 source of truth，但它们不是给人类第一眼打开的入口。

## 哪些适合提交，哪些只保留本地

适合提交到仓库的：

- campaign manifest
- 选中数据集快照
- architecture map
- claim catalog
- `report.txt`
- `run-summary.json`
- `probe-summary.json`
- `claim-evidence.json`
- `run-evidence.txt`
- `attempt-log.txt`
- `replay.json`
- campaign 级专题 Markdown
- CSV 数据集

通常只保留本地的：

- 重量级预热缓存
- worktree / workspace
- 全量 raw JSONL 流（除非刻意整理后纳入）
- 较大的 prompt 与环境 staging 文件
- 体积很大的临时 patch/runtime 文件

## 回填规则

当满足以下条件时，reporting 层允许用旧的 raw artifact 回填新的派生产物：

- raw event stream 仍然存在
- report schema 已经演进
- 当前 `run-summary.json` 落后于新的 report / probe 结构

这是有意设计的。它允许旧 campaign 在 **不重跑 Codex** 的情况下获得更好的证据产物。

## 连续输出线

当前目标行为是：

- `run` 完成后自动生成：
  - campaign 级 `report.txt`
  - Markdown 专题
  - `datasets/*.csv`
- `grade` 完成后自动 ingest 官方评分结果并自动刷新这些产物

因此：

- `report` 命令应被视作重建 / 回填入口
- 但不是正常工作流里唯一生成报告的入口

## 失败语义

artifact 完整性本身就是解释的一部分。

这个 bench 应该明确区分：

- 一个 run 是否压根没有走到 patch extraction
- 一个 run 是否走到了 patch extraction 但没有完成 grading
- 某份 report 是否建立在不完整证据之上
- 某个 run 是否缺失特定的 normalized 文件

缺失 artifact 应被视为 **evidence gap**，而不是被静默忽略。

## 字段级 classification

summary、Markdown 与 CSV 中的字段，也应尽量能够区分：

- `observed`
- `inferred`
- `estimated`

如果某个字段本质上是推断层，就不应当在报告里被写成 Codex 原生直接给出的事实。
