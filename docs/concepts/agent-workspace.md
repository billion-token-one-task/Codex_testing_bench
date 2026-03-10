---
summary: "Agent workspace: location, layout, skills roots, and backup strategy."
read_when:
  - You need to explain the agent workspace or its file layout
  - You want to back up or migrate an agent workspace
title: "Agent Workspace"
---

# Agent workspace

The workspace is the agent's operating directory. OpenClaw treats it as the place for instructions, memory, and project-local skills. Codex uses it as the main working context for the built-in runtime.

This is separate from `~/.openclaw/`, which stores config, credentials, gateway state, and session metadata.

## Default location

- Default: `~/.openclaw/workspace`
- Profile-specific default: `~/.openclaw/workspace-<profile>`
- Override in `~/.openclaw/openclaw.json`

```json5
{
  agent: {
    workspace: "~/.openclaw/workspace",
  },
}
```

`openclaw setup --one-click`, `openclaw setup`, and `openclaw configure` can create the workspace and seed the bootstrap files when missing.

## Important behavior

- The workspace is the default `cwd`, not a hard sandbox.
- Relative paths resolve from the workspace.
- Absolute paths can still reach outside it unless sandboxing is enabled.
- When sandboxing is enabled, tools may run inside a sandbox workspace under `~/.openclaw/sandboxes`.

## Standard workspace files

- `AGENTS.md`: operating instructions and tool/behavior rules
- `SOUL.md`: tone, persona, and communication style
- `USER.md`: information about the user
- `IDENTITY.md`: agent identity details
- `TOOLS.md`: local conventions and tool notes
- `HEARTBEAT.md`: optional heartbeat checklist
- `BOOT.md`: optional startup checklist
- `BOOTSTRAP.md`: one-time first-run ritual for a brand-new workspace
- `memory/YYYY-MM-DD.md`: daily memory logs
- `MEMORY.md`: optional curated long-term memory
- `.agents/skills/`: workspace-local Codex-compatible skills
- `canvas/`: optional canvas/UI assets for nodes and web surfaces

OpenClaw still supports the older `skills/` folder as a compatibility layer, but the canonical workspace skills root is:

```text
<workspace>/.agents/skills
```

## What is not in the workspace

These stay under `~/.openclaw/`:

- `~/.openclaw/openclaw.json`
- `~/.openclaw/credentials/`
- `~/.openclaw/agents/<agentId>/sessions/`
- `~/.openclaw/agents/<agentId>/agent/`
- `~/.openclaw/logs/`

For Codex-backed sessions, OpenClaw keeps shell metadata and cached projections locally while the canonical conversation history lives in the Codex thread state.

## Skills layout

Use these roots:

- Shared/user skills: `~/.agents/skills`
- Workspace skills: `<workspace>/.agents/skills`

OpenClaw syncs older installs such as `<workspace>/skills` into the Codex-compatible layout where needed.

## Backup strategy

Treat the workspace as private memory and keep it in a private git repo.

```bash
cd ~/.openclaw/workspace
git init
git add AGENTS.md SOUL.md TOOLS.md USER.md IDENTITY.md memory/ .agents/skills/
git commit -m "Initialize OpenClaw workspace"
```

Keep credentials, logs, and `~/.openclaw` state out of version control.
