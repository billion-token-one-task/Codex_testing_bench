# Bench Architecture

## Design Goal

Keep the research bench legible at the repo root while preserving deep access to the vendored Codex runtime.

The architecture intentionally separates:

- outer-bench orchestration and research logic
- vendored Codex runtime behavior

## High-Level Boundary

The outer bench owns:

- campaign preparation
- benchmark adapters
- workspace materialization
- run orchestration
- raw artifact collection
- derived probes
- claim catalogs
- reporting

Vendored Codex owns:

- App Server
- agent runtime behavior
- session/config freezing
- prompt assembly
- compaction and reconstruction
- tool mediation
- persistence and resume
- thin study-gated raw probe emission

## Why This Boundary Exists

If the whole benchmark stack lived inside vendored Codex, the GitHub repo would be hard to navigate and the research layer would be tightly coupled to one runtime implementation.

By keeping the bench outside:

- the repo is much easier to understand
- adapters can grow independently
- reports and claim logic stay reusable
- Codex patches can stay minimal and auditable

## Outer Workspace

The active workspace lives under [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench).

### `codex-bench-core`

Owns:

- shared types
- manifests
- artifact paths
- JSON / JSONL IO
- adapter and renderer traits

This crate is the stability layer for the rest of the bench.

### `codex-bench-codex`

Owns:

- direct in-process App Server startup
- thread start / turn start orchestration
- runtime configuration for evaluated Codex runs
- raw event capture
- raw diagnostic capture
- architecture-map generation

This is the only outer crate that directly speaks to vendored Codex crates.

### `codex-bench-swebench`

Owns:

- SWE-bench Verified sampling
- repo-patch task normalization
- worktree setup
- prompt construction
- patch extraction
- grading integration

### `codex-bench-nl2repo`

Owns:

- NL2Repo task discovery
- blank repository setup
- benchmark-local grading commands

### `codex-bench-newtonbench`

Owns:

- NewtonBench task generation
- local experiment-lab setup
- NewtonBench evaluation wrapping

### `codex-bench-probes`

Owns:

- raw-to-derived probe derivation
- probe summaries
- claim evidence derivation
- behavioral counters and evidence labels

### `codex-bench-report`

Owns:

- campaign `report.txt`
- per-run `run-evidence.txt`
- per-run `attempt-log.txt`
- replay text
- report-time backfilling of newer derived artifacts from older raw evidence

### `codex-bench-cli`

Owns the user-facing commands:

- `prepare`
- `run`
- `bootstrap-local`
- `warm-cache`
- `grade`
- `report`
- `replay`
- `inspect-codex`
- `list-presets`

## Data Flow

### 1. Prepare

`prepare` writes:

- `campaign-manifest.json`
- `selected-dataset.json`
- `codex-architecture-map.json`
- campaign-local claim catalogs

At this point the campaign is defined but no Codex run has happened yet.

### 2. Run

`run` does:

1. materialize a benchmark workspace
2. start a study-tagged Codex thread
3. capture raw App Server notifications
4. capture raw Codex study probe events
5. extract patch/output
6. derive normalized telemetry
7. write per-run summaries and human-readable evidence

### 3. Grade

`grade` uses the benchmark adapter's grading path and writes grader outputs back into campaign artifacts.

### 4. Report

`report` reads local artifacts only and produces:

- campaign `report.txt`
- refreshed per-run `run-evidence.txt`
- refreshed per-run `attempt-log.txt`

If newer derived artifact formats exist, `report` may backfill them from older raw artifacts.

## Artifact Philosophy

The bench intentionally keeps two layers:

- raw truth
- readable evidence

Raw truth:

- raw agent events
- raw diagnostics
- raw Codex probe events

Readable evidence:

- turn metrics
- tool and skill summaries
- probe summaries
- run evidence
- campaign report

This is why the repo can stay GitHub-friendly without losing scientific utility.

## Current Evaluation Policy

For benchmark evaluation runs:

- Codex web search is disabled
- the bench aborts if a web-search event still appears
- the study stays local-only
- the runtime path is Codex-only

Those constraints are part of the bench architecture, not just operator convention.
