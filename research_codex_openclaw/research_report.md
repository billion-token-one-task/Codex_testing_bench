# Codex <-> OpenClaw Harness Swap Research Report

## Scope

Question:
Can OpenClaw keep its local gateway, integrations, channels, tooling, and device/runtime features while replacing its inner agent harness with Codex in a way that stays maintainable as both upstream projects evolve?

Repositories analyzed locally on 2026-03-10:

- Codex: `openai/codex` at commit `1165a16e6ffad719e8f852900fd7ff438ec88fae`
- OpenClaw: `openclaw/openclaw` at commit `5decb00e9d2ae36c948e4cc83e42957e83108950`

## Executive Summary

Yes, it is technically possible, but not as a drop-in swap.

The best long-term architecture is **not**:

- OpenClaw calls `codex exec` as a thin subprocess and hopes it behaves like the current runtime
- or vendoring Codex Rust internals directly into OpenClaw's TypeScript runtime

The best long-term architecture is:

- **OpenClaw remains the outer local platform**
- **Codex becomes the agent engine behind a process/protocol boundary**
- **OpenClaw exposes its integrations/tools to Codex through an adapter layer**
- **session/thread state is mapped explicitly instead of trying to preserve Pi session internals**

## Main Findings

### 1. Codex is reusable, but its cleanest boundary is process/protocol, not direct TS embedding

Codex is more than a CLI.

- `codex-core` is a real library crate: `repos/codex/codex-rs/core/src/lib.rs`
- thread/session lifecycle is owned by `ThreadManager` and `CodexThread`: `repos/codex/codex-rs/core/src/thread_manager.rs`, `repos/codex/codex-rs/core/src/codex_thread.rs`
- the protocol is explicit (`Submission`, `Op`, events, turn state): `repos/codex/codex-rs/protocol/src/protocol.rs`
- Codex also exposes process-facing surfaces:
  - CLI `exec`: `repos/codex/codex-rs/cli/src/main.rs`
  - TypeScript SDK that wraps the CLI over JSONL stdio: `repos/codex/sdk/typescript/README.md`
  - MCP server: `repos/codex/codex-rs/mcp-server/src/message_processor.rs`
  - experimental app-server protocol: `repos/codex/codex-rs/app-server-protocol/src/protocol/v2.rs`

Implication:
Directly embedding Codex inside OpenClaw's TypeScript runtime would be high-friction and brittle. A protocol boundary is much safer.

### 2. OpenClaw's outer platform is separable from its inner agent runtime

OpenClaw's gateway, channels, and UI do not themselves implement the model loop.

- gateway request contracts live in `repos/openclaw/src/gateway/server-methods/types.ts`
- gateway agent entrypoint hands off to the agent system: `repos/openclaw/src/gateway/server-methods/agent.ts`
- the control UI talks to the gateway protocol rather than directly to Pi internals: `repos/openclaw/ui/src/ui/gateway.ts`

Implication:
You can preserve OpenClaw's main value, its local multi-channel control plane and integrations, while changing the engine behind it.

### 3. OpenClaw is currently deeply coupled to Pi for embedded runs

The current "real" embedded runtime is Pi-based.

- core run loop: `repos/openclaw/src/agents/pi-embedded-runner/run.ts`
- single-attempt runtime/session/tool setup: `repos/openclaw/src/agents/pi-embedded-runner/run/attempt.ts`
- streaming/event adaptation: `repos/openclaw/src/agents/pi-embedded-subscribe.ts`
- Pi integration doc: `repos/openclaw/docs/pi.md`
- Pi deps are still present in the manifest: `repos/openclaw/package.json`

Implication:
Replacing `runEmbeddedPiAgent()` with Codex without first introducing an engine abstraction would be painful and fragile.

### 4. OpenClaw already has a Codex path, but not the one you want

OpenClaw already supports a `codex-cli` backend.

- backend config: `repos/openclaw/src/agents/cli-backends.ts`
- CLI runner: `repos/openclaw/src/agents/cli-runner.ts`
- runtime selection between CLI backends and embedded Pi: `repos/openclaw/src/commands/agent.ts`, `repos/openclaw/src/auto-reply/reply/agent-runner-execution.ts`

Critical limitation:

- `runCliAgent()` injects: `Tools are disabled in this session. Do not call tools.`

Implication:
OpenClaw already knows how to delegate to Codex CLI, but only in a reduced "text-only backend" mode. That does not preserve OpenClaw's best feature set.

### 5. The real reuse point is OpenClaw's tool catalog, not its Pi tool adapter

OpenClaw's tool inventory is valuable and fairly modular.

