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
- 当前选中 campaign 的 operational dossier
- 直接执行 `bootstrap-local / run / grade / report`

### Live

面向正在发生的运行：

- 活跃进程
- 实时 stdout / stderr
- 最近事件
- 并行槽位与当前运行负载
- live run cards
- `activity heat / current focus / warning tape`
- `campaign operational summary`
  - `active_warning_count`
  - `stalled_live_run_count`
  - `personality_fallback_live_count`
  - `heat_counts`
  - `focus_samples`
  - `latest_message_previews`
- latest tool / patch / mechanism rails
- focused run spotlight
  - 选中某个活跃 run 后，直接显示它的 live message rail、tool / command rail、patch / mechanism rail
  - 直接显示这个 run 的 operational snapshot
  - 直接显示 `attempt-log.txt` 的 tail
- process dossier
  - inspect 某个受管进程后，直接看完整 command、stdout/stderr 计数、最近输出、最后输出时间
- live overview 数据总线
  - `active_live_runs`
  - `hottest_live_runs`
  - `stalled_live_runs`
  - `latest_global_focus_samples`
  - `latest_global_message_previews`
  - `latest_global_warnings`
- mission status strip
  - 当前 campaign 热度、活跃 cohort、warnings、heat mix、signal mix 一次性看全
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
- same-task 2x2 quadrant board
- 2x2 signal board
  - 同题四格直接看 visible output / tool count / bridge language / verification framing 的相对强弱
- phrase delta surface
- tool inventory surface
- mechanism surface
- 直接跳转四个 quadrant 对应 run 的 war room

### Artifacts

面向证据包浏览：

- `report.txt`
- Markdown 专题
- `datasets/*.csv`
- 直接预览单个 artifact 内容
- campaign / run 双 scope 切换
- source-of-truth role / scope / size / row stats
- truth level 视角
  - `raw_truth`
  - `derived_summary`
  - `derived_evidence`
  - `human_readable_dossier`
- campaign / run operational dossier
  - 当前 phase / latest focus / latest message
  - live warnings
  - latest report / dataset readiness
  - artifact / event table counts

### Research

面向 hypothesis / task-class / mechanism 的研究工作台：

- claim evidence
- personality mechanism
- task-class 维度摘要
- 研究型聚合表
- methods appendix dock
- observability / probe / study docs 快速预览

### Run Detail

这是最重要的一页，当前已经是完整 war room：

- 运行概览
- 实时状态与最新更新时间
- live snapshot 状态带
  - `current phase`
  - `activity heat`
  - `current focus`
  - `warnings`
  - `message category`
  - `top tool route`
  - `active skill names`
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
- operational snapshot
  - artifact type counts
  - event table counts
  - latest report / latest dataset
  - run observer warnings
- live event rail 统计
  - message / tool / patch / mechanism appended counts
- token / turn strip
  - 每 turn token 压力条
  - 快速看哪个阶段最耗 token
- message mechanism strip
  - bridge / verification / state externalization / collaboration 四类语言强度一眼看清

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

最新一版里，顶部 `ActionLauncher` 已经升级成更像研究发射台的版本：

- context quick actions
  - 直接使用当前 active campaign
  - 直接 clone active config
  - 直接对 active campaign 执行 `grade / report`
- preset shortcuts
  - 单题 `2x2`
  - `5` 题 `2x2` pilot
  - SWE-bench evidence batch
- recent launches
  - 最近动作历史会本地记忆
  - 可直接从历史目标再次发起 run / grade / report
- prepare 阶段显式参数
  - `max_parallel_runs`
  - `per_repo_prepare_parallelism`

## 实时能力

当前控制台已经不再只是“扫一遍 artifacts 再刷新页面”，而是通过共享 SSE 总线和 live overview 聚合层，把控制面健康度、活跃 run snapshot 与 artifact 变化串成一条连续数据线。

新增的 live 关键面板包括：

- `Stream Bus`
  - 全局事件流连接状态
  - 最近事件时间
  - 事件总数 / 错误数
- `Control Plane Health`
  - workspace refresh
  - latest process output
  - active campaign
  - focus sample
  - warnings
- `Jump Desk`
  - 一键跳转当前 focused run 的 war room
  - 一键跳转 compare / artifacts
- `Active War Rooms`
  - 从 campaign 直接进入正在跑的 run

`Live` 页面现在更像 mission control，而不是 process 列表：

- mission status strip
- parallel slots
- hot runs board
- stalled / warning board
- focused run spotlight
- process dossier

`Run Detail` 现在也增加了更强的 live 头部和 operational snapshot：

- stream status
- current phase / heat / latest focus
- latest tool / patch / command / mechanism
- artifact type counts
- event table counts
- latest report / dataset readiness

这意味着：

- 你可以在 `Campaigns` 看 experiment ledger
- 在 `Live` 看当前系统状态与异常
- 在 `Run Detail` 看单题 war room
- 在 `Compare` 看 2x2 研究对比

而不再需要自己在文件系统里来回切换。

当前版本通过 SSE 推送：

- process stdout / stderr
- process 生命周期变化
- workspace index 更新
- campaign / run 状态变化
- active run live snapshot
- `run.message.appended`
- `run.tool.appended`
- `run.patch.appended`
- `run.command.appended`
- `run.personality.appended`
- `run.skill.appended`
- `run.token.appended`
- `run.mechanism.appended`
- `run.live.updated`
- `run.summary.updated`
- `run.phase.changed`
- `run.focus.changed`
- `run.warning.appended`
- `campaign.summary.updated`

同时，控制台会通过 HTTP 拉取并刷新：

- campaign list / detail
- run detail
- reports / datasets
- artifact preview
- artifact tail

最新一版里，`Live` 和 `Run Detail` 已经开始直接消费 run 级结构化事件流：

- `GET /api/runs/:id/stream`
- `run.message.appended`
- `run.tool.appended`
- `run.patch.appended`
- `run.command.appended`
- `run.personality.appended`
- `run.skill.appended`
- `run.token.appended`

这让控制台可以更直接回答：

- 现在它在说什么
- 现在它在调用哪个具体工具
- patch 到哪一步了
- personality / skill / mechanism 有没有刚发生变化

对于活跃 run，控制台会把：

- process 输出
- live snapshot
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
- `GET /api/runs/:id/operational-summary`
- `GET /api/runs/:id/detail`
- `GET /api/runs/:id/attempts/:n`
- `GET /api/artifacts/file`
- `GET /api/artifacts/tail`
- `GET /api/events`
- `GET /api/live/runs`
- `GET /api/live/runs/:id`
- `GET /api/campaigns/:id/operational-summary`
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
