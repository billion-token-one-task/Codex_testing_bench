---
summary: "OpenClaw CLI reference for `openclaw` commands, setup, runtime control, and operations."
read_when:
  - Adding or modifying CLI commands or options
  - Documenting new command surfaces
title: "CLI Reference"
---

# CLI reference

The CLI is the operator surface for CodexPlusClaw. For most local installs, start with:

```bash
openclaw setup --one-click
```

Use `openclaw onboard` only when you intentionally want the manual wizard path.

## Command pages

- [`setup`](/cli/setup)
- [`onboard`](/cli/onboard)
- [`configure`](/cli/configure)
- [`config`](/cli/config)
- [`completion`](/cli/completion)
- [`doctor`](/cli/doctor)
- [`dashboard`](/cli/dashboard)
- [`backup`](/cli/backup)
- [`reset`](/cli/reset)
- [`uninstall`](/cli/uninstall)
- [`update`](/cli/update)
- [`message`](/cli/message)
- [`agent`](/cli/agent)
- [`agents`](/cli/agents)
- [`acp`](/cli/acp)
- [`status`](/cli/status)
- [`health`](/cli/health)
- [`sessions`](/cli/sessions)
- [`gateway`](/cli/gateway)
- [`logs`](/cli/logs)
- [`system`](/cli/system)
- [`models`](/cli/models)
- [`memory`](/cli/memory)
- [`directory`](/cli/directory)
- [`nodes`](/cli/nodes)
- [`devices`](/cli/devices)
- [`node`](/cli/node)
- [`approvals`](/cli/approvals)
- [`sandbox`](/cli/sandbox)
- [`tui`](/cli/tui)
- [`browser`](/cli/browser)
- [`cron`](/cli/cron)
- [`dns`](/cli/dns)
- [`docs`](/cli/docs)
- [`hooks`](/cli/hooks)
- [`webhooks`](/cli/webhooks)
- [`pairing`](/cli/pairing)
- [`qr`](/cli/qr)
- [`plugins`](/cli/plugins)
- [`channels`](/cli/channels)
- [`security`](/cli/security)
- [`secrets`](/cli/secrets)
- [`skills`](/cli/skills)
- [`daemon`](/cli/daemon) (legacy alias namespace)
- [`clawbot`](/cli/clawbot) (legacy alias namespace)
- [`voicecall`](/cli/voicecall) (plugin; if installed)

## High-value commands

- `openclaw setup --one-click`: install, verify, and launch the local CodexPlusClaw stack.
- `openclaw dashboard`: open the Control UI.
- `openclaw status --all`: collect a pasteable local diagnosis.
- `openclaw gateway status`: inspect service/runtime state.
- `openclaw skills check`: inspect skill roots, status, and sync issues.
- `openclaw sessions cleanup --dry-run`: preview session maintenance and retention work.
- `openclaw models list`: inspect the available model catalog.

## Global flags

- `--dev`: isolate state under `~/.openclaw-dev` and shift default ports.
- `--profile <name>`: isolate state under `~/.openclaw-<name>`.
- `--no-color`: disable ANSI colors.
- `--update`: shorthand for `openclaw update` (source installs only).
- `-V`, `--version`, `-v`: print version and exit.
