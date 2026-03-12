# Research Console Mission Control Upgrade Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship the mission-control redesign for the research console so every page (Shell, Campaigns, Live, Runs, Compare, Artifacts, Research, Run Detail) feels tightly coupled to the live Codex harness and produces paper-ready evidence.

**Architecture:** Control plane exposes richer operational summaries and per-run SSE streams; React console consumes them with high-density TokenMart styling, live rails, spotlight war rooms, and research desks. Styling sticks to condensed display sans + mono rails, heavy structure, and warning tapes.

**Tech Stack:** Rust (Axum) for control plane; React + TypeScript + Vite for frontend; CSS modules via `global.css` for design system.

---

### Task 1: Control Plane Event Bus & Live Snapshot Enrichment

**Files:**
- Modify: `bench/crates/codex-bench-control-plane/src/api.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/live.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/index.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/processes.rs`
- Modify: `bench/crates/codex-bench-control-plane/src/server.rs`

**Steps:**
1. Add `event_bus`, `run_observer`, and `artifact_watch` helpers: maintain workspace cache, watch key artifact JSONL files per active run, emit UiEvents (`run.message.appended`, `run.tool.appended`, `run.patch.appended`, `run.command.appended`, `run.personality.appended`, `run.skill.appended`, `run.token.appended`, `run.mechanism.appended`, `run.timeline.appended`, `run.phase.changed`, `run.focus.changed`, `run.warning.appended`).
2. Enhance `LiveRunSnapshot` extraction (activity heat, warnings, focus text, instruction layers, personality fallback counts, tokens/min, bursts/min). Update builder functions accordingly.
3. Update `/api/runs/:id/stream` to accept optional `event_types` query, hooking into new file tail watchers. Build SSE stream using `event_bus` filter.
4. Expand campaign/run operational summaries with live telemetry totals, warning counts, heat counts, active cohorts/instances, latest previews, latest reports/datasets, solver/grading/task/personality/tool distributions.
5. Ensure workspace cache refresh job reuses new watchers and events. Update unit tests or add new ones for builder functions (if existing tests, extend; otherwise add targeted sanity checks).
6. Verify with `cargo test -p codex-bench-control-plane` and manual `curl http://127.0.0.1:4274/api/runs/<id>/stream`.

### Task 2: Shell & Global Layout Enhancements

**Files:**
- Modify: `apps/research-console/src/components/Shell.tsx`
- Modify: `apps/research-console/src/styles/global.css`

**Steps:**
1. Redesign shell: left rail (brand block, nav, workspace summary cards, campaign pulse board, quick action deck), scanline background, accent bars per design system.
2. Add status strip metrics (latest campaign ID, workspace refresh timestamp, live run counts, token signal, event mix, warnings). Use mono metadata + condensed display headings.
3. Right rail: live run rail showing top 4 runs, structured event feed, process console with stop buttons. Ensure cards handle `LiveRunSnapshot` vs `RunIndexEntry` gracefully.
4. Update CSS with new layout grids, accent textures, warning tapes, mono label styles, hover/focus states.
5. `npm run build` to ensure layout compiles; optionally run Playwright smoke.

### Task 3: Campaigns Page — Ledger + Operational Dossier

**Files:**
- Modify: `apps/research-console/src/pages/CampaignsPage.tsx`

**Steps:**
1. Build campaign ledger cards with statuses, benchmark/stage meta, sample/cohort metrics, clickable selection.
2. Operational dossier panel: metrics grid (benchmark, sample, cohorts, tokens, tool/cmd counts, live telemetry), warning tape, quick action chips invoking API actions (bootstrap/run/grade/report). Handle spinner/error states.
3. Add run surface panel (live run cards + completed summary) and mechanism highlights (top tools/routes, solver/grading/task/personality distributions). Use `campaignOperationalSummary` data.
4. Pulse rail showing recent campaign/run events. Provide artifact preview toggles (reports/datasets) referencing `ArtifactViewer`.
5. Update CSS for ledger boards, chips, warning tapes.
6. `npm run build` for regression.

### Task 4: Live Mission Control

**Files:**
- Modify: `apps/research-console/src/pages/LivePage.tsx`

