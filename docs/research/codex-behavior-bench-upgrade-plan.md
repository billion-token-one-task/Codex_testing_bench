# Codex 行为研究 Bench 升级执行计划

## 目的

把当前 `Codex_testing_bench` 从一个已经可用的研究原型，升级成一个：

- 以 `Codex` 真实可观测面为基础
- 能自动产出完整证据包
- 能支持 `model × personality × benchmark × task class` 对比
- 能生成论文可直接复用的 Markdown 与 CSV 数据集

的研究平台。

这份计划是**执行文档**，不是研究结论。后续所有实现都应对照这份计划逐步完成。

## 总原则

### 原则 1：先观测边界，后扩功能

任何新字段、新 probe、新报告、新 CSV，在进入 bench 之前，都必须先回答：

- 它是 `observed` 还是 `inferred`
- 它的 source refs 是什么
- 它是否会在写论文时被误当作“Codex 原生直接给出的信息”

### 原则 2：先产出不断线，再追求分析更重

研究流水线应该默认是完整的：

- `prepare`
- `run`
- 自动 derivation
- 自动 report
- 自动 datasets
- `grade`
- 自动 ingest
- 自动刷新 report 与 datasets

### 原则 3：证据优先于叙事

这套 bench 的主输出是：

- `report.txt`
- `run-evidence.txt`
- 专题 Markdown
- CSV 数据集

不是 final paper。所有结论都应能回指本地 artifact。

## Phase 1：建立 Codex 可观测面契约

### 目标

建立一份正式的 observability contract，明确：

- Codex 原生直接暴露了什么
- bench 可以稳定推导什么
- 哪些只能弱推断
- 哪些当前不可观测

### 主要产物

- `docs/research/codex-observability-contract.md`
- `studies/observability/codex-observability-map.json`

### 完成标准

- turn / token / exec / MCP / patch / compaction / collaboration / personality request/effective 等主要 seam 全部分类
- 文档、结构化 map、refs 三者一致

## Phase 2：按契约重构 schema 与 artifact contract

### 目标

把 bench 中现有字段重新整理，避免把推断层字段伪装成原生观测。

### 主要工作

- 重构 core types
- 为派生字段增加 `classification`
- 统一 `sourceRefs`
- 更诚实地命名 skill / route / overhead 类字段

### 相关文件

- `bench/crates/codex-bench-core/src/types.rs`
- `bench/crates/codex-bench-core/src/artifacts.rs`
- `docs/artifacts/artifact-contract.md`

### 完成标准

- 关键 summary 与 row schema 都能明确区分 `observed/inferred/estimated`
- artifact contract 文档同步更新

## Phase 3：修复连续输出线

### 目标

让 `run` 与 `grade` 默认生成和刷新完整研究证据，而不是依赖额外手动 `report`。

### 主要工作

#### `run`

- 每个 attempt 完成后：
  - `run-evidence.txt`
  - `attempt-log.txt`
  - `message-metrics.jsonl`
  - `verbosity-tool-coupling.jsonl`
  - `probe-summary.json`
  - `claim-evidence.json`
  必须已经落盘

- campaign solver 全部结束后自动生成：
  - `reports/report.txt`
  - Markdown 专题
  - `datasets/*.csv`

#### `grade`

- 自动 ingest 官方 grading 结果
- 更新 campaign / run manifest
- 自动刷新所有报告与数据集

### 完成标准

- 不手动执行 `report`，run 后也能看到完整 campaign 级报告和 datasets
- grade 后报告自动变成“已评分版本”

## Phase 4：扩展 Codex 具体工具画像

### 目标

不再只看 `tool_count`，而是研究：

- 调用了哪个具体工具
- 调了多少次
- 在什么上下文里调用
- 调用前后说了什么
- 是否后接 patch / verification

### 主要工作

扩展逐 tool call 字段：

- `tool_kind`
- `tool_name`
- `turn_id`
- `call_id`
- `seq`
- `cwd`
- `duration_ms`
- `success/failure`
- `stderr_present`
- `output_size`
- `structured_output`
- `preceded_by_commentary_tokens`
- `followed_by_commentary_tokens`
- `followed_by_patch_event`
- `followed_by_verification_event`

### 主要输出

- `reports/tool-inventory.md`
- `reports/tool-route-analysis.md`
- `datasets/tool_inventory.csv`
- `datasets/tool_route_summary.csv`
- `datasets/tool_by_turn.csv`

## Phase 5：增强 message-level NLP

### 目标

不只研究“说了多少”，还研究：

- 说了什么
- 怎么说
- 哪些词最常出现
- 哪些短语组合最常出现
- 哪类 discourse function 在不同模型 / personality 下上升

### 技术路线

- 本地
- 确定性
- 英语优先
- 中文先做基础统计

### 主要输出

- 扩展 `message-metrics.jsonl`
- `datasets/message_lexical_summary.csv`
- `datasets/message_discourse_summary.csv`
- `datasets/model_phrase_deltas.csv`
- `datasets/personality_phrase_deltas.csv`

### Markdown 专题

- `reports/linguistic-profile.md`
- `reports/phrase-and-tone-analysis.md`
- `reports/bridge-language-analysis.md`

