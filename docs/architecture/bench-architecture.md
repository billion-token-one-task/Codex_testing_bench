# Bench Architecture

## Goal

Keep the bench readable and extensible outside the vendored Codex runtime.

## Boundary

- The outer bench owns orchestration, study manifests, benchmark adapters, derived probes, and reporting.
- Vendored Codex owns runtime behavior and thin study-gated raw probe emission only.

## Outer Crates

- `codex-bench-core`
  - common types
  - artifact helpers
  - IO helpers
  - adapter/report traits
- `codex-bench-codex`
  - in-process App Server startup
  - thread start / turn start orchestration
  - raw event capture
  - architecture map generation
- `codex-bench-swebench`
  - SWE-bench sampling
  - workspace setup
  - prompt construction
  - patch extraction
  - predictions and grading wrapper
- `codex-bench-probes`
  - raw-to-derived probe derivation
  - claim catalogs
  - claim evidence labels
- `codex-bench-report`
  - campaign `report.txt`
  - per-run `run-evidence.txt`
  - replay text
- `codex-bench-cli`
  - `prepare`
  - `run`
  - `grade`
  - `report`
  - `replay`
  - `inspect-codex`

## Artifact Flow

1. `prepare`
   - writes campaign manifest
   - samples tasks
   - writes architecture map and claim catalogs
2. `run`
   - prepares isolated worktree
   - starts a study-tagged Codex App Server thread
   - captures raw agent events, diagnostics, and Codex raw study probes
   - extracts patch and writes normalized telemetry
3. `grade`
   - writes grader outputs
4. `report`
   - reads only local artifacts
   - writes `report.txt`
   - writes or refreshes `run-evidence.txt`

