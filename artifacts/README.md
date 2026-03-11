# Artifacts

这个目录是 Codex research bench 在 GitHub 上可见的结果展示面。

设计意图是：

- 人可以直接在 GitHub 上浏览 campaign 结果
- 机器本地依然保留更重、更细的研究证据
- 仓库不会因为把所有 raw telemetry 全量倒进版本控制而变得不可读

## 适合提交到仓库的内容

- campaign manifest
- 选中数据集快照
- claim catalog 与 architecture map
- campaign 级 `report.txt`
- grader 摘要
- 每个 run 的 `manifest.json`
- 每个 run 的 `record.json`
- 每个 attempt 的 `run-summary.json`
- 每个 attempt 的 `probe-summary.json`
- 每个 attempt 的 `claim-evidence.json`
- 每个 attempt 的 `run-evidence.txt`
- 每个 attempt 的 `attempt-log.txt`
- 每个 attempt 的 `replay.json`

## 只保留本地并刻意忽略的内容

- 预热后的 repo cache
- 准备好的 workspace / worktree
- raw event JSONL 流
- raw diagnostics
- 完整 prompt 捕获
- environment staging 文件
- binary diff 和其他较重的临时产物

这样的拆分使得：

- GitHub 页面仍然清爽可读
- 同时又保留了在运行机器上重新生成更丰富本地证据的能力

## 第一次浏览一个 campaign 时的推荐顺序

1. `reports/report.txt`
2. `runs/<instance>/manifest.json`
3. `runs/<instance>/attempt-01/run-evidence.txt`
4. `runs/<instance>/attempt-01/attempt-log.txt`
