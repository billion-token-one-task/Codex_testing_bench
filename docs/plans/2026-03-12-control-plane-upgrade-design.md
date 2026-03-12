# Control Plane Upgrade Design (2026-03-12)

## Context
- The bench control plane currently exposes workspace/campaign/run APIs, a basic SSE stream, and managed process registry.
- Real-time fidelity is limited: run-level data largely comes from periodic rescans, live snapshots lack tool/message granularity, and artifact append events are only partially streamed.
- The research console now needs a mission-control grade backend capable of powering live run war rooms, campaign dossiers, and compare workbenches without polling-heavy hacks.

## Goals
1. Turn the control plane into a **research data bus** with dual tracks: static scans + streaming delta feeds.
2. Provide **richer live snapshots** per run (phase, focus, telemetry, mechanism signals, warnings) and surface them through `/api/live/runs`, `/api/runs/:id/operational-summary`, and `/api/runs/:id/stream` without breaking existing structures.
3. Add an **artifact watcher** that tails key JSONL/CSV/text artifacts and emits structured SSE events (message/tool/patch/personality/skill/token/command).
4. Strengthen **workspace/campaign caches** so repeated scans become incremental, and campaign operational summaries expose aggregated heat, warnings, and latest artifacts.
5. Keep existing endpoints/states backward compatible while augmenting payloads with new fields the frontend can light up immediately.

## Approaches Considered
1. **Polling-only refresh** – keep current model, optimise scan frequency, and have the frontend poll smaller endpoints. *Cons*: still laggy, high IO, no live rails. *Rejected*.
2. **Partial streaming additive** – stream only raw agent events and let frontend derive everything. *Cons*: frontend duplication of backend logic, high bandwidth. *Rejected*.
3. **Full data bus hybrid (Chosen)** – backend derives live snapshots, streams curated events, maintains caches, and exposes enriched summaries while preserving existing schema. *Pros*: authoritative backend, lower frontend complexity, easier future benchmarks.*

## High-level Architecture
1. **Event Bus**
   - Replace single broadcast channel with typed `UiEvent` router.
   - Build `event_bus` module responsible for
     - reading process/stdout events
     - watching artifact files via `notify`-style poll loop
     - emitting structured SSE events (campaign/run updates, stream deltas, warnings).
   - Provide `/api/events` SSE for global feed plus `/api/runs/:id/stream` for fine-grained run feeds with optional `event_types` filters.

2. **Live Snapshot Manager**
   - Maintain `LiveSnapshotMap` keyed by `run_id` with `LiveRunSnapshot` containing
     - identity (campaign/run/cohort/model/personality/task)
     - telemetry (tokens, visible tokens/km metrics)
     - progress counts (messages, tools, commands, patch events, verification events, raw event count)
     - mechanism snapshot (personality effectiveness, instruction layers, compaction, harness friction, skills)
     - status (phase, activity heat, warnings, focus) and `latest_*` fields (message/tool/patch/command).
   - Update snapshot whenever artifact watcher detects appended rows or token snapshots.
   - Use snapshot both for `/api/live/runs` and for campaign summaries.

3. **Artifact Watcher**
   - Poll/stream appended rows for `message-metrics`, `tool-events`, `command-events`, `patch-events`, `personality-events`, `skill-events`, `codex-probe-events`, `token-snapshots`, `raw-agent-events`, `patch-chain`, `skill-mechanism`, `verbosity-tool-coupling`.
   - Convert each appended row into semantically named UiEvents (e.g., `run.tool.appended`, `run.patch.appended`, `run.mechanism.appended`).
   - Update live snapshot progress counts + latest previews.

4. **Workspace Cache + Operational Summaries**
   - Introduce `workspace_cache` storing last scan result, refreshed on interval or `workspace.updated` triggers.
   - For campaign operational summary:
     - include aggregated counts for solver/grading status, cohorts, task classes, models, personalities, tool routes/names, heat buckets, warnings, focus samples, message previews.
     - include `active_live_runs` (current snapshots) + `latest_reports/datasets` from detail scan.
   - For run operational summary:
     - include derived artifact/event counts, warnings, latest artifacts, and `live_snapshot` embed.

5. **API Surface Enhancements (Back compat)**
   - Existing endpoints keep returning previous fields but gain additive ones; run detail includes new tables/previews for normalized rows.
   - `/api/runs/:id/stream` supports `event_types` query for targeted SSE; defaults to everything.
   - `/api/events` includes new event types `run.phase.changed`, `run.focus.changed`, `run.warning.appended`, `campaign.summary.updated`, etc.
   - `ActionResponse` unchanged; new events simply inform front-end.

## Data Flow
```mermaid
flowchart LR
    subgraph Watchers
        A[Artifact pollers]
        B[Process stdout]
        C[Workspace refresh]
    end
    A -->|rows| D(EventBus)
    B --> D
    C --> D
    D -->|UiEvent SSE| E[/api/events]
    D -->|filtered SSE| F[/api/runs/:id/stream]
    D -->|snapshot updates| G(LiveSnapshotMap)
    G -->|JSON| H(/api/live/runs)
    G --> H2(/api/runs/:id/operational-summary)
    G --> I(/api/campaigns/:id/operational-summary)
    H2 --> J(Research Console)
    I --> J
```

## API Notes
- Maintain existing JSON structure, add fields at the end/object-level for new data to avoid front-end breakage.
- Provide `event_types` query param for run stream; default to all known types if not provided.
- Document new SSE types and semantics for frontend instrumentation.

## Implementation Steps
1. Backend foundation
   - Add event bus module, restructure SSE stream code, and create artifact watchers.
   - Extend live snapshot builder to include mechanism + telemetry fields.
2. API enrichment
   - Update campaign/run operational summary endpoints + run detail to embed new structures.
   - Add run stream filter support.
3. Integration
   - Hook watchers into workspace poll loop and ensure cache invalidation works.
4. Frontend alignment (handled by separate effort) – ensure war room surfaces subscribe to new fields.

## Testing Plan
- `cargo test -p codex-bench-control-plane` for backend unit coverage.
- Manual run:
  1. Launch control plane (`cargo run …`).
  2. Trigger `run` on a sample campaign; verify `/api/live/runs` returns enriched snapshots.
  3. Observe `/api/events` SSE for new event types when artifacts append.
  4. Hit `/api/runs/:id/stream?event_types=run.tool.appended,run.patch.appended` to confirm filtering.
  5. Open research console pointing at new control plane, ensure Live/Campaigns/Run Detail surfaces light up without errors.

