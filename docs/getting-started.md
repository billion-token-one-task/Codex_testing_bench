# Getting Started

## Goal

Get a reproducible Codex study campaign running locally and end with a human-readable evidence package.

## Prerequisites

- macOS host
- Rust toolchain installed
- Python available for benchmark-specific grading workflows
- vendored Codex already present under `repos/codex`
- local auth/config already working for the Codex runtime you are studying

## Typical Workflow

All commands below run from [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench).

### 1. Prepare a campaign

```bash
cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../artifacts \
  --preset-path ../studies/task-presets/swebench-v1.json \
  --stage behavior-pilot \
  --seed my-study
```

This writes:

- `campaign-manifest.json`
- `selected-dataset.json`
- `codex-architecture-map.json`
- claim catalog copies for the campaign

### 2. Bootstrap local assets

```bash
cargo run -p codex-bench-cli -- bootstrap-local \
  --campaign-dir ../artifacts/<campaign-id>
```

This is the preferred preflight because it:

- builds the local bench binary
- ensures benchmark-local assets are downloaded into the repo filesystem
- warms the shared git object cache for the selected tasks when possible

### 3. Run the benchmark

```bash
cargo run -p codex-bench-cli -- run ../artifacts/<campaign-id>
```

As each run executes, look under:

- `artifacts/<campaign-id>/runs/<instance>/manifest.json`
- `artifacts/<campaign-id>/runs/<instance>/attempt-01/`

### 4. Grade the outputs

For SWE-bench:

```bash
cargo run -p codex-bench-cli -- grade ../artifacts/<campaign-id> \
  --command 'python -m swebench.harness.run_evaluation --predictions_path {predictions}'
```

### 5. Render the evidence dossier

```bash
cargo run -p codex-bench-cli -- report ../artifacts/<campaign-id>
```

Primary outputs:

- `artifacts/<campaign-id>/reports/report.txt`
- `artifacts/<campaign-id>/runs/<instance>/attempt-01/run-evidence.txt`
- `artifacts/<campaign-id>/runs/<instance>/attempt-01/attempt-log.txt`

## What To Read In A Finished Run

If you want the shortest path to understanding one attempted question:

1. `manifest.json`
2. `run-summary.json`
3. `run-evidence.txt`
4. `attempt-log.txt`

If you need the full machine-level detail:

1. `raw-agent-events.jsonl`
2. `codex-probe-events.jsonl`
3. `probe-events.jsonl`
4. `turn-metrics.jsonl`
5. `tool-events.jsonl`
6. `command-events.jsonl`

## Policy Notes

- benchmark runs are local-only
- benchmark runs are Codex-only
- web search is intentionally disabled in evaluated benchmark runs
- if a forbidden web-search event appears, the run is treated as invalid

## Troubleshooting

If a campaign looks stuck:

- inspect the top-level run process with `ps`
- inspect the active run manifest
- inspect whether `raw-agent-events.jsonl` is still growing
- inspect whether `codex-probe-events.jsonl` is still growing

If a report is missing newer artifacts:

- rerun `report`
- the reporting path can backfill newer derived files from older raw artifacts when possible
