---
summary: "Multi-agent routing: isolated agents, workspaces, auth, and bindings."
title: "Multi-Agent Routing"
read_when: "You want multiple isolated agents in one gateway process."
status: active
---

# Multi-Agent Routing

An agent is an isolated shell + runtime context with its own workspace, auth, sessions, and bindings.

## What one agent includes

- workspace files (`AGENTS.md`, `SOUL.md`, `USER.md`, memory, workspace-local skills)
- per-agent auth and state under `~/.openclaw/agents/<agentId>/agent`
- per-agent session metadata under `~/.openclaw/agents/<agentId>/sessions`

Skills are per-agent via each workspace’s:

```text
<workspace>/.agents/skills
```

Shared skills live in:

```text
~/.agents/skills
```

OpenClaw can still ingest older `<workspace>/skills` layouts as a compatibility layer, but `.agents/skills` is the canonical model.

## Single-agent mode

If you do nothing, OpenClaw runs one agent:

- `agentId`: `main`
- default workspace: `~/.openclaw/workspace`
- default built-in runtime: Codex app-server

## Multi-agent mode

Use `openclaw agents add <id>` and bindings to isolate different personas, workspaces, and channel routes behind the same gateway.
