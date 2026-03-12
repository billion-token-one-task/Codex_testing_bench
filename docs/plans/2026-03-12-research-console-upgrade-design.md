# Research Console Mission Control Upgrade — Design

Date: 2026-03-12  
Author: Dewey (frontend lead subagent)

## 1. Context
- Current research console already has Campaigns / Live / Runs / Compare / Artifacts / Research / Run Detail pages, but each page still feels like a thin ledger.  
- Control plane emits rich live snapshots but the UI only consumes a subset, so “live” feels delayed and disconnected from raw Codex artifacts.  
- User expectation mirrors TokenMart’s design language (see `designsystem.md`): editorial tone, industrial typography, telemetry rails, evidence desks, and clear notion of status/route/pressure.  
- Objective: turn console into default mission-control surface, make live runs observable in real time, give compare/research desks that read like paper-ready evidence, and keep operations (prepare/run/grade/report) at fingertip.

## 2. Goals & Success Criteria
1. **Realtime coupling**: Live runs must update within seconds for message/tool/patch/personality/token events. Operator should see runway warnings, stall detection, and attempt logs without leaving console.  
2. **Operational dashboards**: Campaigns and Live pages should feel like mission-control, with status strip + rail, not plain tables.  
3. **War room run detail**: Each run detail view must include timeline, message feed, tool/patch rails, command ledger, mechanism rail, token telemetry, and evidence dock.  
4. **Compare workbench**: Provide 2×2 matrix, same-task matrix, delta tables, lexical/tool/personality deltas and direct links to original run evidence.  
5. **Artifacts & research desks**: Archives should highlight classification (observed/inferred/estimated), artifact roles, preview/tail, and quick access to reference docs (hypotheses, observability contract, etc.).  
6. **Design-system fidelity**: reinforce TokenMart aesthetic—condensed display headlines, mono metadata, route strips, scanline/texture background, evidence tapes, signal badges.

## 3. Architecture & Data Flow
- **Control plane** continues to serve HTTP REST + SSE. We extend `/api/runs/:id/stream` to allow event-type filtering, exposing `run.message.appended`, `run.tool.appended`, etc.  
- Add `event_bus`, `run_observer`, and `artifact_watch` modules to aggregate append-only JSONL changes.  
- Add `LiveRunSnapshot` enhancements (activity heat, personality fallback, instruction stacks, focus text, warnings, derived telemetry).  
- Frontend `store.ts` gets a `useRunStream` hook + event bucket aggregator; `Run Detail` subscribes to run-scoped SSE streams for timeline rails.  
- `useWorkspaceIndex`/`useLiveRuns` continue polling but auto-refresh when SSE indicates relevant update.  
- Page components consume `operational summary` endpoints for campaign/run-level stats.

## 4. Page Designs
### Shell
- Left rail: brand kicker + nav stack + workspace summary + campaign pulse + action launcher.  
- Status strip: campaign ID, workspace refresh, live run count, signal totals, last activity, event mix, warnings.  
- Right rail: active run rail (cards) + structured events + process console (stdout/stderr).  
- Background: accent bar + scanline overlay per design system.

### Campaigns
- Layout: ledger (left), operational dossier (center), pulse rail (right).  
- Dossier includes metrics (benchmark, sample, cohorts, tokens, live telemetry), quick action chips (bootstrap/run/grade/report), warnings, live run cards, mechanism highlights, solver/grading/task/personality distribution chips.  
- Artifact rail: toggle between campaign reports/datasets with preview.  
- Event rails show recent campaign/run events filtered to selection.

### Live
- Mission status panel summarizing active campaign stats, heat mix, warnings.  
- Parallel slot grid of RunCards with telemetry chips (tokens/min, tools/min, heat, focus).  
- Focused run spotlight: mini war room showing event counts, message/tool/patch/mechanism rails, attempt log tail, latest message preview, warnings.  
- Live rails for recent messages, tools, patches, mechanisms, and process logs.  
- Process control section for stop/replay actions.

### Runs
- Ledger view with filters (model/personality/task class/status).  
- Table + card toggles.  
- Cohort grouping and anomaly grouping.  
- Quick actions: open war room, replay, show artifacts.  
- Embedded charts for solver/grading distribution.

### Run Detail (War Room)
- Top: identity strip + progress telemetry + warnings tape.  
- Left column: timeline rail, message stream (with tone badges), verification/bridge counters.  
- Center: tool rail, patch rail, command ledger (with stdout/stderr preview).  
- Right: mechanism rail (personality/instruction/skill/harness friction), token/turn strip, evidence dock (run-evidence, attempt log, raw JSONL).  
- Provide filters for event lanes, allow quick switch between attempts.

### Compare
- Top: campaign selector + metric cards.  
- Quadrant board summarizing cohorts.  
- Pair delta highlights + same-task 2×2 matrix (with RunCards).  
- Task-specific focus grid (“most verbose”, “most tool-dense”, etc.).  
- Phrase/tool/personality delta tables.  
- Research reading guides embedded as textual panels.  
- Links jump to Run Detail war rooms.

### Artifacts
- Scope tabs (campaign/run).  
- Artifact classification grid (role/format/preview).  
- Artifact ledger grouped by role with preview pane + tail snippet.  
- Operational dossier for selected campaign/run showing live warnings, latest reports/datasets, metrics.  
- Observability footnotes clarifying classification layer.

### Research
- Hypothesis board (H1–H6), evidence status grid, task-class lens, personality/skill mechanism tables.  
- Methods lens linking to observability contract + probe taxonomy.  
- Mechanism event rail plus methods appendix artifact viewer.

## 5. Operations & Controls
- Action launcher refactor: quick actions (context-aware) + advanced launchpad (preset/prompt inputs).  
- Process console shows running CLI commands with stop buttons + status badges.  
- Warnings aggregated from control plane (stalls, fallback, infra failures) displayed in warning tape components.

## 6. Testing & Verification
- Backend: `cargo test -p codex-bench-control-plane`, plus manual SSE smoke (curl run stream).  
- Frontend: `npm run build` + Playwright smoke snapshot (optional).  
- Manual e2e: start control plane on test port, open console, ensure Live/Campaigns/RunDetail/Compare load and update when tailing active runs.

## 7. Open Questions
- Need to confirm if we should persist user-selected run/campaign in localStorage for continuity.  
- Confirm whether action launcher should guard potentially destructive commands (e.g., confirm before stopping run).  
- Consider dark-theme variant later; current scope is single theme consistent with design system.

