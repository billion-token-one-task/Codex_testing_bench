---
summary: "OpenClaw capabilities across channels, routing, local integrations, and the Codex runtime boundary."
read_when:
  - You want a full list of what OpenClaw supports
title: "Features"
---

## Highlights

<Columns>
  <Card title="Channels" icon="message-square">
    WhatsApp, Telegram, Discord, Slack, Signal, iMessage, BlueBubbles, and more with one gateway.
  </Card>
  <Card title="Codex Brain" icon="cpu">
    Codex app-server with planning, review, compaction, approvals, skills, and thread APIs.
  </Card>
  <Card title="Routing" icon="route">
    Multi-agent routing with isolated workspaces, sessions, and channel bindings.
  </Card>
  <Card title="Media" icon="image">
    Images, audio, video, and documents in and out.
  </Card>
  <Card title="Apps and UI" icon="monitor">
    Web Control UI, macOS companion app, TUI, and WebChat.
  </Card>
  <Card title="Mobile nodes" icon="smartphone">
    iOS and Android nodes with pairing, voice, camera, screen, and device commands.
  </Card>
</Columns>

## Full list

- Multi-channel gateway for WhatsApp, Telegram, Discord, Slack, Signal, iMessage, BlueBubbles, IRC, Matrix, Microsoft Teams, Feishu, LINE, Mattermost, Nextcloud Talk, Nostr, Synology Chat, Tlon, Twitch, Zalo, Zalo Personal, and WebChat
- Codex app-server runtime as the recommended built-in agent path
- `gpt-5.4` as the default model in one-click local setup
- Codex thread lifecycle support: `thread/start`, `thread/read`, `thread/fork`, `thread/compact/start`
- Codex review and planning support: `review/start`, plan events, structured item lifecycle events
- Codex interactive control loops routed through OpenClaw: approvals, permission requests, file-change requests, tool questions, and MCP elicitation
- Dynamic OpenClaw tool bridge into Codex with namespaced `openclaw_*` tools
- Codex-native skills support with OpenClaw-managed skill roots and skill syncing
- Structured event streaming into the OpenClaw control plane
- Streaming and chunking for long responses
- Multi-agent routing for isolated sessions per workspace, sender, or channel
- Subscription auth and API-key auth flows, with one-click focusing on Codex/OpenAI
- Sessions: direct chats collapse into shared `main` by default; groups and channels are isolated
- Group chat support with mention-based activation
- Media support for images, audio, video, and documents
- Optional voice note transcription and media understanding hooks
- Web Control UI, WebChat, TUI, and macOS menu bar app
- iOS node with pairing, Canvas, camera, screen recording, location, and voice features
- Android node with pairing, chat sessions, voice tab, Canvas/camera, notifications, contacts/calendar, motion, photos, and SMS commands
- Browser automation, Chrome extension support, and browser takeover flows
- Cron, hooks, webhooks, and background automation

<Note>
CodexPlusClaw treats **OpenClaw as the shell** and **Codex as the brain**. If you are looking for the normal local happy path, use `openclaw setup --one-click`.
</Note>
