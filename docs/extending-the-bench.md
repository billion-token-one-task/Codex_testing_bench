# Extending The Bench

## Goal

Make it straightforward to add new task suites, new probe families, and new report surfaces without turning the repo into a monolith.

## Extension Points

The main extension interfaces live in `codex-bench-core`.

Important boundaries:

- `BenchmarkAdapter`
  - sample tasks
  - prepare workspace
  - build prompt/input bundle
  - extract patch/output
  - invoke grader
- `RuntimeAdapter`
  - start session
  - start turn
  - drain events
  - return runtime capture
- `ProbeDeriver`
  - derive structured evidence rows from raw artifacts
- `ReportRenderer`
  - write campaign and per-run human-readable outputs
- `ClaimCatalog`
  - load claim sets and score evidence labels

## When To Add A New Benchmark Adapter

Add a dedicated adapter if the new benchmark has at least one of these:

- a different workspace materialization model
- a different grading model
- a different task object schema
- a genuinely different observation regime

Examples:

- SWE-bench: repo patching under regression verification
- NL2Repo: zero-to-one repository construction
- NewtonBench: scientific experimentation and law discovery

## When To Reuse `repo-patch-jsonl`

Reuse the generic repo-patch lane if the benchmark can be normalized into:

- `instance_id`
- `repo`
- `base_commit`
- `problem_statement`
- optional hints/tests/metadata

That lets you reuse the same runtime, probe, and report stack without creating a full bespoke adapter too early.

## Adding A Probe Family

Good probes should:

- be grounded in observable artifacts
- carry `classification` as `exact`, `inferred`, or `estimated`
- attach `evidenceCode`
- point back to source artifacts
- avoid mixing interpretation with raw observation

There are two layers:

- raw study probes emitted inside vendored Codex
- derived probes computed in the outer bench

Prefer adding only lightweight raw probe emission inside vendored Codex. Keep heavy interpretation in `codex-bench-probes`.

## Adding A New Report Type

Current canonical human-readable outputs are:

- `report.txt`
- `run-evidence.txt`
- `attempt-log.txt`

If you add a new report type:

- keep it deterministic
- keep it local-only
- make it artifact-derived
- do not require external dashboards or collectors

## Documentation Expectation

If you add:

- a benchmark adapter
- a probe family
- a new artifact type
- a new report surface

you should also update:

- [README.md](/Users/kevinlin/Downloads/CodexPlusClaw/README.md)
- [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
- [docs/artifacts/artifact-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/artifacts/artifact-contract.md)
- [docs/references/benchmarks.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/benchmarks.md) if the benchmark surface changed
