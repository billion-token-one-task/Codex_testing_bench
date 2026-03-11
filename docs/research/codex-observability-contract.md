# Codex 可观测面契约

这份文档定义本仓库在研究 Codex 行为时的一个硬约束：

> 只有先明确 `Codex` 真正暴露了什么，我们才允许在 bench、报告、CSV 和论文证据里使用这些字段。

它的目的不是描述“我们希望看到什么”，而是描述：

- vendored Codex **原生直接暴露**了什么
- bench 可以从这些原生信号中 **稳定推导** 出什么
- 哪些结论只能算 **弱推断**
- 哪些东西 **当前不可观测**

后续所有 schema、probe、`report.txt`、Markdown 专题和 CSV 数据集，都应以这份契约为准。

## 观测分层

本仓库统一使用四类观测级别：

- `observed`
  - 直接来自 Codex 原生协议、App Server notification 或原生 raw probe
- `inferred`
  - 由 bench 从多个原生信号稳定推导得到
- `estimated`
  - 需要额外近似、估算或启发式打分
- `unobservable`
  - 当前 runtime 没有暴露，不能写成结论性字段

后续每个 artifact 字段都应当能回答：

- 它来自哪一层
- 它依赖哪些 source refs
- 如果不是 `observed`，它的推导规则是什么

## 证据优先级

当以下几类信息发生冲突时，优先级应当是：

1. 本地真实 run artifact
2. 本地 vendored Codex 源码
3. 外部架构参考资料

这意味着：

- DeepWiki 和 OpenAI 官方文章只用于帮助理解 seam 和设计 probe
- 真正写进报告和论文里的行为结论，必须能回到本地 artifact

## 主要源码 seam

本契约当前主要依据以下本地 seam：

- 协议层
  - [protocol.rs](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex/codex-rs/protocol/src/protocol.rs)
- App Server v2 接口层
  - [v2.rs](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex/codex-rs/app-server-protocol/src/protocol/v2.rs)
- App Server 请求处理
  - [codex_message_processor.rs](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex/codex-rs/app-server/src/codex_message_processor.rs)
- 外层 bench runtime 集成
  - [runtime.rs](/Users/kevinlin/Downloads/CodexPlusClaw/bench/crates/codex-bench-codex/src/runtime.rs)
- 派生 probe 逻辑
  - [derive.rs](/Users/kevinlin/Downloads/CodexPlusClaw/bench/crates/codex-bench-probes/src/derive.rs)

## A. 原生直接可观测

这一层是 bench 的 ground truth。

### 1. turn 生命周期

可以直接观测：

- `TurnStarted`
- `TurnComplete`
- `TurnAborted`

直接可拿字段包括：

- `turn_id`
- `model_context_window`
- `collaboration_mode_kind`
- `last_agent_message`

### 2. token 使用

可以直接观测：

- `TokenCount`

直接可拿字段包括：

- `total_token_usage.input_tokens`
- `total_token_usage.cached_input_tokens`
- `total_token_usage.output_tokens`
- `total_token_usage.reasoning_output_tokens`
- `total_token_usage.total_tokens`
- `last_token_usage.*`
- `model_context_window`
- `rate_limits`

因此：

- “session 累积 token”
- “最近一次 token delta”

都属于 `observed`。

### 3. 用户可见与 reasoning 输出

可以直接观测：

- `AgentMessage`
- `AgentMessageDelta`
- `AgentReasoning`
- `AgentReasoningDelta`
- `AgentReasoningRawContent`
- `AgentReasoningRawContentDelta`
- `AgentReasoningSectionBreak`

注意：

- `AgentMessage*` 属于用户可见输出层
- `AgentReasoning*` / `AgentReasoningRawContent*` 是否进入正式研究使用，应额外受实验 policy 约束

### 4. shell / exec 工具

可以直接观测：

- `ExecCommandBegin`
- `ExecCommandOutputDelta`
- `TerminalInteraction`
- `ExecCommandEnd`

直接可拿字段包括：

- `call_id`
- `process_id`
- `turn_id`
- `command`
- `cwd`
- `parsed_cmd`
- `source`
- `interaction_input`
- `stdout`
- `stderr`
- `aggregated_output`
- `exit_code`
- `duration`
- `formatted_output`
- `status`

### 5. MCP 工具

可以直接观测：

- `McpToolCallBegin`
- `McpToolCallEnd`

直接可拿字段包括：

- `call_id`
- `invocation.server`
- `invocation.tool`
- `invocation.arguments`
- `duration`
- `result`

### 6. patch apply

可以直接观测：

- `PatchApplyBegin`
- `PatchApplyEnd`

直接可拿字段包括：

- `call_id`
- `turn_id`
- `auto_approved`
- `changes`
- `stdout`
- `stderr`
- `success`
- `status`

### 7. 其他工具 / 动作事件

可直接观测：

- `DynamicToolCallRequest`
- `DynamicToolCallResponse`
- `ViewImageToolCall`
- `WebSearchBegin`
- `WebSearchEnd`
- `ImageGenerationBegin`
- `ImageGenerationEnd`

### 8. config / personality 请求边界

在 App Server 层可以直接观测：

