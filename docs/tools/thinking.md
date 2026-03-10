---
summary: "Directive syntax for /think, /verbose, and /reasoning in CodexPlusClaw."
read_when:
  - Adjusting thinking or verbose directive parsing or defaults
title: "Thinking Levels"
---

# Thinking Levels (/think directives)

## What it does

- Inline directive in any inbound body: `/t <level>`, `/think:<level>`, or `/thinking <level>`.
- Levels (aliases): `off | minimal | low | medium | high | xhigh | adaptive`
  - minimal -> `think`
  - low -> `think hard`
  - medium -> `think harder`
  - high -> `ultrathink`
  - xhigh -> `ultrathink+`
  - adaptive -> provider-managed adaptive reasoning budget when supported

## Resolution order

1. Inline directive on the message
2. Session override
3. Global default (`agents.defaults.thinkingDefault`)
4. Model/provider fallback behavior

## Setting a session default

- Send a directive-only message such as `/think:medium`.
- That sticks for the current session until cleared or reset.
- Send `/think` with no argument to inspect the current setting.

## Application by runtime

- **Codex app-server**: the resolved level is forwarded through the Codex-backed agent path and stored in session metadata so the shell and Control UI stay in sync.
- **ACP sessions**: OpenClaw forwards the directive to the ACP-managed harness when the target runtime supports it.

## Verbose directives (/verbose or /v)

- Levels: `on | full | off`.
- Directive-only messages toggle the session default.
- Inline directives affect only that message.
- When verbose is enabled, OpenClaw surfaces structured tool and runtime events as separate UI/chat summaries when the target runtime emits them.

For Codex-backed runs, that includes dynamic tool lifecycle, command output, file-change output, plan deltas, and other structured item events when available.

## Reasoning visibility (/reasoning)

- Levels: `on | off | stream`.
- Directive-only message toggles whether reasoning is shown.
- When enabled, reasoning is emitted separately from the final answer.
- `stream` is channel-dependent; some surfaces support draft-style incremental reasoning display while others fall back to non-streamed visibility.

## Related

- [Compaction](/concepts/compaction)
- [Session management](/concepts/session)
- [Token use](/reference/token-use)
- [Elevated mode](/tools/elevated)
