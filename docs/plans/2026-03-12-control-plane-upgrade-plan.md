# Control Plane Upgrade Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Turn the control plane into a real-time research data bus with richer live snapshots, artifact-driven SSE events, and stronger operational summaries without breaking existing API contracts.

**Architecture:** Extend the Axum control plane with an event bus that watches artifacts/processes, keeps a live snapshot map, and feeds enriched REST + SSE endpoints. Operational summaries embed live data while staying backward compatible.

**Tech Stack:** Rust (Tokio/Axum), serde_json, broadcast channels, filesystem watchers/pollers, CSV/JSONL readers, cargo test suite.

---

### Task 1: Event Bus & Artifact Watcher Foundation

**Files:**
- Modify: `bench/crates/codex-bench-control-plane/src/server.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/processes.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/api.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/live.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/lib.rs` (if new modules needed)

**Step 1:** Introduce a dedicated event bus module or structs (within `live.rs` or new file) that expose helpers to publish UiEvents for run/campaign/workspace updates.

**Step 2:** Extend `ProcessRegistry` to emit richer `process.updated` events (including exit status) and wire stdout/stderr lines with timestamps + process kind.

**Step 3:** Add artifact watcher loop in `server.rs` (spawned alongside workspace poller) that iterates over active runs' attempt dirs, tracks file cursors for JSONL/CSV/text artifacts, and sends UiEvents using helpers from `live.rs`.

**Step 4:** Update `/api/events` SSE handler in `api.rs` to subscribe to the event bus and forward new event types (`run.phase.changed`, `run.focus.changed`, `run.warning.appended`, `campaign.summary.updated`, `run.mechanism.appended`, etc.).

**Step 5:** Run `cargo test -p codex-bench-control-plane` to ensure event bus changes compile and basic tests pass.

### Task 2: Live Snapshot Expansion

**Files:**
- Modify: `bench/crates/codex-bench-control-plane/src/live.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/index.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/api.rs`

**Step 1:** Enhance `LiveRunSnapshot` to include telemetry (tokens/min, message/tool ratios), mechanism snapshot (personality requested/effective, fallback counts, compaction, harness friction, instruction layers, skill info), `latest_*` previews, warnings, activity heat, and progress counters (message/tool/command/patch/verification/raw counts + artifact row count).

**Step 2:** Update snapshot builder (`build_live_run_snapshot`) to read additional artifacts (`message-metrics.jsonl`, `tool-events.jsonl`, `command-events.jsonl`, `patch-events.jsonl`, `personality-events.jsonl`, `skill-events.jsonl`, `token-snapshots.jsonl`, `verbosity-tool-coupling.jsonl`, `raw-agent-events.jsonl`) and fill new fields. Handle missing files gracefully.

**Step 3:** Ensure live snapshot map gets mutated whenever watcher emits events, and expose `/api/live/runs` plus `/api/live/runs/{run_id}` endpoints returning the new structure.

**Step 4:** Extend `/api/runs/{run_id}/stream` to accept `event_types` query filtering; default to all known types. Ensure SSE payloads include timestamps and canonical `run_id`/`campaign_id` for filtering.

**Step 5:** Run `cargo test -p codex-bench-control-plane`.

### Task 3: Workspace Cache & Operational Summaries

**Files:**
- Modify: `bench/crates/codex-bench-control-plane/src/api.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/index.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/live.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/server.rs`

**Step 1:** Replace ad-hoc workspace scans with `workspace_cache` (RwLock<Option<WorkspaceIndex>>). Populate on startup and refresh in the poll loop plus whenever `workspace.updated` UiEvent is triggered.

**Step 2:** Update `scan_campaign_detail` and workspace summary builders to compute aggregated counts (solver/grading status, cohorts, task classes, models, personalities, tool routes/names, heat, warnings) and include latest report/dataset descriptors.

**Step 3:** Make `/api/campaigns/{id}/operational-summary` embed live snapshots, aggregated metrics, unresolved infra failure counts, active cohorts/instances, focus samples, message previews, and warnings.

**Step 4:** Make `/api/runs/{id}/operational-summary` include live snapshot, latest reports/datasets for that run, artifact type counts, event table counts, warnings, and mechanism signals.

**Step 5:** Run `cargo test -p codex-bench-control-plane`.

### Task 4: Run Detail & Stream Payload Additions

**Files:**
- Modify: `bench/crates/codex-bench-control-plane/src/api.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/index.rs`

**Step 1:** Extend `RunDetailResponse` to include `live_snapshot`, `timeline` rows for new event types, `tables` keyed by normalized artifacts, and `previews` for text artifacts.

**Step 2:** Assemble timeline rows from artifacts and raw events using helpers in `live.rs`, ensuring timestamps + lane/kind fields for the frontend war room.

**Step 3:** Include `attempt_artifacts` role metadata (role/scope/format/previewable) so the artifacts page can highlight source-of-truth classification.

**Step 4:** Update `/api/runs/{run_id}/stream` SSE loop to emit `run.summary.updated` or `run.timeline.appended` when run detail tables change, and support `event_types` filter logic implemented earlier.

**Step 5:** Run `cargo test -p codex-bench-control-plane`.

### Task 5: Documentation & Smoke Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/getting-started.md`
- Modify: `docs/research/research-console.md`

**Step 1:** Document the enriched control plane capabilities (event bus, live snapshots, SSE event taxonomy, API additions).

**Step 2:** Run `cargo test -p codex-bench-control-plane` one final time plus `npm run build` under `apps/research-console` to ensure API changes remain compatible with the frontend build.

**Step 3:** Manual smoke:
  - `cargo run -p codex-bench-control-plane -- --port 4274` (separate port to avoid interrupting live run) and ensure it prints listen info.
  - `curl http://127.0.0.1:4274/api/workspace/index | jq '.summary'` to confirm cache fields.
  - `curl http://127.0.0.1:4274/api/runs/<run_id>/stream?event_types=run.tool.appended` and observe events.
  - Launch research console pointing at new control plane, verify Live/Campaigns/Run Detail render new data without console errors.

**Step 4:** Commit all changes with message `feat: upgrade control plane data bus` once review is green.