- OpenClaw tool catalog assembly: `repos/openclaw/src/agents/openclaw-tools.ts`
- Pi-specific tool assembly/normalization: `repos/openclaw/src/agents/pi-tools.ts`
- Pi adapter layer: `repos/openclaw/src/agents/pi-tool-definition-adapter.ts`

Implication:
The tool catalog can survive a harness swap, but the Pi-specific adapter should be replaced with a Codex-facing adapter.

### 6. Session persistence is one of the hardest migration problems

OpenClaw session operations are gateway-owned, but transcript persistence assumes Pi semantics.

- gateway session handlers: `repos/openclaw/src/gateway/server-methods/sessions.ts`
- transcript/session persistence logic relies on Pi session structures: `repos/openclaw/src/config/sessions/transcript.ts`

Implication:
Trying to keep Pi session files while swapping in Codex is the wrong move. A mapping layer from OpenClaw session keys to Codex thread ids is cleaner.

## Feasibility Judgment

### Is it possible?

Yes.

### Is it a drop-in replacement?

No.

### Is there a maintainable path?

Yes, if the integration boundary is designed correctly.

## Recommended Architecture

### Recommended boundary

`OpenClaw Gateway / Channels / UI / Nodes / Local Integrations`

`<->`

`Codex Engine Adapter`

`<->`

`Codex process boundary (prefer app-server or structured exec/client boundary)`

### What the adapter should own

1. Session mapping
   - OpenClaw `sessionKey/sessionId` -> Codex `thread_id`

2. Tool bridge
   - expose OpenClaw tools to Codex through a Codex-compatible adapter
   - best option: present OpenClaw tools/resources as MCP servers or a Codex-facing tool shim

3. Event translation
   - map Codex lifecycle/tool/output events into OpenClaw's gateway event model

4. Policy translation
   - sandbox, approvals, and runtime permissions need explicit mapping

5. Prompt/context injection
   - OpenClaw persona, channel context, routing hints, and workspace context still need to be assembled outside Codex

## Best Integration Options

### Option A. Keep using `codex-cli` as a subprocess and grow that path

Pros:

- fastest path
- OpenClaw already has this codepath

Cons:

- currently text-only in practice
- poor tool/runtime parity
- likely awkward approvals and streaming behavior
- highest chance of "almost works" forever

Verdict:
Good for experiments, not the final architecture.

### Option B. Build a first-class Codex engine adapter over a process/protocol boundary

Pros:

- best balance of maintainability and power
- allows OpenClaw to stay TS and Codex to stay Rust
- upstream changes are isolated to a narrower adapter layer

Cons:

- requires real integration work
- must design event/session/tool mapping carefully

Verdict:
Best overall choice.

### Option C. Re-implement OpenClaw's runtime inside Codex directly

Pros:

- deepest integration

Cons:

- very high coupling
- hardest to keep synced with both upstreams
- likely to fork behavior rather than compose it

Verdict:
Not recommended.

## Suggested Build Order

1. Introduce an engine abstraction in OpenClaw
   - hide `runEmbeddedPiAgent()` and `runCliAgent()` behind a common interface

2. Add a `CodexEngineRunner`
   - start with the existing Codex CLI path as scaffolding

3. Replace the "tools disabled" path
   - expose OpenClaw tools via a Codex-facing bridge

4. Add session/thread mapping persistence
   - maintain Codex thread ids in OpenClaw session metadata

5. Add event translation
   - map Codex tool/lifecycle events into gateway/chat consumers

6. Only then migrate selected agents/routes to Codex by default

## Biggest Risks

- OpenClaw's Pi assumptions are spread across transcript/session code
- approvals and sandbox semantics do not match 1:1
- tool invocation contracts differ
- streaming expectations in gateway/chat flows will need parity work
- direct embedding would increase cross-language upgrade pain

## Recommendation

Build **CodexClaw** as:

- OpenClaw outside
- Codex inside
- explicit adapter in the middle

Do **not** make Codex a hidden subprocess behind the current Pi assumptions.
Do **not** try to preserve Pi transcript internals.
Do **not** vendor Codex core directly into OpenClaw's TypeScript runtime.

The most future-proof version is:

- OpenClaw continues owning integrations, local execution surfaces, routing, session identity, and UI
- Codex owns planning, tool orchestration, latest-model support, and sub-agent harness behavior
- a narrow adapter translates tools, sessions, and events between them

## Upstream Sources

- Codex repo: https://github.com/openai/codex
- Codex docs: https://developers.openai.com/codex
- OpenClaw repo: https://github.com/openclaw/openclaw
- OpenClaw architecture docs: https://docs.openclaw.ai/concepts/architecture
- OpenClaw agent docs: https://docs.openclaw.ai/concepts/agent
