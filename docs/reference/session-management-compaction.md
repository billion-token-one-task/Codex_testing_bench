---
summary: "Deep dive into session metadata, Codex thread history, and compaction behavior."
read_when:
  - You need to debug session ids, local session metadata, or compaction behavior
  - You are changing compaction or pre-compaction housekeeping behavior
title: "Session Management Deep Dive"
---

# Session Management & Compaction (Deep Dive)

This document explains how CodexPlusClaw manages sessions end to end:

- shell-level session routing (`sessionKey`)
- local session metadata (`sessions.json`)
- Codex thread authority for history
- cached projections on disk
- manual and automatic compaction surfaces

## Source of truth

For Codex-backed sessions:

- **OpenClaw** owns routing, session keys, local metadata, cached projections, and retention policies.
- **Codex** owns canonical thread history, turn history, and thread compaction state.

That means local session artifacts are not the final authority for conversation history.

## Local persistence

Per agent on the gateway host:

- `~/.openclaw/agents/<agentId>/sessions/sessions.json`
- `~/.openclaw/agents/<agentId>/sessions/` for cached projections and compatibility artifacts

Important fields stored per session include:

- `sessionKey`
- `sessionId`
- `threadId`
- `lastTurnId`
- `threadStatus`
- protocol/runtime compatibility metadata

## Codex thread operations

Codex-backed sessions rely on thread operations such as:

- `thread/start`
- `thread/resume`
- `thread/read`
- `thread/fork`
- `thread/compact/start`

OpenClaw projects these thread semantics into its own session and UI model.

## Compaction

For Codex-backed sessions, `/compact` and background compaction flows map to Codex thread compaction.

OpenClaw may still do local housekeeping around compaction, such as:

- memory flush reminders
- local metadata updates
- UI and event-bus projection

But the persisted compaction authority is the Codex thread.

## Maintenance

Local session maintenance settings still apply to shell artifacts:

- `session.maintenance.pruneAfter`
- `session.maintenance.maxEntries`
- `session.maintenance.rotateBytes`
- `session.maintenance.maxDiskBytes`

These settings bound OpenClaw-managed files. They do not redefine Codex thread retention semantics.
