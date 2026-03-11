# 扩展 Bench

## 目标

让这个仓库可以持续添加新的任务集、新的 probe 家族和新的报告形式，而不会演化成一个难以维护的巨型单体。

## 扩展点

主要扩展接口位于 `codex-bench-core`。

关键边界包括：

- `BenchmarkAdapter`
  - 抽样任务
  - 准备 workspace
  - 构建 prompt / input bundle
  - 提取 patch / output
  - 调用 grader
- `RuntimeAdapter`
  - 启动 session
  - 启动 turn
  - 消费事件流
  - 返回 runtime capture
- `ProbeDeriver`
  - 从 raw artifact 推导结构化证据行
- `ReportRenderer`
  - 写 campaign 级和单 run 级的人类可读报告
- `ClaimCatalog`
  - 加载 claim 集并评估 evidence label

## 什么时候应该新增一个 Benchmark Adapter

如果新的 benchmark 至少满足以下一项，就应当单独做 adapter：

- workspace 的构造方式不同
- grading 方式不同
- 任务对象 schema 不同
- 它代表了一个真正不同的观察场景

典型例子：

- SWE-bench：已有代码库中的 patch 修复与回归验证
- NL2Repo：从自然语言规格开始的零到一仓库构建
- NewtonBench：科学实验与规律发现

## 什么时候复用 `repo-patch-jsonl`

如果某个 benchmark 可以被归一化成以下字段：

- `instance_id`
- `repo`
- `base_commit`
- `problem_statement`
- 可选的 hint / test / metadata

那么优先考虑复用通用的 `repo-patch-jsonl` 这条路径。

这样可以在不急着创建完整 bespoke adapter 的前提下，复用同一套 runtime、probe 和 report 栈。

## 新增 Probe Family 的原则

一个好的 probe 应该：

- 扎根于可观测 artifact
- 带上 `classification`，即 `exact`、`inferred` 或 `estimated`
- 带上 `evidenceCode`
- 指回源 artifact
- 避免把解释性结论和原始观测混在一起

bench 里有两层 probe：

- vendored Codex 内部发出的 raw study probe
- 外层 bench 里从 artifact 派生出的 derived probe

优先原则：

- 只有在必须的时候，才在 vendored Codex 内部新增轻量 raw probe
- 重的解释工作尽量放到 `codex-bench-probes`

## 新增报告类型的原则

当前标准的人类可读输出是：

- `report.txt`
- `run-evidence.txt`
- `attempt-log.txt`

如果你要新增一种报告：

- 保持 deterministic
- 保持 local-only
- 保持 artifact-derived
- 不要依赖外部 dashboard 或 collector

## 文档更新要求

如果你新增了：

- benchmark adapter
- probe family
- 新 artifact 类型
- 新报告形式

那么通常还应同步更新：

- [README.md](/Users/kevinlin/Downloads/CodexPlusClaw/README.md)
- [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
- [docs/artifacts/artifact-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/artifacts/artifact-contract.md)
- [docs/references/benchmarks.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/benchmarks.md)（如果 benchmark 面发生变化）
