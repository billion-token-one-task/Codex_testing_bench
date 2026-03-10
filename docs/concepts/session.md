---
summary: "Session management rules, keys, persistence, and Codex thread authority."
read_when:
  - Modifying session handling or storage
title: "Session Management"
---

# Session Management

OpenClaw routes inbound messages into session keys such as `agent:<agentId>:<...>`. Those keys are the shell-level identity for a conversation. In CodexPlusClaw, the **conversation authority for Codex-backed sessions is the Codex thread**, while OpenClaw stores routing metadata, projections, and policy state around it.

## Direct-message scope

Use `session.dmScope` to control how direct messages are grouped:

- `main`
- `per-peer`
- `per-channel-peer`
- `per-account-channel-peer`

For multi-user inboxes, prefer `per-channel-peer` or `per-account-channel-peer`.

## Source of truth

All session ownership lives with the gateway, but there are two layers:

- **OpenClaw shell state**: `sessionKey`, routing metadata, UI projections, status, overrides
- **Codex thread state**: canonical message history, compaction state, review state, turn history

UI clients should talk to the gateway, not read local files directly.

## Where state lives

- Shell metadata: `~/.openclaw/agents/<agentId>/sessions/sessions.json`
- Cached projections and compatibility transcript artifacts: `~/.openclaw/agents/<agentId>/sessions/`
- Shared config and credentials: `~/.openclaw/`

Codex-backed sessions also store:

- `threadId`
- `lastTurnId`
- `threadStatus`
- runtime/protocol compatibility metadata

OpenClaw does not use old Pi/Tau session folders as an active runtime source.

## Compaction and resets

- `/new` or `/reset` starts a fresh shell session binding.
- `/compact` maps to Codex thread compaction for Codex-backed sessions.
- Reset and maintenance policies still apply to the OpenClaw shell metadata and cached artifacts.

## Maintenance

OpenClaw still applies retention and disk controls to the local session store:

- `session.maintenance.mode`
- `session.maintenance.pruneAfter`
- `session.maintenance.maxEntries`
- `session.maintenance.rotateBytes`
- `session.maintenance.maxDiskBytes`

These controls manage local shell artifacts. They do not replace Codex thread history semantics.
