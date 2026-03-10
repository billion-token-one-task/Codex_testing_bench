---
summary: "Message flow, sessions, queueing, streaming, and how OpenClaw projects Codex thread history."
read_when:
  - Explaining how inbound messages become replies
  - Clarifying sessions, queueing modes, or streaming behavior
title: "Messages"
---

# Messages

This page ties together inbound messages, session keys, queueing, streaming, and history projection.

## Message flow

```text
Inbound message
  -> routing / bindings -> session key
  -> active run handling / queueing
  -> Codex turn or other runtime turn
  -> OpenClaw event projection
  -> outbound replies
```

## Sessions and clients

Sessions are owned by the gateway, not by clients.

- Direct chats can collapse into the agent main session key depending on `session.dmScope`.
- Groups, channels, and bound threads get separate session keys.
- The gateway stores shell metadata locally.
- For Codex-backed sessions, the canonical message history is the Codex thread. The Control UI and TUI render gateway-projected history from that canonical state.

## Pending history versus stored history

OpenClaw keeps pending history buffers for messages that have not yet been folded into a run. Those buffers:

- include pending group messages that did not trigger a run
- exclude messages already represented in the canonical history for the active session

For Codex-backed sessions, that means “already represented” is defined by the Codex thread projection rather than by treating a local transcript file as the authoritative source.

## Queueing and followups

If a run is active, OpenClaw can:

- interrupt
- steer
- follow up
- collect

See [Queueing](/concepts/queue) for policy details.

## Streaming and structure

OpenClaw can surface:

- assistant text
- tool lifecycle
- plan deltas
- reasoning summaries
- command/file-change output
- compaction and review events

Legacy channels may still see a text-first projection, while the Control UI can consume the richer structured event model.
