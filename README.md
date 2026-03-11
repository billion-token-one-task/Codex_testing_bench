# Codex Testing Bench

Codex Testing Bench 是一个 **仅围绕 Codex 本体** 构建的研究型测试台，用来研究 vendored Codex 运行时在真实任务和基准上的行为方式。

整个仓库围绕一个核心原则设计：

- 研究工作台应当能直接从 GitHub 根目录被读懂
- vendored 的 Codex 树应保持为一个固定版本的运行时目标，只承载很薄的一层研究探针补丁
- 本地产物应当足够丰富，能够支撑严肃的内部研究，而不依赖仪表盘或外部遥测系统

## 这个仓库是做什么的

这个仓库的目标不是单纯跑分，而是对 Codex 作为 agentic harness 做深入经验研究。

这个 bench 试图回答的问题包括：

- Codex 究竟如何在 `thread` 启动时冻结 runtime 和 session 状态？
- Codex 如何在模型原生指令、开发者指令、重建上下文和运行时更新之间组装最终指令？
- Codex 的 compaction 机制到底保留了什么、压缩了什么、遗忘了什么？
- 一个任务中的“工作量”到底有多少来自模型本身，有多少来自 Codex 自己的编排机制？
- Codex 什么时候会进入高产出的 edit / verify 循环，什么时候会把预算消耗在 harness friction 上？
- 我们关于 token 预算和长程调度的论文里提出的方向性观点，哪些能在 Codex 行为中被具体观察到？

## 当前核心实验

当前这套 bench 的主研究线，不是“哪个模型分数更高”，而是：

- `gpt-5.4` 与 `gpt-5.3-codex` 的行为方式是否存在系统性差异
- `friendly` 与 `pragmatic` personality 是否只是表层语气变化，还是会改变 agent policy
- 模型“说更多”到底表现为哪些可观察输出
- “说更多”与工具调用、验证、patch 生成、harness 调节机制之间是如何耦合的

第一阶段的标准实验矩阵是一个固定的 `2x2`：

- `gpt-5.3-codex × pragmatic`
- `gpt-5.3-codex × friendly`
- `gpt-5.4 × pragmatic`
- `gpt-5.4 × friendly`

并且默认要求：

- 四个 cohort 使用同一批任务样本
- 默认按同一 `instance_id` 做配对比较
- prompt、tool policy、web-search policy、sandbox policy、grading policy 冻结
- 先看行为证据，再看 benchmark 评分

这意味着我们不是在做泛泛的“模型体验对比”，而是在做：

> **同一 Codex harness 中，模型代际差异与 personality 条件如何改变 agent 的可观察状态外显化、工具耦合模式、验证习惯与任务推进节奏。**

## 我们当前最关心的研究假设

现阶段最核心的假设不是某个 benchmark 能否多解几题，而是下面这些更接近架构研究的问题：

- `H1`：`gpt-5.4` 的用户可见输出总体上多于 `gpt-5.3-codex`
- `H2`：`gpt-5.4` 多出来的输出，更多是任务桥接语、验证 framing、决策解释，而不是纯礼貌包装
- `H3`：`gpt-5.4` 的“说更多”与工具调用之间存在更强耦合，而不是单纯的 verbosity
- `H4`：`friendly` 会放大这种可观察状态外显化，`pragmatic` 则更接近压缩后的执行风格
- `H5`：这些差异并不纯粹来自 base model，而是和 Codex 的 instruction layering、tool mediation、config freeze、compaction/reconstruction 等 harness 机制共同作用的结果

因此，这个仓库的研究对象始终是：

- **模型本身**
- **harness 机制**
- **两者的交互效应**

而不是把脚手架和模型割裂开来单独研究。

## 这套 bench 为什么适合做这种研究

这套系统不是只抓最终 patch，而是把一次运行拆成多个可分析层：

- 用户可见输出层
- 工具调用层
- 验证与 patch 层
- Codex 内部 probe 层
- campaign 级比较与 claim evidence 层

因此我们不仅能回答：

- 哪个模型更爱“说”
- 哪个 personality 更简洁

还能回答：

- 它到底说了什么
- 这些话是在工具前、工具后，还是验证前后出现
- 哪些话与实际任务推进强相关，哪些更像热量
- 哪些差异更像 harness 的 regulation / control-rod 效应
- 哪些差异只在某一类任务下出现，例如 bootstrap-heavy、verification-heavy、compaction-likely

这个仓库的主要输出不是仪表盘，也不是最终论文，而是一组证据包：

