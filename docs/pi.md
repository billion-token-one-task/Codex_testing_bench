---
title: "Pi Runtime (Legacy)"
summary: "Historical note for the older embedded Pi integration."
read_when:
  - You are reading older design discussions or migrating historical Pi-era docs
---

# Pi Runtime (Legacy)

This page is retained for historical context only.

CodexPlusClaw no longer documents Pi as the supported built-in runtime architecture. The current built-in runtime is:

- **Codex app-server** as the brain
- **OpenClaw** as the shell for channels, sessions, Control UI, skills management, integrations, and setup

If you are looking for the current architecture, start here instead:

- [Architecture](/concepts/architecture)
- [Agent runtime](/concepts/agent)
- [Session management](/concepts/session)
- [Compaction](/concepts/compaction)

If you are maintaining legacy Pi-era code paths during migration work, treat the old Pi docs and code as compatibility material rather than the product’s current runtime contract.
