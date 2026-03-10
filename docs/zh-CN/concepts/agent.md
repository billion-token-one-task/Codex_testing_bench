---
read_when:
  - 更改智能体运行时、工作区或会话行为时
summary: Codex app-server 运行时、工作区契约与会话引导。
title: 智能体运行时
x-i18n:
  generated_at: "2026-03-11T00:00:00Z"
  model: gpt-5
  provider: openai
  source_path: concepts/agent.md
---

# 智能体运行时 🤖

CodexPlusClaw 使用 **Codex app-server** 作为内置运行时。

OpenClaw 负责外层 shell：

- 渠道身份和会话 key
- Control UI 与 Gateway 事件投影
- 本地工具与集成
- setup / doctor / 审批展示
- 围绕 Codex 线程的持久化元数据

Codex 负责内部 harness：

- 模型执行
- planning / review
- approvals 和 request-user-input
- skills 加载
- 线程历史与 compaction
- 沙箱命令和文件修改语义

## 工作区

OpenClaw 使用单一工作区目录（`agents.defaults.workspace`）作为默认
`cwd`。建议使用 `openclaw setup` 或 `openclaw setup --one-click` 初始化
工作区。

完整布局参见：[智能体工作区](/concepts/agent-workspace)

## 引导文件

在 `agents.defaults.workspace` 中，OpenClaw 期望这些可编辑文件：

- `AGENTS.md`
- `SOUL.md`
- `TOOLS.md`
- `BOOTSTRAP.md`
- `IDENTITY.md`
- `USER.md`

新会话开始时，OpenClaw 会把这些内容注入到智能体上下文中。

## Skills

OpenClaw 从三个位置加载 skills：

- 内置 skills
- 共享/托管：`~/.agents/skills`
- 工作区：`<workspace>/.agents/skills`

OpenClaw 仍然保留旧的 `<workspace>/skills` 兼容层，但 `.agents/skills`
才是 CodexPlusClaw 的标准布局。

## Codex 集成契约

OpenClaw 通过 JSON-RPC v2 over stdio 与 Codex app-server 通信，并依赖：

- `initialize`（`experimentalApi`）
- `skills/list`
- `thread/start` / `thread/resume`
- `thread/read`
- `thread/fork`
- `thread/compact/start`
- `turn/start` / `turn/interrupt`
- `review/start`
- approvals 和 request-user-input 相关 server requests

OpenClaw 也会把自己的本地集成作为 `openclaw_*` 动态工具暴露给 Codex。

## 会话

对于 Codex 支持的会话，**Codex 线程历史是运行时的事实来源**。
OpenClaw 持久化的主要是这些元数据：

- `sessionKey -> threadId`
- `lastTurnId`
- `threadStatus`
- 运行时与协议兼容信息

OpenClaw 仍会保留本地投影和兼容性历史文件，以支持 UI、聊天历史和迁移场景，
但不会把这些本地文件当成 Codex 线程的替代品。
