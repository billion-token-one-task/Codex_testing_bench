---
summary: "Overview of setup paths: one-click local bootstrap first, manual wizard second."
read_when:
  - Choosing a setup path
  - Setting up a new environment
title: "Onboarding Overview"
sidebarTitle: "Onboarding Overview"
---

# Onboarding Overview

OpenClaw now has two main setup paths:

- **One-click local bootstrap** for most CodexPlusClaw installs
- **Manual wizard** for remote or advanced setups

## Recommended path

Run:

```bash
openclaw setup --one-click
```

Use this when you want a working local OpenClaw + Codex stack with minimal ceremony.

It handles:

- compatible Codex installation or upgrade
- Codex auth bootstrap
- local gateway config
- workspace and skills roots
- daemon install
- health checks and dashboard launch

Docs:

- [Getting Started](/start/getting-started)
- [`openclaw setup`](/cli/setup)

## Manual wizard

Run:

```bash
openclaw onboard
```

Use the manual wizard when you want full control over remote mode, provider/auth details, or channel-by-channel guided setup.

Docs:

- [Onboarding Wizard (CLI)](/start/wizard)
- [`openclaw onboard` command](/cli/onboard)

## macOS app onboarding

Use the OpenClaw app when you want a guided first run on macOS.

Docs:

- [Onboarding (macOS App)](/start/onboarding)