- campaign 级别的 `report.txt`
- campaign 级别的 `model-comparison.md`
- campaign 级别的 `verbosity-analysis.md`
- campaign 级别的 `tool-language-coupling.md`
- campaign 级别的 `linguistic-profile.md`
- campaign 级别的 `phrase-and-tone-analysis.md`
- campaign 级别的 `bridge-language-analysis.md`
- campaign 级别的 `tool-inventory.md`
- campaign 级别的 `tool-route-analysis.md`
- campaign 级别的 `benchmark-research-profile.json`
- campaign 级别的 `personality-analysis.md`
- campaign 级别的 `task-class-analysis.md`
- 每个运行的 `run-evidence.txt`
- 每个运行的 `attempt-log.txt`
- `datasets/*.csv` 形式的研究数据集
- 原始与归一化后的本地遥测产物

其中新增的研究型数据集现在包括：

- `message_style.csv`
- `cohort_lexical_summary.csv`
- `model_phrase_deltas.csv`
- `personality_phrase_deltas.csv`
- `tool_inventory.csv`
- `tool_route_summary.csv`
- `tool_by_turn.csv`

这轮之后，语言分析也明确对齐到当前实验 vision，而不再只是做泛泛的 verbosity 统计。现在重点追踪的是：

- `state externalization`
  - 模型是否把中间状态、局部目标、下一步动作外显出来
- `bridge language`
  - 工具调用前后的桥接语言是否上升
- `verification framing`
  - 模型是否更愿意解释验证目标、验证结果与信心状态
- `personality mechanism`
  - `friendly` / `pragmatic` 的差异到底是社交包装，还是会改变工具耦合与任务推进节奏

对应地，`message_style.csv` / `linguistic-profile.md` / `tool-language-coupling.md` 现在会显式输出：

- `bridge_language_score_bps`
- `verification_language_score_bps`
- `state_externalization_score_bps`
- 词汇多样性、hapax ratio、最常见 lemma / bigram / trigram
- 第一人称 / 第二人称 / modal / sequencing cue
- artifact / code reference 密度
- 具体工具调用前后的 commentary 量与 route 差异

## 先看什么信息是 Codex 真的给出来的

这套 bench 当前已经明确采用一个新的研究约束：

> **先建立 Codex 可观测面契约，再扩 probes、报告和 CSV。**

原因很简单：

- 有些信息是 Codex 原生协议直接给出的
- 有些只能由 bench 稳定推导
- 有些只能弱推断
- 还有一些当前根本不可观测

如果不先把这几层分开，后面的研究很容易把 `inferred` 写成 `observed`。

当前这部分正式文档在：

- [docs/research/codex-observability-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/research/codex-observability-contract.md)
- [studies/observability/codex-observability-map.json](/Users/kevinlin/Downloads/CodexPlusClaw/studies/observability/codex-observability-map.json)

它们规定了：

- 哪些 turn / token / tool / patch / personality / compaction / collaboration 信息属于原生可观测
- 哪些字段只能稳定推导
- 哪些字段只能弱推断
- 哪些东西当前不应该写成研究结论

后续的 probe、`report.txt`、Markdown 专题、CSV 数据集都会以这份契约为准。

这些输出的目标是：

- 让人可以直接在 GitHub 上读懂一次实验
- 让后续论文写作可以直接复用 Markdown 结论与 CSV 数据表
- 让我们能够区分“模型行为证据”和“benchmark/harness 环境失败”

## 仓库结构

- [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench)：当前激活的外层研究 bench 工作区
- [repos/codex](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex)：vendored 的 Codex 运行时，仅保留轻量级、研究模式下启用的 probe 补丁
- [vendor-benchmarks](/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks)：vendored 的外部 benchmark 资源
- [studies](/Users/kevinlin/Downloads/CodexPlusClaw/studies)：可复用的 claim catalog、preset 与 benchmark catalog 元数据
- [docs](/Users/kevinlin/Downloads/CodexPlusClaw/docs)：架构、参考资料、probe 分类、artifact 契约和扩展文档
- [artifacts](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts)：GitHub 可见的 campaign 输出和整理后的研究证据

## Bench 工作区

活跃的 crate 位于 [bench/crates](/Users/kevinlin/Downloads/CodexPlusClaw/bench/crates)：

- `codex-bench-core`
  - manifest、artifact、IO 辅助、trait 与共享类型
- `codex-bench-codex`
  - 直接集成 Codex App Server 的进程内运行路径
  - 原始事件捕获
  - 架构映射生成
  - benchmark 运行时“禁止 web search”的强约束
- `codex-bench-swebench`
  - SWE-bench Verified 适配器
  - 通用 `repo-patch-jsonl` 适配路径
- `codex-bench-nl2repo`
  - NL2RepoBench 适配器
- `codex-bench-newtonbench`
  - NewtonBench 适配器
- `codex-bench-probes`
  - 从原始数据到派生 probe 的逻辑
  - claim evidence 推导