- `thread/start` / `turn/start` 请求里的：
  - `model`
  - `model_provider`
  - `service_tier`
  - `cwd`
  - `approval_policy`
  - `sandbox`
  - `base_instructions`
  - `developer_instructions`
  - `dynamic_tools`
  - `experimental_raw_events`
  - `personality`
  - `persist_extended_history`
  - `study_metadata`

并且在运行中可以直接观测：

- requested vs active config mismatch details

其中包括：

- model mismatch
- provider mismatch
- service tier mismatch
- cwd mismatch
- approval mismatch
- sandbox mismatch
- personality mismatch
- ignored override 提示

### 9. compaction / rollback / reroute / stream error

可直接观测：

- `ContextCompacted`
- `ThreadRolledBack`
- `ModelReroute`
- `StreamError`
- `Warning`
- `Error`

### 10. collaboration / sub-agent

可直接观测：

- `CollabAgentSpawnBegin/End`
- `CollabAgentInteractionBegin/End`
- `CollabWaitingBegin/End`
- `CollabCloseBegin/End`
- `CollabResumeBegin/End`

### 11. study 原生 probe

可直接观测：

- `StudyMetadata`
- `StudyProbeEvent`

当前 raw probe 分 subsystem：

- `ConfigFreeze`
- `InstructionChannel`
- `TurnLifecycle`
- `ContextCompaction`
- `ToolMediation`
- `PersistenceReconstruction`
- `HarnessFriction`
- `ArchitectureMap`

## B. 稳定可推导

这些字段不属于原生协议直接提供，但可以由 bench 通过本地 artifact 稳定重建。

### 1. 每 turn token 变化

来源：

- `TurnStarted/Complete`
- `TokenCount.last_token_usage`
- `TokenCount.total_token_usage`

可稳定推导：

- `turn_input_tokens`
- `turn_output_tokens`
- `turn_cached_input_tokens`
- `turn_total_tokens`

### 2. 用户可见 message 级分析

来源：

- `AgentMessage`
- `AgentMessageDelta`

可稳定推导：

- 句子数
- 段落数
- bullet 数
- code block 数
- 粗粒度 discourse category
- 可见文本总量

### 3. 语言 × 工具时序耦合

来源：

- message timeline
- exec / MCP / patch timeline

可稳定推导：

- `tokens_before_first_tool`
- `visible_text_before_first_tool`
- `visible_text_between_tool_calls`
- `commentary_to_tool_delay`
- `tool_result_to_commentary_delay`
- `tool_burst_count`
- `silent_tool_burst_count`
- `micro_narrated_tool_burst_count`

### 4. patch / verification 关联

来源：

- `PatchApply*`
- command timeline
- message timeline

可稳定推导：

- tool 是否后接 patch
- tool 是否后接 verification
- commentary 是否桥接 patch / verification

## C. 弱推断

这些字段可以研究，但必须显式标注 `inferred`，不能当作原生事实。

### 1. skill 实际使用

当前原生协议里可以直接列出：

- skill catalog
- skill metadata

但当前没有统一的 `SkillUsedEvent`。

因此：

- “有哪些 skill 可用”是 `observed`
- “这次 run 里某个 skill 被实际使用了”当前通常是 `inferred`

### 2. instruction layering shift

我们能观察到：

- requested personality
- developer/base instructions 请求
- study probe 中部分 instruction channel 事件

但“每一时刻模型内部实际用哪一层为主”并不是完全直接暴露的。

因此：

- `instruction_channel_count` 可以研究
- 更强的“某层主导了当前策略”多半属于 `inferred`

### 3. harness overhead tax

像：

- `language_overhead_tax`
- `tool_mediation_tax`
- `harness_overhead_tax`
- `bootstrap_env_tax`

都属于分析层概念，不是原生事件。

### 4. persistence continuity / stale-state

可以研究，但应明确是基于：

- resume / compaction / replacement history
- rediscovery pattern

的 `inferred` 结论。

## D. 当前不可观测

这一层不能写成结论性字段。

### 1. 完整 hidden CoT

即使有 reasoning 相关事件，也不等于我们拿到了模型完整的、稳定定义的内部推理。

### 2. 所有内部 routing branch

我们可以看到 shell / MCP / patch 等显式事件，但不能默认看到了每个内部 orchestration decision。

### 3. 每次 skill 调用的原生显式事件

在当前 vendored Codex 中，这一层暂时没有统一稳定的原生事件。

## 研究实现约束

后续 bench 增强必须遵守：

1. 新字段先问“是 observed 还是 inferred”
2. `report.txt` 中不得把 `inferred` 写成“Codex 直接表明”
3. 所有 CSV 里最好保留：
   - `classification`
   - `source_refs`
4. 凡是 tool / personality / skill / instruction 层的研究结论，都应当回指至少一个 raw artifact

## 与当前 roadmap 的关系

这份契约是后续几项工作的前置条件：

- 自动报告流水线
- 更细 tool inventory
- 更细 message NLP
- personality mechanism analysis
- 2x2 cohort paired comparison

没有这份契约，就不应该继续盲目扩 schema。

## 外部参考

用于高层架构理解，不用于替代本地源码和 artifact：

- [DeepWiki Codex](https://deepwiki.com/openai/codex)
- [OpenAI: Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness/)
- [OpenAI: Introducing upgrades to Codex](https://openai.com/index/introducing-upgrades-to-codex/)
- [OpenAI: Introducing the Codex app](https://openai.com/index/introducing-the-codex-app/)

