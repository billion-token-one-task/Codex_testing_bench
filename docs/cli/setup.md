---
summary: "CLI reference for `openclaw setup` (one-click Codex bootstrap + local workspace setup)"
read_when:
  - You’re doing first-run setup without the full onboarding wizard
  - You want to set the default workspace path
title: "setup"
---

# `openclaw setup`

`openclaw setup` is the preferred local bootstrap command for CodexPlusClaw.

It has two modes:

- `openclaw setup` — initialize `~/.openclaw/openclaw.json` and the local workspace layout.
- `openclaw setup --one-click` — install or upgrade a compatible Codex CLI, configure the local gateway + Control UI, prepare skills directories, run health checks, validate the Codex app-server contract, and open the dashboard.

Related:

- Getting started: [Getting started](/start/getting-started)
- Wizard: [Onboarding](/start/onboarding)
- Agent runtime: [Agent Runtime](/concepts/agent)
- Architecture: [Gateway Architecture](/concepts/architecture)

## Examples

```bash
openclaw setup
openclaw setup --workspace ~/.openclaw/workspace
openclaw setup --one-click
openclaw setup --one-click --auth-choice skip --no-open-dashboard
```

To run the manual wizard via setup:

```bash
openclaw setup --wizard
```

## Recommended path

For a new local install, use:

```bash
openclaw setup --one-click
```

That flow is intended to leave you with a working local stack:

1. OpenClaw config written
2. Workspace + sessions directories created
3. Compatible Codex CLI available
4. Codex auth checked or bootstrapped
5. Gateway daemon installed
6. Control UI assets ready
7. Codex app-server compatibility probe passed
8. Dashboard opened

## One-click behavior

`--one-click` makes OpenClaw act as the outer shell around Codex:

- Codex app-server becomes the built-in runtime
- `gpt-5.4` is set as the default model
- Codex runtime defaults are written under `agents.defaults.codex`
- CLI fallback wiring is aligned to launch `codex app-server --listen stdio://`
- workspace skills are synced into Codex-compatible skill roots
- health/doctor checks run before the flow completes

Compatibility checks include the Codex app-server surfaces OpenClaw depends on:

- `initialize` with `experimentalApi`
- `skills/list`
- `thread/start`
- `thread/read`
- `thread/compact/start`
- `thread/fork`
- `turn/start`
- `review/start`

If the installed Codex is too old or missing one of those capabilities, setup fails closed with an actionable repair path instead of silently degrading.

## Auth options

One-click setup supports these auth choices:

- browser/OAuth-style Codex login
- OpenAI API key
- `skip` for environments where auth is handled separately

When possible, credentials are left to Codex’s own secure storage behavior instead of OpenClaw inventing a second credential format.

## When to use `openclaw onboard` instead

Use `openclaw onboard` when you want:

- the manual onboarding wizard
- remote-gateway setup
- fine-grained interactive channel setup
- a more explicit, step-by-step auth/config flow

`setup --one-click` is the fastest local path.
`onboard` is the detailed/manual path.
