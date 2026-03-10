# CodexPlusClaw

<p align="center">
    <picture>
        <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/openclaw/openclaw/main/docs/assets/openclaw-logo-text-dark.png">
        <img src="https://raw.githubusercontent.com/openclaw/openclaw/main/docs/assets/openclaw-logo-text.png" alt="OpenClaw" width="500">
    </picture>
</p>

**CodexPlusClaw** is an experimental **academic research, benchmarking, and testbed fork** of OpenClaw.

This fork studies a specific architecture:

- **OpenClaw as the shell**
  - gateway
  - channels and integrations
  - Control UI
  - local services and daemon lifecycle
  - session identity and routing
  - operator-facing approvals and externalities
- **Codex app-server as the brain**
  - agent turns
  - planning
  - review
  - approvals and request-user-input flows
  - skills
  - thread lifecycle and compaction
  - GPT-5.4 as the default local model

The purpose of this repository is not to present a polished consumer assistant product. It is a platform for:

- benchmarking shell-vs-brain boundaries
- evaluating Codex as a drop-in harness replacement inside OpenClaw
- testing integration coverage across OpenClaw’s local and channel surfaces
- studying approval, skills, session, and compaction behavior under a unified runtime

## Research focus

This fork is centered on the question:

> Can OpenClaw remain the integration-heavy local shell while Codex becomes the sole built-in agent harness?

The current implementation aims to make that boundary concrete:

- Codex app-server is the built-in runtime on the Codex path
- one-click setup is Codex-first
- Codex thread history is treated as the canonical conversation history for Codex-backed sessions
- OpenClaw projects Codex-native events, approvals, tools, and skills into its own gateway and UI surfaces

## What this repo is for

Use this repository if you want to:

- reproduce Codex/OpenClaw integration experiments
- run local benchmark scenarios across channels, Control UI, and operator flows
- evaluate session, compaction, skills, and approval behavior in a hybrid shell/brain architecture
- compare upstream OpenClaw assumptions against a Codex-first runtime design
- extend the testbed with additional measurements, fixtures, and end-to-end probes

## Current architecture

```text
chat apps / Control UI / nodes / CLI
                |
                v
         OpenClaw Gateway shell
                |
                v
          Codex app-server brain
                |
                v
             GPT-5.4
```

OpenClaw owns the outer platform boundary. Codex owns the inner agent runtime boundary.

## Status

This repository is **experimental**.

What is already in scope:

- Codex-first one-click setup
- Codex app-server compatibility checks
- Codex-backed session and compaction plumbing
- operator request bridging
- Control UI rendering for Codex-oriented flows
- broad docs migration toward the CodexPlusClaw architecture

What is still best treated as research/testbed territory rather than product claims:

- live credentialed end-to-end validation against every external channel/account combination
- full browser/CDP parity beyond the remaining browser-specific failures
- long-term stability guarantees across future upstream changes in both OpenClaw and Codex

## Quick start

Runtime: **Node >= 22**

```bash
npm install -g openclaw@latest
openclaw setup --one-click
```

That flow installs or upgrades a compatible Codex CLI, configures the local gateway and Control UI, prepares the workspace and skills layout, validates the Codex app-server surface, and opens the dashboard.

## Development setup

```bash
git clone https://github.com/openclaw/openclaw.git
cd openclaw
pnpm install
pnpm ui:build
pnpm build
pnpm openclaw setup --one-click
```

Useful commands:

```bash
pnpm gateway:watch
pnpm check:docs
pnpm exec vitest run
```

## Benchmarking and testbed use

Suggested evaluation surfaces in this fork:

- setup/bootstrap behavior
- Codex capability probing
- session routing and persistence
- thread read/fork/compact flows
- operator approvals and request-user-input loops
- skills loading and path synchronization
- channel-to-gateway-to-runtime behavior
- Control UI event projection

The most relevant docs are:

- [Getting Started](https://docs.openclaw.ai/start/getting-started)
- [Architecture](https://docs.openclaw.ai/concepts/architecture)
- [Agent Runtime](https://docs.openclaw.ai/concepts/agent)
- [Session Management](https://docs.openclaw.ai/concepts/session)
- [Compaction](https://docs.openclaw.ai/concepts/compaction)
- [Skills](https://docs.openclaw.ai/tools/skills)
- [Control UI](https://docs.openclaw.ai/web/control-ui)

## Provenance

This repository is based on upstream OpenClaw and experiments with a Codex-first runtime boundary inside that shell.

Upstream and related references:

- [OpenClaw](https://github.com/openclaw/openclaw)
- [OpenAI Codex](https://github.com/openai/codex)

## License

[MIT](LICENSE)
