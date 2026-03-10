---
summary: "Manual onboarding wizard for remote setups, advanced auth, and detailed gateway/channel configuration."
read_when:
  - Running or configuring the manual onboarding wizard
  - Setting up a remote or advanced environment
title: "Onboarding Wizard (CLI)"
sidebarTitle: "Onboarding: CLI"
---

# Onboarding Wizard (CLI)

The onboarding wizard is the **manual and advanced** setup path for OpenClaw.

Use it when you want:

- remote gateway setup
- advanced provider/auth flows
- explicit channel-by-channel configuration
- non-interactive scripting with the older wizard surface

For the normal local CodexPlusClaw happy path, use:

```bash
openclaw setup --one-click
```

Main manual entry point:

```bash
openclaw onboard
```

<Info>
Fastest first chat: open the Control UI. Run `openclaw dashboard` and chat in the browser.
</Info>

## What the wizard is for

The wizard still matters, but it is no longer the recommended first step for most local installs.

Use `openclaw onboard` or `openclaw setup --wizard` when you need:

- local setup with full manual control
- remote gateway client mode
- advanced provider auth choices beyond one-click
- guided channel and daemon configuration
- scripted non-interactive onboarding

## QuickStart vs Advanced

The wizard starts with **QuickStart** vs **Advanced**:

<Tabs>
  <Tab title="QuickStart">
    - Local gateway
    - Default workspace
    - Token auth
    - Recommended channel defaults
    - Minimal prompts
  </Tab>
  <Tab title="Advanced">
    - Full control over mode, auth, workspace, gateway, channels, daemon, and skills
  </Tab>
</Tabs>

## What the wizard configures

**Local mode** can walk you through:

1. model and auth selection
2. workspace location
3. gateway bind/port/auth settings
4. channels and pairing defaults
5. daemon install
6. health checks
7. skills recommendations

**Remote mode** configures the local client to connect to a gateway elsewhere.

## Follow-up commands

```bash
openclaw configure
openclaw agents add <name>
```

## Related docs

- CLI command reference: [`openclaw onboard`](/cli/onboard)
- One-click setup: [`openclaw setup`](/cli/setup)
- Wizard reference: [Wizard Reference](/reference/wizard)
- Onboarding overview: [Onboarding Overview](/start/onboarding-overview)
