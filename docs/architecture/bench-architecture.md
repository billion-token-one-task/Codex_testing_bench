# Bench 架构

## 设计目标

在保留对 vendored Codex runtime 深度观测能力的同时，让研究 bench 本身在仓库根目录层面仍然是清晰、可读、可扩展的。

这个架构有意将两类职责分开：

- 外层 bench 的 orchestration 与 research logic
- vendored Codex 的 runtime 行为

## 高层边界

外层 bench 负责：

- campaign 准备
- benchmark adapter
- workspace materialization
- run orchestration
- raw artifact 收集
- derived probe
- claim catalog
- reporting

vendored Codex 负责：

- App Server
- agent runtime 行为
- session/config freeze
- prompt assembly
- compaction 与 reconstruction
- tool mediation
- persistence 与 resume
- 研究模式下启用的轻量 raw probe 发射

## 为什么要这样分层

如果整套 benchmark 系统都塞在 vendored Codex 内部：

- GitHub 仓库会很难读
- 研究层会与某一个 runtime 实现强耦合

把外层 bench 放在外面之后：

- 仓库更容易理解
- adapter 可以独立增长
- report 与 claim logic 更容易复用
- Codex 补丁层可以保持更薄、更易审计

## 外层工作区

当前活跃工作区位于 [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench)。

### `codex-bench-core`

负责：

- 共享类型
- manifest
- artifact 路径
- JSON / JSONL IO
- adapter 与 renderer trait

它是整个 bench 的稳定层。

### `codex-bench-codex`

负责：

- 直接启动进程内 App Server
- thread start / turn start orchestration
- 被评测 Codex run 的 runtime 配置
- 原始事件捕获
- 原始诊断事件捕获
- architecture map 生成

这是外层 bench 中唯一直接和 vendored Codex crate 交互的模块。

### `codex-bench-swebench`

负责：

- SWE-bench Verified 抽样
- repo-patch 任务归一化
- worktree 设置
- prompt 构造
- patch 提取
- grading 集成

### `codex-bench-nl2repo`

负责：

- NL2Repo 任务发现
- 空白仓库初始化
- benchmark 本地 grading 命令

### `codex-bench-newtonbench`

负责：

- NewtonBench 任务生成
- 本地实验环境搭建
- NewtonBench evaluator 封装

### `codex-bench-probes`

负责：

- raw 到 derived 的 probe 推导
- probe summary
- claim evidence 推导
- 行为计数器与 evidence label

### `codex-bench-report`

负责：

- campaign 级 `report.txt`
- 单 run 级 `run-evidence.txt`
- 单 run 级 `attempt-log.txt`
- replay 文本输出
- 从旧 raw evidence 回填较新派生产物的 reporting 能力

### `codex-bench-cli`

用户入口命令包括：

- `prepare`
- `run`
- `bootstrap-local`
- `warm-cache`
- `grade`
- `report`
- `replay`
- `inspect-codex`
- `list-presets`

## 数据流

### 1. Prepare

`prepare` 会写出：

- `campaign-manifest.json`
- `selected-dataset.json`
- `codex-architecture-map.json`
- campaign 局部的 claim catalog

这一步结束时，campaign 已经定义完成，但还没有真正跑 Codex。

### 2. Run

`run` 的步骤是：

1. materialize 一个 benchmark workspace
2. 启动一个带 study tag 的 Codex thread
3. 捕获原始 App Server notification
4. 捕获原始 Codex study probe event
5. 提取 patch / output
6. 派生 normalized telemetry
7. 写入单 run 摘要和人类可读证据文件

### 3. Grade

`grade` 调用对应 benchmark adapter 的 grading 路径，并把 grader 输出写回 campaign artifact。

### 4. Report

`report` 只读取本地 artifact，并产出：

- campaign 级 `report.txt`
- 刷新的单 run `run-evidence.txt`
- 刷新的单 run `attempt-log.txt`

如果发现当前格式演进了，而旧 campaign 仍有足够 raw artifact，`report` 可以从旧 raw artifact 回填新的派生文件。

## Artifact 哲学

这个 bench 明确保留两层：

- raw truth
- readable evidence

raw truth：

- raw agent events
- raw diagnostics
- raw Codex probe events

readable evidence：

- turn metrics
- tool 和 skill 摘要
- probe summary
- run evidence
- campaign report

正因为这两层分开，仓库才可以同时做到：

- GitHub 上足够清晰
- 研究上仍然有足够科学价值

## 当前评测策略

对于 benchmark evaluation run：

- Codex web search 被禁用
- 一旦仍然出现 web-search 事件，bench 会立即中止该 run
- 研究路径保持 local-only
- active runtime path 是 Codex-only

这些约束是 bench 架构的一部分，而不是仅靠人工约定。