- `codex-bench-report`
  - `report.txt`
  - `run-evidence.txt`
  - `attempt-log.txt`
  - replay 文本输出
- `codex-bench-cli`
  - `prepare` / `run` / `grade` / `report` / `replay` / `inspect-codex`

## 当前支持的基准

目前已经有一等支持的本地 adapter：

- SWE-bench Verified
- NL2RepoBench
- NewtonBench

同时，这个 bench 的设计也支持向更多基准扩展：

- `repo-patch-jsonl` 是 repo-based patch 类任务的通用桥接层
- preset 和 adapter 的边界设计，就是为了以后加入新的 benchmark 家族时不需要改动 Codex shim

换句话说，SWE-bench 只是 v1 主战场，而不是这套系统的边界。
这套架构是为了让后续的：

- Terminal-Bench
- RepoBench
- NL2Repo
- NewtonBench
- 内部 synthetic long-context tasks

都能共享同一套 Codex probe、report、CSV 数据导出和 claim evidence 推导机制。

参考：

- [docs/references/benchmarks.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/benchmarks.md)
- [studies/benchmarks/2026-benchmark-catalog.json](/Users/kevinlin/Downloads/CodexPlusClaw/studies/benchmarks/2026-benchmark-catalog.json)

## 它会测量什么

这个 bench 会抓取四层证据：

1. 原始 runtime 流
2. Codex 内部研究探针
3. 派生出的行为 probe
4. 人类可读的 evidence report

已覆盖的观测信号包括：

- token 输入 / 输出 / cache-read 随时间的变化
- 每个 turn 的 token 增量
- 每条用户可见 assistant 输出的 `message-metrics`
- 每个 cohort 的词汇画像、短语画像与 discourse category 聚合
- `verbosity-tool-coupling` 里的语言-工具时序耦合
- 命令时间线
- 工具调用时间线
- 具体工具名、route、成功状态、duration、输出体积与前置 commentary
- `apply_patch` 活动
- skill 使用事件
- compaction 和 reconstruction 证据
- instruction channel 转移
- config-freeze drift
- persistence / resume 效应
- harness friction 事件
- 与本地证据强绑定的 claim evidence label
- `friendly` / `pragmatic` personality 是否真正生效
- `gpt-5.4` 与 `gpt-5.3-codex` 的配对行为差异

参考 [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)。

## 快速开始

以下命令都从 [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench) 目录执行。

### 1. 准备一个 campaign

```bash
cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../artifacts \
  --preset-path ../studies/task-presets/swebench-v1.json \
  --stage architecture-validation \
  --seed codex-study
```

默认的 `swebench-v1` preset 会展开为一个第一阶段 2×2 对比矩阵：

- `gpt-5.3-codex × pragmatic`
- `gpt-5.3-codex × friendly`
- `gpt-5.4 × pragmatic`
- `gpt-5.4 × friendly`

如果你只是想做一次最小但真实的“同题四格”实验，那么 `architecture-validation` 就是最适合的入口：

- 它只抽取 `1` 道题
- 但会把这道题扩成四个 cohort
- 非常适合快速观察 `5.3-codex` / `5.4` 在 `friendly` / `pragmatic` 下的语言与工具行为差异

如果要显式覆写单个 cohort，可以额外传：

- `--model`
- `--provider`
- `--personality`
- `--prompt-style`
- `--experiment-name`

### 2. 预热本地资源与共享缓存

```bash
cargo run -p codex-bench-cli -- bootstrap-local \
  --campaign-dir ../artifacts/<campaign-id>
```

### 3. 运行 Codex

```bash
cargo run -p codex-bench-cli -- run ../artifacts/<campaign-id>
```

`run` 完成后会自动生成：

- `reports/report.txt`
- `reports/model-comparison.md`
- `reports/verbosity-analysis.md`
- `reports/tool-language-coupling.md`
- `reports/linguistic-profile.md`
- `reports/phrase-and-tone-analysis.md`
- `reports/bridge-language-analysis.md`
- `reports/tool-inventory.md`
- `reports/tool-route-analysis.md`
- `reports/personality-mechanism-analysis.md`
- `reports/patch-mechanism-analysis.md`
- `reports/skill-mechanism-analysis.md`
- `datasets/*.csv`
- 以及机制专题数据集：
  - `personality_mechanism.csv`
  - `patch_chain.csv`
  - `skill_mechanism.csv`

### 4. 打分并生成 evidence dossier

```bash
cargo run -p codex-bench-cli -- grade ../artifacts/<campaign-id> \
  --command 'python -m swebench.harness.run_evaluation --predictions_path {predictions}'

cargo run -p codex-bench-cli -- report ../artifacts/<campaign-id>
```

