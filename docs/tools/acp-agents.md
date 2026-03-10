---
summary: "Use ACP sessions when you intentionally want an external harness instead of the built-in Codex runtime."
read_when:
  - Running external coding harnesses through ACP
  - Setting up thread-bound ACP sessions
  - Troubleshooting ACP backend and plugin wiring
title: "ACP Agents"
---

# ACP agents

[Agent Client Protocol (ACP)](https://agentclientprotocol.com/) lets OpenClaw run external coding harnesses through ACP backends.

In CodexPlusClaw, ACP is **not** the built-in runtime path. The built-in runtime is Codex app-server. Use ACP only when you intentionally want a different external harness such as Claude Code, Gemini CLI, OpenCode, or a second Codex instance managed through ACP.

## When to use ACP

Use ACP when you want:

- an external harness that is not the built-in Codex runtime
- a thread-bound external coding session
- a separate persistent harness workflow with its own ACP backend

Use the built-in OpenClaw agent path when you want the normal CodexPlusClaw experience.

## Example operator flow

1. Spawn a session:
   - `/acp spawn claude --mode persistent --thread auto`
2. Work in the bound thread.
3. Check runtime state:
   - `/acp status`
4. Stop work:
   - `/acp cancel` or `/acp close`

## ACP versus built-in Codex

| Area              | Built-in runtime             | ACP session                          |
| ----------------- | ---------------------------- | ------------------------------------ |
| Default harness   | Codex app-server             | External ACP backend                 |
| Setup path        | `openclaw setup --one-click` | ACP backend/plugin config            |
| History authority | Codex thread state           | ACP backend session state            |
| Best for          | Normal CodexPlusClaw usage   | Explicit alternate harness workflows |

See also [Sub-agents](/tools/subagents).
