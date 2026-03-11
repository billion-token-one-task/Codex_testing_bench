# Artifact 契约

## 目标

定义每个 campaign 和每个 run 应该产出什么，哪些文件适合公开放在 GitHub 上，哪些只应保留在本地。

## Campaign 级别产物

期望存在的 campaign 级文件：

- `campaign-manifest.json`
- `selected-dataset.json`
- `codex-architecture-map.json`
- `grounding-claims.json`
- `codex-unique-claims.json`
- `predictions.jsonl`
- `grader.json`
- `reports/report.txt`

这些文件回答的是：

- 跑了什么
- 选中了哪些任务
- 观察的是 Codex 哪些子系统
- 哪些 claim 在研究范围内
- 汇总后的证据是什么样子

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
- `command-events.jsonl`
- `tool-events.jsonl`
- `skill-events.jsonl`
- `patch-events.jsonl`
- `anomalies.jsonl`
- `patch.diff`
- `run-summary.json`
- `probe-events.jsonl`
- `probe-summary.json`
- `claim-evidence.json`
- `run-evidence.txt`
- `attempt-log.txt`
- `replay.json`

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

## 失败语义

artifact 完整性本身就是解释的一部分。

这个 bench 应该明确区分：

- 一个 run 是否压根没有走到 patch extraction
- 一个 run 是否走到了 patch extraction 但没有完成 grading
- 某份 report 是否建立在不完整证据之上
- 某个 run 是否缺失特定的 normalized 文件

缺失 artifact 应被视为 **evidence gap**，而不是被静默忽略。
