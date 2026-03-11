# Codex 模型 × Personality 行为研究

## 研究问题

这条研究主线聚焦：

- `gpt-5.4` 与 `gpt-5.3-codex` 的行为差异
- `friendly` 与 `pragmatic` personality 是否只影响表层语气，还是会改变 agent policy
- “说更多”到底表现为哪些可观察语言行为
- 这些语言行为如何与工具调用、验证、patch 生成、Codex harness 的 regulation 机制耦合

## 第一阶段实验矩阵

默认使用同一批任务样本，在外层 bench 中展开四个 cohort：

- `gpt-5.3-codex × pragmatic`
- `gpt-5.3-codex × friendly`
- `gpt-5.4 × pragmatic`
- `gpt-5.4 × friendly`

默认 benchmark：

- `SWE-bench Verified`

默认比较方式：

- 同一 `instance_id` 在四个 cohort 下形成配对样本
- 优先做 paired comparison，而不是跨样本 aggregate

## 主要证据层

### 用户可见输出

重点分析：

- `message-metrics.jsonl`
- `datasets/message_metrics.csv`
- `datasets/message_lexical_summary.csv`
- `datasets/message_discourse_summary.csv`

它们回答：

- 谁说得更多
- 说的主要是什么
- 哪些话语功能在某个 cohort 中上升
- 哪些词、bigram、trigram 具有区分度

### 工具与语言耦合

重点分析：

- `verbosity-tool-coupling.jsonl`
- `datasets/verbosity_tool_coupling.csv`
- `reports/tool-language-coupling.md`
- `reports/tool-inventory.md`
- `reports/tool-route-analysis.md`

它们回答：

- 说和工具之间是如何耦合的
- 哪个 cohort 更像 `talk_then_act`
- 哪个 cohort 更像 `silent_tool_burst`
- 具体调用了哪些 Codex 工具、多少次、在哪些上下文中出现

### Harness 机制

重点分析：

- `codex-probe-events.jsonl`
- `probe-events.jsonl`
- `reports/personality-mechanism-analysis.md`
- `reports/instruction-stratification-analysis.md`

它们回答：

- personality 是否通过 model-native instruction 生效
- instruction layering 是否与 verbosity 上升相关
- Codex 的 config freeze / compaction / persistence 是否调节了语言外显化

## 关键输出

campaign 级：

- `reports/report.txt`
- `reports/model-comparison.md`
- `reports/verbosity-analysis.md`
- `reports/linguistic-profile.md`
- `reports/phrase-and-tone-analysis.md`
- `reports/bridge-language-analysis.md`
- `reports/tool-language-coupling.md`
- `reports/tool-inventory.md`
- `reports/tool-route-analysis.md`
- `reports/personality-analysis.md`
- `reports/personality-mechanism-analysis.md`
- `reports/instruction-stratification-analysis.md`
- `reports/cohort-pair-analysis.md`

数据集：

- `datasets/campaign_runs.csv`
- `datasets/message_metrics.csv`
- `datasets/message_lexical_summary.csv`
- `datasets/message_discourse_summary.csv`
- `datasets/tool_usage.csv`
- `datasets/tool_inventory.csv`
- `datasets/tool_route_summary.csv`
- `datasets/tool_by_turn.csv`
- `datasets/verbosity_tool_coupling.csv`
- `datasets/model_pair_deltas.csv`

## 方法边界

这条研究线默认遵守：

- 只对**用户可见输出**做“说更多”分析
- 不把 hidden CoT 当作已观测事实
- 任何字段都应根据 [Codex 可观测面契约](/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/codex-observability-contract.md) 区分 `observed / inferred / estimated`

## 当前最值得验证的假设

- `H1`: `gpt-5.4` 的用户可见输出显著多于 `gpt-5.3-codex`
- `H2`: `gpt-5.4` 多出来的输出更多是 bridge / verification / decision explanation，而不是单纯礼貌包装
- `H3`: `friendly` 不只是 cosmetic，而会改变可观察状态外显化方式
- `H4`: `pragmatic` 会压低表层 verbosity，但不一定降低工具密度
- `H5`: 一部分差异来自 Codex harness 的 instruction layering、tool mediation 与 regulation 机制，而不只是 base model