**Steps:**
1. Add mission status panel summarizing active campaign data (heat mix, warnings, tokens, focus, bridging). Use `useCampaignOperationalSummary`.
2. Implement parallel slot grid that renders `LiveRunSnapshot` telemetry cards with tokens/min, tools/min, heat badges, route/focus metadata. Allow selecting run to spotlight.
3. Implement run spotlight panel: metrics grid, warning tapes, event count cards, event rails (message/tool/patch/mechanism) sourced via `useRunEventBuckets` + `useRunStream`. Show attempt-log tail with `useArtifactTail`.
4. Add live rails for process outputs, structured events, message/tool/patch/mechanism lists. Use `EventRail` component.
5. Keep quick operations (stop process, replay) accessible with new button row.
6. Build + manual UI check.

### Task 5: Run Detail War Room

**Files:**
- Modify: `apps/research-console/src/pages/RunDetailPage.tsx`

**Steps:**
1. Create operational snapshot panel with identity strip, telemetry, warnings, event table counts, latest reports/datasets, artifact type counts.
2. Add timeline rail + message stream showing text previews with tone badges, bridging/verification scores, and event metadata. Use `useRunEventBuckets` with SSE.
3. Add tool rail, patch rail, command ledger (with stdout/stderr preview). Provide filters by event type.
4. Add mechanism rail summarizing personality/instruction/skill events, tokens, harness friction. Add token/turn strip (charts or metric chips).
5. Evidence dock listing `run-evidence`, `attempt-log`, raw JSONL with preview + tail.
6. Update CSS for war room layout. Build + manual check.

### Task 6: Compare Workbench

**Files:**
- Modify: `apps/research-console/src/pages/ComparePage.tsx`

**Steps:**
1. Build 2×2 quadrant board summarizing cohort counts. Use `campaignRunRows` to compute counts.
2. Add same-task board with RunCards grouped per instance + focus grid (most verbose/tool-dense/bridge/verification). Provide story panels referencing lexical/tool/personality deltas.
3. Add phrase/tool/personality delta tables and research reading guide panels. Emphasize ability to click to run detail.
4. Ensure selection states + dataset loads are memoized.
5. Build + manual check.

### Task 7: Artifacts & Research Desks

**Files:**
- Modify: `apps/research-console/src/pages/ArtifactsPage.tsx`
- Modify: `apps/research-console/src/pages/ResearchPage.tsx`

**Steps:**
1. `Artifacts`: support campaign/run scopes, artifact grouping by role, classification grids, preview panel + tail, operational dossier for campaign/run, scope toggles.  
2. `Research`: show hypothesis board, evidence status, task-class lens, mechanism tables, methods lens referencing contract/hypothesis docs, event rails.  
3. Provide references panel using `ArtifactViewer` for `model-personality-study`, `observability-contract`, `probe-taxonomy`.  
4. Build + manual check.

### Task 8: Store/API Enhancements

**Files:**
- Modify: `apps/research-console/src/lib/store.ts`
- Modify: `apps/research-console/src/lib/api.ts`

**Steps:**
1. Add `runStreamUrl` helper, `useRunStream` hook, event bucket aggregator deduping by run, support event-type filtering.  
2. Extend `useWorkspaceIndex`, `useLiveRuns`, `useRunDetail`, `useRunOperationalSummary`, `useCampaignOperationalSummary` to auto-refresh on new SSE events.  
3. Add selectors for artifact tail, dataset fetch, run selection persistence if needed.  
4. Build + manual check.

### Task 9: Styling & Components

**Files:**
- Modify: `apps/research-console/src/styles/global.css`
- Modify: shared components (`Panel`, `MetricCard`, `EventRail`, `RunCard`, etc.) as needed.

**Steps:**
1. Introduce CSS variables for colors/spacing per design system.  
2. Add background textures (scanlines, barcode, noise), warning tapes, rails, chips, focus notes.  
3. Ensure responsive breakpoints for mission control layout.  
4. Keep typography consistent: condensed display headings, mono labels, body sans paragraphs.  
5. Build + visually inspect.

### Task 10: Verification & Smoke Tests

**Files:**
- N/A (commands)

**Steps:**
1. `cargo test -p codex-bench-control-plane`.  
2. `npm run build` inside `apps/research-console`.  
3. Launch control plane on dev port (`cargo run -p codex-bench-control-plane -- --repo-root ../ --port 4274`).  
4. `npm run dev` (or `vite preview`) to inspect UI.  
5. Optional Playwright smoke screenshot `npm run test:e2e` if available.

---

Plan saved to `docs/plans/2026-03-12-research-console-upgrade-plan.md`.