`grade` 完成后会自动把评分结果 ingest 回 campaign / run manifest，并自动刷新上述 Markdown 与 CSV。`report` 主要用于重建与回填。

### 5. 查看结果

- campaign 报告：`artifacts/<campaign-id>/reports/report.txt`
- 模型对比：`artifacts/<campaign-id>/reports/model-comparison.md`
- verbosity 专题：`artifacts/<campaign-id>/reports/verbosity-analysis.md`
- 语言-工具耦合专题：`artifacts/<campaign-id>/reports/tool-language-coupling.md`
- 语言画像专题：`artifacts/<campaign-id>/reports/linguistic-profile.md`
- 工具清单专题：`artifacts/<campaign-id>/reports/tool-inventory.md`
- personality 机制专题：`artifacts/<campaign-id>/reports/personality-mechanism-analysis.md`
- patch 机制专题：`artifacts/<campaign-id>/reports/patch-mechanism-analysis.md`
- skill 机制专题：`artifacts/<campaign-id>/reports/skill-mechanism-analysis.md`
- 单题证据：`artifacts/<campaign-id>/runs/<instance>/attempt-01/run-evidence.txt`
- 单题线性日志：`artifacts/<campaign-id>/runs/<instance>/attempt-01/attempt-log.txt`
- CSV 数据集：`artifacts/<campaign-id>/datasets/*.csv`

更完整的上手说明见 [docs/getting-started.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/getting-started.md)。

## 评测策略

benchmark 运行时被有意地施加了若干约束，以减少污染并让解释更干净。

当前重要策略包括：

- `run` 默认走有界并行
  - 当前 preset 默认 `max_parallel_runs=2`
  - 同一 repo 的 workspace/cache 物化阶段会按 repo 再限流
- 继续复用本地 dataset snapshot、repo cache、grader venv 与 Rust 编译产物
- `grade` 失败时仍会自动刷新报告，并把失败归为 benchmark/harness，而不是直接归咎于模型行为

- benchmark run 只使用本地路径
- benchmark run 不经过 OpenClaw
- 被评测的 Codex runtime 明确禁用了 web search
- 如果 Codex 仍然发出了 web-search 事件，则该次 benchmark run 立即判为失败

这样做是为了让证据尽可能集中在 **repo 本地运行时里的 Codex harness 行为** 上。

## GitHub 可见产物 vs 本地重型产物

这个仓库会把 GitHub 可见的研究证据和机器本地的大型中间产物分开。

GitHub 可见：

- campaign manifest
- 选中的数据集快照
- architecture map
- claim catalog
- hypothesis catalog
- `report.txt`
- 研究专题 Markdown
- `run-evidence.txt`
- `attempt-log.txt`
- CSV 数据集
- 摘要型 JSON 产物

仅本地保留：

- 预热后的 repo cache
- worktree 与重量级 workspace
- 全量 raw JSONL 流（除非刻意整理后提交）
- 体积很大的临时文件

详见 [artifacts/README.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/README.md) 和 [docs/artifacts/artifact-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/artifacts/artifact-contract.md)。

## 为什么要 vendored Codex

这个 bench 依赖对 Codex 内部行为的深度观测：

- session/config freeze
- instruction assembly
- compaction 与 reconstruction
- event/listener translation
- tool mediation
- persistence/resume 行为

这些信号只有在 runtime 被固定版本并且可以本地打补丁时才真正可见。

外层 bench 负责 orchestration、reporting 和 extensibility。
vendored Codex 只负责 runtime 行为与研究模式下的轻量 probe 发射。

详见 [docs/architecture/bench-architecture.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)。

## 核心参考资料

- [docs/references/codex.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/codex.md)
- [docs/references/benchmarks.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/benchmarks.md)
- [DeepWiki Codex](https://deepwiki.com/openai/codex)
- [OpenAI: Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness/)

## 推荐阅读顺序

如果你是第一次进入这个仓库：

1. [README.md](/Users/kevinlin/Downloads/CodexPlusClaw/README.md)
2. [docs/getting-started.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/getting-started.md)
3. [docs/architecture/bench-architecture.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)
4. [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
5. [docs/artifacts/artifact-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/artifacts/artifact-contract.md)

如果你是来理解“我们到底在研究什么”的：

1. [README.md](/Users/kevinlin/Downloads/CodexPlusClaw/README.md)
2. [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
3. [studies/hypotheses/model-behavior-v1.json](/Users/kevinlin/Downloads/CodexPlusClaw/studies/hypotheses/model-behavior-v1.json)
4. 任意一个 campaign 下的 `reports/model-comparison.md`
5. 任意一个 run 下的 `run-evidence.txt`

如果你是来扩展系统的：

1. [docs/architecture/bench-architecture.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)
2. [docs/extending-the-bench.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/extending-the-bench.md)
3. [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
