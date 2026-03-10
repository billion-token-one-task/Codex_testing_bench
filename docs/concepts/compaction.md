---
summary: "Context window + compaction: how OpenClaw keeps sessions under model limits"
read_when:
  - You want to understand auto-compaction and /compact
  - You are debugging long sessions hitting context limits
title: "Compaction"
---

# Context Window & Compaction

Every model has a **context window** (max tokens it can see). Long-running chats accumulate messages and tool results; once the window is tight, CodexPlusClaw compacts older history to stay within limits.

## What compaction is

For Codex-backed sessions, compaction is a **Codex thread operation**. OpenClaw
requests `thread/compact/start`, and Codex performs the compaction against the
thread history it owns.

What persists:

- Codex thread history is compacted on the Codex side.
- OpenClaw stores metadata and UI projections around that thread.
- Legacy transcript-trimming behavior still exists for older non-Codex session paths.

## Configuration

Use the `agents.defaults.compaction` setting in your `openclaw.json` to configure legacy/local compaction behavior where applicable.
For Codex-backed sessions, the important runtime contract is that OpenClaw can
trigger Codex thread compaction and surface compaction lifecycle events in the UI.

You can optionally specify a different model for compaction summarization via `agents.defaults.compaction.model`. This is useful when your primary model is a local or small model and you want compaction summaries produced by a more capable model. The override accepts any `provider/model-id` string:

```json
{
  "agents": {
    "defaults": {
      "compaction": {
        "model": "openrouter/anthropic/claude-sonnet-4-5"
      }
    }
  }
}
```

This also works with local models, for example a second Ollama model dedicated to summarization or a fine-tuned compaction specialist:

```json
{
  "agents": {
    "defaults": {
      "compaction": {
        "model": "ollama/llama3.1:8b"
      }
    }
  }
}
```

When unset, compaction uses the agent's primary model where a local summarization
path exists. Codex-native thread compaction uses the active Codex runtime.

## Auto-compaction (default on)

When a session nears or exceeds the model’s context window, Codex or OpenClaw
may trigger compaction depending on the runtime path. For Codex-backed sessions,
OpenClaw surfaces Codex context-compaction events to the chat UI.

You’ll see:

- `🧹 Auto-compaction complete` in verbose mode
- `/status` showing `🧹 Compactions: <count>`

Before compaction, OpenClaw can run a **silent memory flush** turn to store
durable notes to disk. See [Memory](/concepts/memory) for details and config.

## Manual compaction

Use `/compact` (optionally with instructions) to force a compaction pass:

```
/compact Focus on decisions and open questions
```

## Context window source

Context window is model-specific. OpenClaw uses the model definition from the configured provider catalog to determine limits.

## Compaction vs pruning

- **Compaction**: summarises and **persists** in JSONL.
- **Session pruning**: trims old **tool results** only, **in-memory**, per request.

See [/concepts/session-pruning](/concepts/session-pruning) for pruning details.

For Codex-backed sessions, `/compact` maps to Codex thread compaction rather than
rewriting the transcript locally.

## Tips

- Use `/compact` when sessions feel stale or context is bloated.
- Large tool outputs are already truncated; pruning can further reduce tool-result buildup.
- If you need a fresh slate, `/new` or `/reset` starts a new session id.