## Phase 6：增强 language × tool coupling 分析

### 目标

验证并量化：

- 5.4 是否更会“说”
- 它说的内容是否与工具调用更紧耦合
- `friendly` 是否放大这种耦合
- `pragmatic` 是否压缩了表层 verbosity 但保留动作效率

### 核心指标

- `tokens_before_first_tool`
- `visible_text_before_first_tool`
- `visible_text_between_tool_calls`
- `visible_text_after_last_tool`
- `tool_calls_per_1k_visible_tokens`
- `visible_tokens_per_tool_call`
- `commentary_to_tool_delay_ms`
- `tool_result_to_commentary_delay_ms`
- `commentary_to_patch_delay_ms`
- `commentary_to_verification_delay_ms`
- `tool_burst_count`
- `silent_tool_burst_count`
- `micro_narrated_tool_burst_count`

### 模式标签

- `talk_then_act`
- `act_then_report`
- `micro_narrated_tooling`
- `silent_tool_burst`
- `verbose_sparse_tooling`
- `high_tool_high_narration`
- `verification_narration_heavy`
- `cleanup_narration_heavy`

## Phase 7：扩展 personality / instruction / skill 机制分析

### 目标

把 personality 与 harness 机制联系起来，而不是只做风格表层统计。

### 主要分析面

- requested vs effective personality
- personality fallback
- model-native personality 注入是否保留
- instruction layering 变化
- reconstructed context 的作用
- skill catalog 与 inferred skill access timing

### 主要输出

- `reports/personality-analysis.md`
- `reports/personality-mechanism-analysis.md`
- `reports/instruction-stratification-analysis.md`

## Phase 8：增强 task-class-aware 分析

### 目标

不要只看全局平均，而是按任务类型分析：

- 哪类任务让 5.4 更会说
- 哪类任务让 `friendly` 最明显
- 哪类任务最容易把 verbosity 变成热量

### 强制 task class

- `bootstrap-heavy`
- `verification-heavy`
- `search-heavy`
- `patch-heavy`
- `compaction-likely`

### 主要输出

- `reports/task-class-analysis.md`
- `datasets/task_class_summary.csv`

## Phase 9：扩展 benchmark adapter，面向更多 2026 benchmark

### 目标

让研究层而不是只有 runner 层具备复用性。

### 要补的 adapter 能力

- `task_classification()`
- `expected_verification_strength()`
- `expected_context_pressure()`
- `expected_tool_mix()`
- `expected_bootstrap_risk()`
- `expected_language_need()`
- `language_profile_hint()`
- `tool_profile_hint()`
- `interaction_style_hint()`
- `default_analysis_overrides()`

### 面向的后续 benchmark

- SWE-bench
- NewtonBench
- NL2Repo
- Terminal-Bench
- RepoBench
- 内部 synthetic tasks

## Phase 10：实验与验收

### 第一阶段实验矩阵

- `gpt-5.3-codex × pragmatic`
- `gpt-5.3-codex × friendly`
- `gpt-5.4 × pragmatic`
- `gpt-5.4 × friendly`

### 第一阶段主 benchmark

- SWE-bench Verified

### 第一阶段实验顺序

1. 同题 `2x2` 单题验证
2. `3-5` 题 paired pilot
3. `10-15` 题 paired batch

### 第一阶段必须能回答的问题

- 5.4 是否更会“说”
- 5.4 多说的主要是哪几类内容
- 多说与工具使用是正耦合、负耦合还是无关
- `friendly` 和 `pragmatic` 的差异是 tone 级还是 policy 级
- 哪些差异更像模型因素，哪些更像 harness 中介

## 文档同步要求

后续每个 phase 都必须同步更新文档，至少包括：

- `README.md`
- `docs/getting-started.md`
- `docs/probes/probe-taxonomy.md`
- `docs/artifacts/artifact-contract.md`

必要时新增：

- `docs/research/model-personality-study.md`
- `docs/research/codex-observability-contract.md`

## 当前状态

当前执行进度：

- [x] Phase 1：Codex 可观测面契约
- [x] Phase 2：schema 与 artifact contract 重构
- [x] Phase 3：连续输出线与自动 report / datasets 刷新
- [x] Phase 4：Codex 具体工具画像
- [x] Phase 5：message-level NLP 与语言画像
- [x] Phase 6：language × tool coupling 分析
- [x] Phase 7：personality / instruction / skill 机制分析
- [x] Phase 8：task-class-aware 分析
- [x] Phase 9：benchmark 研究画像扩展接口
- [x] Phase 10：多轮测试与验收

已落地的关键文件：

- [Codex 可观测面契约](/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/codex-observability-contract.md)
- [Codex observability map](/Users/kevinlin/Downloads/CodexPlusClaw/studies/observability/codex-observability-map.json)
- [模型人格化研究说明](/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/model-personality-study.md)
- [产物契约](/Users/kevinlin/Downloads/CodexPlusClaw/docs/artifacts/artifact-contract.md)

本计划当前状态：

- 进入维护与扩展阶段
- 后续新增 probe / benchmark / report 时，必须继续遵守 observability contract
- 任何新增字段都必须标注 `observed / inferred / estimated / unobservable`
