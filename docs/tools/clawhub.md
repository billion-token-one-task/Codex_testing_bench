---
summary: "ClawHub guide: discover, install, publish, and sync skills into the Codex-compatible layout."
read_when:
  - Introducing ClawHub to new users
  - Installing, searching, or publishing skills
  - Explaining ClawHub CLI flags and sync behavior
title: "ClawHub"
---

# ClawHub

ClawHub is the public skill registry for OpenClaw and CodexPlusClaw. It lets you search, install, update, publish, and sync skill bundles.

Site: [clawhub.ai](https://clawhub.ai)

## What ClawHub does

- discovers public skills
- versions and hosts skill bundles
- installs skills into your local environment
- helps you publish and back up your own skills

## How it fits into CodexPlusClaw

ClawHub installs and syncs skills for the OpenClaw shell, but the runtime-facing layout is Codex-compatible:

- shared/user skills: `~/.agents/skills`
- workspace skills: `<workspace>/.agents/skills`

If a legacy workspace uses `./skills` or `<workspace>/skills`, OpenClaw syncs that content into the Codex-compatible workspace root so Codex and OpenClaw see the same skills.

## Quick start

```bash
npm i -g clawhub
clawhub search "calendar"
clawhub install <skill-slug>
```

Start a new OpenClaw session after install so the runtime sees the new skill set.

## Common workflows

### Search

```bash
clawhub search "postgres backups"
```

### Install

```bash
clawhub install my-skill-pack
```

### Update

```bash
clawhub update --all
```

### Publish

```bash
clawhub publish ./my-skill --slug my-skill --name "My Skill" --version 1.0.0 --tags latest
```

### Sync a local skills collection

```bash
clawhub sync --all
```

## Defaults and paths

- `--workdir <dir>`: base working directory
- `--dir <dir>`: skills directory relative to `workdir`
- `--root <dir...>`: extra roots for `clawhub sync`

OpenClaw will prefer the Codex-compatible roots even if older skill layouts still exist.

For more on runtime loading and precedence, see [Skills](/tools/skills).
