# 研究控制台 Web App

## 目标

研究控制台是这套 bench 的本地优先操作与观测面板。它不是一个独立的第二系统，而是直接建立在：

- `artifacts/`
- 现有 bench CLI
- 现有 report / dataset builder
- 现有 run artifact 契约

之上的统一观察和控制层。

## 架构

控制台由两部分组成：

- `bench/crates/codex-bench-control-plane`
  - Rust 本地服务
  - 暴露 HTTP + SSE
  - 管理 bench 进程
  - 索引 `artifacts/`
- `apps/research-console`
  - React + Vite 前端
  - 负责 UI、面板、artifact 浏览、实时刷新

这套控制台不是一个旁路系统，而是直接绑定现有：

- benchmark adapter
- report builder
- dataset builder
- `artifacts/` 目录
- `codex-bench-cli` 动作语义
- Codex 行为研究的 observability contract

也就是说，它展示的不是“另一套缓存后的影子数据”，而是 bench 当前实际在生成和消费的同一份证据面。

## 当前主视图

- `Campaigns`
- `Live`
- `Runs`
- `Compare`
- `Artifacts`
- `Research`

此外，控制台已经有一等的 `Run Detail` 战情页，用来查看单题的完整行为链。

## 页面说明

### Campaigns

面向实验总览：

- 所有 campaign 的 ledger
- benchmark / stage / sample size / cohort 数
- 运行状态、活跃 run 数、累计 token、累计工具调用
- 当前 campaign 的 reports / datasets 入口

### Live

面向正在发生的运行：

- 活跃进程
- 实时 stdout / stderr
- 最近事件
- 并行槽位与当前运行负载
- 可以直接发起 `prepare / run / grade / report / replay / stop`

### Runs

面向 run 级筛选与巡检：

- model / personality / task class / grading status 过滤
- tool / token / visible output / mechanism 摘要
- 直接跳转到某个 run 的 detail 页面

### Compare

面向研究对比：

- `5.4 vs 5.3-codex`
- `friendly vs pragmatic`
- 同题多 cohort 配对比较
- `model_pair_deltas.csv` / `campaign_runs.csv` / `message_style.csv` 驱动的结构化视图

### Artifacts

面向证据包浏览：

- `report.txt`
- Markdown 专题
- `datasets/*.csv`
- 直接预览单个 artifact 内容

### Research

面向 hypothesis / task-class / mechanism 的研究工作台：

- claim evidence
- personality mechanism
- task-class 维度摘要
- 研究型聚合表

### Run Detail

这是最重要的一页，当前已经是完整 war room：

- 运行概览
- 实时状态与最新更新时间
- timeline rail
- 用户可见输出
- tool / route / approval / patch 机制
- personality / instruction / skill 机制
- command ledger
- patch diff
- `run-evidence.txt`
- `attempt-log.txt`
- attempt artifact 列表
- artifact tail 预览

## 当前主操作

- `prepare`
- `bootstrap-local`
- `warm-cache`
- `run`
- `grade`
- `report`
- `replay`
- `inspect-codex`
- `stop`

## 实时能力

当前版本通过 SSE 推送：

- process stdout / stderr
- process 生命周期变化
- workspace index 更新

同时，控制台会通过 HTTP 拉取并刷新：

- campaign list / detail
- run detail
- reports / datasets
- artifact preview
- artifact tail

对于活跃 run，控制台会把：

- process 输出
- run summary 更新
- 新增 tool / patch / message 表
- campaign 聚合变化

组合成一个接近实时的操作面。

## 当前 API 与数据面

目前 control plane 已经稳定暴露：

- `GET /api/workspace/index`
- `GET /api/campaigns`
- `GET /api/campaigns/:id`
- `GET /api/campaigns/:id/reports`
- `GET /api/campaigns/:id/datasets`
- `GET /api/runs/:id`
- `GET /api/runs/:id/detail`
- `GET /api/runs/:id/attempts/:n`
- `GET /api/artifacts/file`
- `GET /api/artifacts/tail`
- `GET /api/events`
- `POST /api/actions/*`

其中 `run detail` 已经会直接聚合：

- `turn-metrics.jsonl`
- `message-metrics.jsonl`
- `tool-events.jsonl`
- `command-events.jsonl`
- `patch-chain.jsonl`
- `patch-events.jsonl`
- `personality-events.jsonl`
- `skill-mechanism.jsonl`
- `verbosity-tool-coupling.jsonl`

以及：

- `run-evidence.txt`
- `attempt-log.txt`
- `patch.diff`

## 运行方式

### 启动 control plane

从 [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench) 目录：

```bash
cargo run -p codex-bench-control-plane -- --repo-root ../
```

默认监听：

- `http://127.0.0.1:4173`

如果你想避免和其他本地服务端口冲突，也可以显式指定，例如：

```bash
cargo run -p codex-bench-control-plane -- --repo-root ../ --port 4273
```

### 启动前端开发服务器

从 [apps/research-console](/Users/kevinlin/Downloads/CodexPlusClaw/apps/research-console) 目录：

```bash
npm install
npm run dev
```

### 构建静态前端

```bash
npm run build
```

构建后产物位于：

- `apps/research-console/dist`

如果 control plane 检测到这个目录存在，它会自动把前端静态资源挂到根路径。

## 典型使用方式

### 作为日常研究控制面

1. 启动 control plane
2. 打开控制台
3. 在 `Live` 页面发起或监视 `run`
4. 在 `Runs` 与 `Run Detail` 页面看具体行为链
5. 在 `Compare` / `Research` 页面看 cohort 对比与研究摘要
6. 在 `Artifacts` 页面打开 Markdown / CSV 证据包

### 作为已有 campaign 的证据浏览器

即使没有活跃 run，控制台也能直接浏览 `artifacts/` 下已有 campaign：

- 读取 campaign manifest
- 浏览 run evidence
- 浏览 Markdown 专题
- 浏览 CSV 数据集
- 打开单个 artifact 文件内容

## 当前限制

- 这是本地优先研究控制台，不做多用户权限模型
- SSE 当前主要推送 process / workspace 变化，某些更细粒度的 artifact 行级事件仍然通过轮询后的 HTTP 聚合进入 UI
- UI 已经高度对齐 [designsystem.md](/Users/kevinlin/Downloads/CodexPlusClaw/designsystem.md)，但后续还可以继续增强更细的 live rail 联动与 cohort compare 分析面
