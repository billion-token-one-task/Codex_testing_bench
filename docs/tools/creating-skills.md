---
title: "Creating Skills"
summary: "Build Codex-compatible skills that OpenClaw manages, syncs, and presents."
read_when:
  - You are creating a new custom skill
  - You need the current CodexPlusClaw skill layout
---

# Creating Custom Skills

In CodexPlusClaw, **skills are native Codex skills**. OpenClaw manages where they live, syncs legacy workspace installs into Codex-readable roots, and exposes them through the shell. It does **not** emulate skills as fake tools.

## Canonical skill locations

Use these locations first:

- Shared/user skills: `~/.agents/skills/<skill-name>/SKILL.md`
- Workspace skills: `<workspace>/.agents/skills/<skill-name>/SKILL.md`

OpenClaw still keeps the older `<workspace>/skills` layout as a compatibility layer, but the `.agents/skills` roots are canonical.

## Your first skill

### 1. Create the directory

```bash
mkdir -p ~/.agents/skills/hello-world
```

Or for a workspace-only skill:

```bash
mkdir -p ~/.openclaw/workspace/.agents/skills/hello-world
```

### 2. Create `SKILL.md`

```markdown
---
name: hello-world
description: Greets the user and confirms the skill wiring is working.
---

# Hello World

When the user asks for a greeting or a quick skill test:

1. Reply with a short friendly greeting.
2. Mention that the response came from the `hello-world` skill.
3. Do not invent extra steps or tools unless the user asks for them.
```

### 3. Reload or refresh

OpenClaw and Codex will pick up new skills on the next relevant session start. To force a refresh:

```bash
openclaw skills check
openclaw gateway restart
```

## How execution works

When OpenClaw intentionally routes a skill, it can send:

- the textual `$skill-name` marker, and
- the matching Codex `skill` input item path

That lets Codex load the exact OpenClaw-managed skill quickly and deterministically.

## Best practices

- Keep the description concrete. Say when the skill applies and what good output looks like.
- Prefer instructions over roleplay. A skill should teach behavior, not identity.
- Reuse existing OpenClaw and Codex tools instead of embedding giant shell snippets.
- Keep supporting files next to the skill if they are truly required.
- Test with a fresh session so you know Codex loaded the newest version.

## Testing a skill

```bash
openclaw agent --message "Use the hello-world skill to greet me."
```

Also check the Control UI skills/status surfaces if you want to verify that the skill is visible and enabled.

## Sharing skills

Use [ClawHub](/tools/clawhub) to publish and sync reusable skills. OpenClaw syncs installs into the Codex-compatible skills layout so the same skill set works for both the shell and the runtime.
