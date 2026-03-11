# Probe 分类体系

## 目标

probe 体系的目的，是生成关于 **Codex 如何工作** 的证据，而不是只回答它有没有解出一道题。

整个设计分成两层：

- 在研究模式下由 Codex runtime 内部发出的 raw probe
- 在 run 结束后由本地 artifact 派生出的 derived probe

## Codex 内部 Raw Probe 家族

这些 probe 只会在研究模式下、由 vendored Codex 内部发出。

### Config Freeze

例子：

- 请求态 runtime 与实际生效 runtime 的差异
- config precedence 最终谁赢了
- 模型原生指令是否被保留或替换
- 已运行 thread 上的 override suppression
- effective session config 在什么时间点被真正固定

### Instruction Channels

例子：

- base / developer / model-native / reconstructed context 的相对来源
- ambient skill 或 session 输入是否被压制或泄漏
- compaction 或 resume 后 instruction makeup 是否发生变化

### Turn Lifecycle

例子：

- session spawn
- turn start / end
- active-turn registration / cleanup
- timeout / interruption / failover 边界

### Context And Compaction

例子：

- compaction 触发时的 token 水位
- 压缩前后 history 大小
- compaction 模式与原因
- reconstruction 后 reinjected 的状态
- compaction 之后是否出现 rediscovery

### Tool Mediation

例子：

- shell / patch / MCP 的路由路径
- approval 与 sandbox 决策路径
- 结构化结果传播还是 raw 风格传播
- tool begin / end 边界

### Persistence And Reconstruction

例子：

- rollout recording 模式
- state DB 使用情况
- resume / fork / rebuild 路径
- listener attach 行为

### Harness Friction

例子：

- state DB contention
- listener attach failure
- rollout writer failure
- 会实质性影响 run 的 runtime mismatch warning

## 外层 Derived Probe 家族

这些 probe 由外层 bench 从 raw artifact 计算而来。

## 与论文方向一致的 Probe 家族

### Activation Threshold

- 到第一次有意义 edit 的 token / 时间
- 到第一次 verification 的 token / 时间
- 到第一次 retained patch 的 token / 时间
- 到最终 patch 的 token / 时间

### Redundancy

- 没有 edit 介入的重复读取
- 没有代码变化时的重复 verification
- 重复的 git inspection
- post-submit 活动
- 纯 cleanup 工作

### Context Pressure

- prompt 增长
- cache-read 比例
- compaction 次数与间隔
- history 增长斜率
- compaction 后 rediscovery

### Verification Structure

- edit-to-verification closure
- edit 之后 verification 状态是否发生变化
- verification retry loop
- 在外部强验证约束下进行的工作占比

### Useful Work vs Friction

- useful-step proxy
- useful-token proxy
- friction-token proxy
- retained-edit ratio
- reverted-work ratio

## Codex 原生 Probe 家族

这些 probe 最可能产出真正 **Codex 特有** 的结论。

### Fission / Ignition

- 第一次真正 retained work 出现的位置
- 从 prompt 提交到第一次受控代码变更的时间 / token
- ignition 的触发方式：shell search、patch apply 还是 tool-mediated edit

### Chain Reaction

- `edit -> verify -> edit -> verify` 的传播深度
- 终止前有多少个生产性 cycle
- run 是进入自维持状态，还是很快停滞

### Control Rod

- compaction 是否充当调节器
- config freeze 是否充当调节器
- persistence / resume 是否充当调节器
- approval / listener 边界是否充当调节器

### Containment

- 状态漂移
- 一致性破裂
- effort 是否泄漏到 orchestration overhead
- harness 是否在不产生任务进展的情况下吸走大量预算

### Instruction Stratification

- Codex 是否更像 layered state，而不是纯扁平 transcript
- 模型原生指令何时占主导
- reconstructed context 或 developer instructions 何时占主导

### Tool Mediation Tax

- orchestration 在哪里带来了帮助
- 在哪里引入了延迟或重复劳动
- tool routing 在哪里偏离了“直接 shell 执行”的朴素预期

### Persistence Half-Life

- 有用状态在 compaction / reconstruction 之后还能存活多久
- “记住的状态”在什么时候衰变成 rediscovery

### Event-Architecture Discontinuity

- typed event、legacy notification 和 probe stream 之间的可见性差异
- listener 或 translation 导致的观察盲区

### Externalized Coordination

- Codex 是否能跨 regulation layer 保持并重用状态
- Codex 是否更像 layered coordination 系统，而不是一个单纯消费平面上下文的 transcript 机器

## 新增的人类友好遥测

现在这个 bench 会额外生成几类更适合人类直接阅读的 attempt 产物：

- `turn-metrics.jsonl`
  - 每个 turn 的 token 增量
  - 每个 turn 的 command / tool / skill 计数
- `message-metrics.jsonl`
  - 每条用户可见 assistant 输出的长度、类别、桥接语言、验证语言、social tone 信号
- `verbosity-tool-coupling.jsonl`
  - 每个 tool burst 前的可见输出量
  - `talk_then_act` / `silent_tool_burst` / `micro_narrated_tool_burst` 等模式
- `skill-events.jsonl`
  - 显式和推断出的 skill 使用事件
- `attempt-log.txt`
  - 将 lifecycle、command、tool、skill 与 anomaly 串成一条线性时间日志

它们与以下文件配合使用：

- `run-evidence.txt`
- `report.txt`
- `model-comparison.md`
- `verbosity-analysis.md`
- `tool-language-coupling.md`

## “说更多”研究专用 Probe

为了支撑 `gpt-5.4` vs `gpt-5.3-codex`、`friendly` vs `pragmatic` 的行为对比，这个 bench 现在额外关注：

### 用户可见输出粒度

- `visible_output_total_tokens_est`
- `visible_output_per_turn`
- `visible_output_per_tool_call`
- `visible_output_per_patch_event`
- `visible_output_per_verification_event`

### message-level 分类

- `orientation`
- `task_restatement`
- `planning`
- `observation`
- `decision_explanation`
- `tool_bridge_before`
- `tool_bridge_after`
- `verification_framing`
- `result_framing`
- `social_tone`
- `redundant_recap`

### 语言有效性代理指标

- `actionable_commentary_ratio`
- `tool_grounded_commentary_ratio`
- `verification_grounded_commentary_ratio`
- `restatement_ratio`
- `redundant_commentary_ratio`
- `speculation_ratio`
- `social_tone_ratio`

### 语言 × 工具耦合

- `tokens_before_first_tool`
- `visible_text_before_first_tool`
- `visible_text_between_tool_calls`
- `tool_burst_count`
- `silent_tool_burst_count`
- `micro_narrated_tool_burst_count`

### personality / model 比较视角

这些 probe 不是只为单 run 设计的，而是服务于：

- `gpt-5.3-codex × pragmatic`
- `gpt-5.3-codex × friendly`
- `gpt-5.4 × pragmatic`
- `gpt-5.4 × friendly`

的配对样本比较。

## Classification 标签

每条 derived row 都应该声明以下之一：

- `exact`
- `inferred`
- `estimated`

## Claim Evidence 标签

claim 的评分标签必须保持 evidentiary 风格：

- `evidence_consistent`
- `evidence_mixed`
- `evidence_inconclusive`
- `evidence_against`
- `not_observable_with_current_probes`

bench 不应当模糊原始观测与解释性判断之间的边界。
