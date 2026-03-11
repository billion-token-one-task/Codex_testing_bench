# Artifacts

This directory is the GitHub-visible results surface for the Codex research bench.

What is intended to be committed here:

- campaign manifests
- selected dataset snapshots
- claim catalogs and architecture maps
- campaign-level `report.txt`
- grader summaries
- per-run `manifest.json`
- per-run `record.json`
- per-attempt `run-summary.json`
- per-attempt `probe-summary.json`
- per-attempt `claim-evidence.json`
- per-attempt `run-evidence.txt`
- per-attempt `replay.json`

What stays local-only and is intentionally ignored:

- warmed repo caches
- prepared workspaces/worktrees
- raw event JSONL streams
- raw diagnostics
- full prompt captures
- environment staging files
- binary diffs and other bulky transient artifacts

This split keeps GitHub legible while preserving the ability to regenerate rich local evidence from a machine that ran the benchmark.
