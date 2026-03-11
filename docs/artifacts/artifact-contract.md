# Artifact Contract

## Goal

Define what each campaign and run should produce, what is meant to be GitHub-visible, and what stays local-only.

## Campaign-Level Artifacts

Expected campaign-level files:

- `campaign-manifest.json`
- `selected-dataset.json`
- `codex-architecture-map.json`
- `grounding-claims.json`
- `codex-unique-claims.json`
- `predictions.jsonl`
- `grader.json`
- `reports/report.txt`

These files describe:

- what was run
- which tasks were selected
- what Codex subsystems were under observation
- which claims were in scope
- what the aggregate evidence looked like

## Per-Run / Per-Attempt Artifacts

Expected attempt-level files:

- `prompt.txt`
- `environment-plan.json`
- `raw-agent-events.jsonl`
- `raw-diagnostics.jsonl`
- `codex-probe-events.jsonl`
- `lifecycle-events.jsonl`
- `token-snapshots.jsonl`
- `turn-metrics.jsonl`
- `command-events.jsonl`
- `tool-events.jsonl`
- `skill-events.jsonl`
- `patch-events.jsonl`
- `anomalies.jsonl`
- `patch.diff`
- `run-summary.json`
- `probe-events.jsonl`
- `probe-summary.json`
- `claim-evidence.json`
- `run-evidence.txt`
- `attempt-log.txt`
- `replay.json`

## Human-Readable Priority Order

If a human is triaging a run, the intended reading order is:

1. `run-summary.json`
2. `run-evidence.txt`
3. `attempt-log.txt`
4. `probe-summary.json`

The raw JSONL files are the source of truth, but they are not intended to be the first thing a researcher opens.

## Publishable vs Local-Only

Intended to be committed:

- campaign manifests
- selected dataset snapshots
- architecture maps
- claim catalogs
- `report.txt`
- `run-summary.json`
- `probe-summary.json`
- `claim-evidence.json`
- `run-evidence.txt`
- `attempt-log.txt`
- `replay.json`

Typically kept local-only:

- heavy warmed caches
- worktrees / workspaces
- raw JSONL streams unless explicitly curated
- bulky prompt and environment staging files
- transient patch/runtime files if they would overwhelm the repo

## Backfilling Rule

The reporting layer is allowed to backfill newer derived artifacts from older raw artifacts when:

- the raw event streams still exist
- the report schema has evolved
- the run summary is older than the current report/probe format

This is intentional. It allows older campaigns to gain better evidence products without rerunning Codex.

## Failure Semantics

Artifact completeness matters for interpretation.

The bench should make it obvious when:

- a run never reached patch extraction
- a run reached patch extraction but not grading
- a report was derived from incomplete evidence
- a run is missing specific normalized files

Missing artifacts should be treated as evidence gaps, not silently ignored.
