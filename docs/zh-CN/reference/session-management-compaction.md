---
read_when:
  - 你需要调试会话 ID、本地会话元数据或压缩行为
  - 你正在更改压缩或压缩前的内务处理逻辑
summary: 深入了解会话元数据、Codex 线程历史和压缩行为。
title: 会话管理深入了解
x-i18n:
  generated_at: "2026-03-11T00:00:00Z"
  model: gpt-5
  provider: openai
  source_path: reference/session-management-compaction.md
---

# 会话管理与压缩（深入了解）

本文档解释 CodexPlusClaw 如何管理会话：

- shell 层的 `sessionKey`
- 本地会话元数据 `sessions.json`
- Codex 线程历史
- 本地缓存投影
- 手动和自动 compaction

## 事实来源

对于 Codex 支持的会话：

- **OpenClaw** 负责路由、会话 key、本地元数据、缓存投影和保留策略
- **Codex** 负责规范的线程历史、turn 历史和线程 compaction 状态

这意味着，本地兼容性文件不是最终权威；Codex 线程才是运行时事实来源。

## 本地持久化

每个智能体在 Gateway 主机上的主要文件：

- `~/.openclaw/agents/<agentId>/sessions/sessions.json`
- `~/.openclaw/agents/<agentId>/sessions/` 中的缓存投影和兼容性工件

常见字段包括：

- `sessionKey`
- `sessionId`
- `threadId`
- `lastTurnId`
- `threadStatus`
- 协议与运行时兼容元数据

## Codex 线程操作

Codex 会话依赖这些线程操作：

- `thread/start`
- `thread/resume`
- `thread/read`
- `thread/fork`
- `thread/compact/start`

OpenClaw 会把这些线程语义映射到自己的会话模型和 UI 中。

## 压缩

对于 Codex 会话，`/compact` 和后台压缩都映射为 Codex 的
`thread/compact/start`。

OpenClaw 在压缩前后仍可能做本地内务处理，例如：

- 记忆刷新提醒
- 会话元数据更新
- UI 与事件总线投影

但真正持久化的压缩结果由 Codex 线程决定。

## 维护

本地会话维护设置仍然适用于 OpenClaw 管理的 shell 文件：

- `session.maintenance.pruneAfter`
- `session.maintenance.maxEntries`
- `session.maintenance.rotateBytes`
- `session.maintenance.maxDiskBytes`

这些设置只约束 OpenClaw 的本地文件，不会改写 Codex 线程的保留语义。
