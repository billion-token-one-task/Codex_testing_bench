---
summary: "Model and auth provider overview, with CodexPlusClaw defaults first and advanced provider flows second."
read_when:
  - You need a provider-by-provider setup reference
  - You want to understand CodexPlusClaw auth and model defaults
title: "Model Providers"
---

# Model providers

This page covers **model and auth providers**. For the default local CodexPlusClaw path, the important part is simple:

- `openclaw setup --one-click`
- Codex CLI is installed or upgraded automatically
- Codex app-server becomes the agent runtime
- `gpt-5.4` becomes the default model unless you override it

Use this page when you want to understand the broader provider/auth surface, manual wizard flows, or advanced overrides.

## Quick rules

- One-click local bootstrap is **Codex-first**.
- The manual wizard is still available via `openclaw onboard` or `openclaw setup --wizard`.
- Model refs use `provider/model` when you are configuring gateway model catalogs directly.
- If you set `agents.defaults.models`, it becomes the allowlist.
- Useful CLI helpers:
  - `openclaw setup --one-click`
  - `openclaw setup --wizard`
  - `openclaw models list`
  - `openclaw models set <provider/model>`
  - `openclaw models auth ...`

## Recommended default

For a local CodexPlusClaw install, prefer:

- Provider/runtime: official Codex CLI + app-server
- Auth: ChatGPT/Codex OAuth or OpenAI API key
- Model: `gpt-5.4`

Example default:

```json5
{
  agents: {
    defaults: {
      codex: {
        defaultModel: "gpt-5.4",
        provider: "openai",
      },
    },
  },
}
```

## What one-click supports directly

`openclaw setup --one-click` supports these auth choices:

- `openai-codex` — browser login / Codex OAuth
- `openai-api-key` — OpenAI API key
- `skip` — prepare everything except auth

Examples:

```bash
openclaw setup --one-click
openclaw setup --one-click --auth-choice openai-codex
openclaw setup --one-click --auth-choice openai-api-key --openai-api-key "$OPENAI_API_KEY"
```

## When to use the manual wizard

Use `openclaw onboard` or `openclaw setup --wizard` when you need:

- remote gateway onboarding
- non-default provider or auth flows
- custom provider endpoints
- advanced multi-provider gateway setup
- explicit non-interactive onboarding scripts

## Built-in providers

OpenClaw still documents and supports a broad provider surface for advanced configurations and manual setup. The important distinction is:

- **Codex app-server remains the built-in local runtime**
- other providers/auth flows remain available for manual setups, model catalogs, proxies, and compatibility scenarios

### OpenAI

- Provider: `openai`
- Auth: `OPENAI_API_KEY`
- Example model: `openai/gpt-5.4`
- One-click: `openclaw setup --one-click --auth-choice openai-api-key`

```json5
{
  agents: { defaults: { model: { primary: "openai/gpt-5.4" } } },
}
```

### OpenAI Codex

- Provider: `openai-codex`
- Auth: Codex / ChatGPT OAuth
- Example model: `openai-codex/gpt-5.4`
- Manual login: `openclaw models auth login --provider openai-codex`
- One-click default: `openclaw setup --one-click`

```json5
{
  agents: { defaults: { model: { primary: "openai-codex/gpt-5.4" } } },
}
```

### Anthropic

- Provider: `anthropic`
- Auth: `ANTHROPIC_API_KEY` or Anthropic setup-token
- Example model: `anthropic/claude-opus-4-6`
- Manual wizard: `openclaw onboard`
- Direct auth helpers:
  - `openclaw models auth paste-token --provider anthropic`
  - `openclaw models auth setup-token --provider anthropic`

Anthropic subscription auth is still documented for compatibility, but the cleanest supported production path is an Anthropic API key.

### Other built-in or proxy-style providers

OpenClaw also documents OpenRouter, Vercel AI Gateway, Cloudflare AI Gateway, MiniMax, Moonshot, Z.AI, Hugging Face, Mistral, Together, OpenCode, and other provider surfaces. Use the dedicated provider pages for exact auth flags and examples.

## API key rotation

- Supports generic provider rotation for selected providers.
- Configure multiple keys via:
  - `OPENCLAW_LIVE_<PROVIDER>_KEY`
  - `<PROVIDER>_API_KEYS`
  - `<PROVIDER>_API_KEY`
  - `<PROVIDER>_API_KEY_*`
- Requests only rotate on rate-limit style failures.

## Custom providers and advanced catalogs

Use `models.providers` (or `models.json`) to add:

- OpenAI-compatible endpoints
- Anthropic-compatible endpoints
- proxy gateways
- self-hosted or region-specific provider catalogs

See the dedicated reference:

- [Configuration reference](/gateway/configuration-reference#custom-providers-and-base-urls)
- [Providers index](/providers)

## Practical guidance

- For the easiest local install: stick with one-click, Codex auth, and `gpt-5.4`.
- For a remote or custom provider environment: use the manual wizard.
- For provider-specific auth after setup: use `openclaw models auth ...`.
- For cost/performance experimentation: set per-agent defaults or session overrides, but keep the main local runtime description aligned with CodexPlusClaw.
