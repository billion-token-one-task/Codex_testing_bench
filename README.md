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

这个仓库的主要输出不是仪表盘，也不是最终论文，而是一组证据包：

- campaign 级别的 `report.txt`
- campaign 级别的 `model-comparison.md`
- campaign 级别的 `verbosity-analysis.md`
- campaign 级别的 `tool-language-coupling.md`
- 每个运行的 `run-evidence.txt`
- 每个运行的 `attempt-log.txt`
- `datasets/*.csv` 形式的研究数据集
- 原始与归一化后的本地遥测产物

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
- `verbosity-tool-coupling` 里的语言-工具时序耦合
- 命令时间线
- 工具调用时间线
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

### 4. 打分并生成 evidence dossier

```bash
cargo run -p codex-bench-cli -- grade ../artifacts/<campaign-id> \
  --command 'python -m swebench.harness.run_evaluation --predictions_path {predictions}'

cargo run -p codex-bench-cli -- report ../artifacts/<campaign-id>
```

### 5. 查看结果

- campaign 报告：`artifacts/<campaign-id>/reports/report.txt`
- 模型对比：`artifacts/<campaign-id>/reports/model-comparison.md`
- verbosity 专题：`artifacts/<campaign-id>/reports/verbosity-analysis.md`
- 语言-工具耦合专题：`artifacts/<campaign-id>/reports/tool-language-coupling.md`
- 单题证据：`artifacts/<campaign-id>/runs/<instance>/attempt-01/run-evidence.txt`
- 单题线性日志：`artifacts/<campaign-id>/runs/<instance>/attempt-01/attempt-log.txt`
- CSV 数据集：`artifacts/<campaign-id>/datasets/*.csv`

更完整的上手说明见 [docs/getting-started.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/getting-started.md)。

## 评测策略

benchmark 运行时被有意地施加了若干约束，以减少污染并让解释更干净。

当前重要策略包括：

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

如果你是来扩展系统的：

1. [docs/architecture/bench-architecture.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)
2. [docs/extending-the-bench.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/extending-the-bench.md)
3. [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
