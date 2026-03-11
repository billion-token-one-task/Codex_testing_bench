# Codex Testing Bench

Codex Testing Bench is a Codex-only research bench for studying how the vendored Codex runtime behaves on real tasks and benchmarks.

The repo is designed around one core principle:

- the research bench should be easy to understand from the GitHub root
- the vendored Codex tree should stay a pinned runtime target with thin study-only probe hooks
- local artifacts should be rich enough to support serious internal research without dashboards or external telemetry systems

## What This Repo Is For

This repo is for deep empirical study of Codex as an agent harness, not just benchmark scoring.

The bench is meant to help answer questions like:

- How does Codex actually freeze runtime and session state?
- How does Codex assemble instructions across model-native prompts, developer instructions, reconstructed context, and probe-time updates?
- What does Codex compaction preserve, compress, or forget?
- How much work is done by the model versus Codex's own orchestration machinery?
- When does Codex enter a productive edit and verification loop, and when does it burn budget in harness friction?
- Which ideas from our token-budget and long-horizon scheduling theses appear concretely in Codex behavior?

The main output is an evidence package:

- campaign-level `report.txt`
- per-run `run-evidence.txt`
- per-run `attempt-log.txt`
- raw and normalized local telemetry artifacts

It is intentionally not a dashboard and not the final paper.

## Repo Map

- [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench): active outer research bench workspace
- [repos/codex](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex): vendored Codex runtime with thin study-gated probe patches
- [vendor-benchmarks](/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks): vendored external benchmark assets
- [studies](/Users/kevinlin/Downloads/CodexPlusClaw/studies): reusable claim catalogs, presets, and benchmark catalog metadata
- [docs](/Users/kevinlin/Downloads/CodexPlusClaw/docs): architecture, references, probe taxonomy, artifact contract, and extension docs
- [artifacts](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts): GitHub-visible campaign outputs and curated evidence

## Bench Workspace

The active crates live under [bench/crates](/Users/kevinlin/Downloads/CodexPlusClaw/bench/crates):

- `codex-bench-core`
  - manifests, artifacts, IO helpers, traits, shared types
- `codex-bench-codex`
  - direct in-process Codex App Server integration
  - raw event capture
  - architecture map generation
  - no-web-search runtime enforcement for benchmark runs
- `codex-bench-swebench`
  - SWE-bench Verified adapter
  - repo-patch-jsonl generic adapter path
- `codex-bench-nl2repo`
  - NL2RepoBench adapter
- `codex-bench-newtonbench`
  - NewtonBench adapter
- `codex-bench-probes`
  - raw-to-derived probe logic
  - claim evidence derivation
- `codex-bench-report`
  - `report.txt`
  - `run-evidence.txt`
  - `attempt-log.txt`
  - replay text
- `codex-bench-cli`
  - prepare / run / grade / report / replay / inspect-codex

## Supported Benchmarks

First-class local adapters currently exist for:

- SWE-bench Verified
- NL2RepoBench
- NewtonBench

The bench is also designed to grow beyond those:

- `repo-patch-jsonl` is the reusable bridge for repo-based patch tasks
- presets and adapter boundaries are meant to support future benchmark families without changing the Codex shim

See:

- [docs/references/benchmarks.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/benchmarks.md)
- [studies/benchmarks/2026-benchmark-catalog.json](/Users/kevinlin/Downloads/CodexPlusClaw/studies/benchmarks/2026-benchmark-catalog.json)

## What Gets Measured

The bench captures four layers of evidence:

1. raw runtime streams
2. Codex-internal study probes
3. derived behavioral probes
4. human-readable evidence reports

Examples of what is captured:

- token in / out / cache-read over time
- token deltas per turn
- command chronology
- tool chronology
- apply-patch activity
- skill usage events
- compaction and reconstruction evidence
- instruction-channel shifts
- config-freeze drift
- persistence and resume effects
- harness-friction incidents
- claim evidence labels grounded in local artifacts

See [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md).

## Quick Start

All commands are run from [bench](/Users/kevinlin/Downloads/CodexPlusClaw/bench).

### 1. Prepare a campaign

```bash
cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../artifacts \
  --preset-path ../studies/task-presets/swebench-v1.json \
  --stage architecture-validation \
  --seed codex-study
```

### 2. Warm local assets and shared cache

```bash
cargo run -p codex-bench-cli -- bootstrap-local \
  --campaign-dir ../artifacts/<campaign-id>
```

### 3. Run Codex on the campaign

```bash
cargo run -p codex-bench-cli -- run ../artifacts/<campaign-id>
```

### 4. Grade and render the evidence dossier

```bash
cargo run -p codex-bench-cli -- grade ../artifacts/<campaign-id> \
  --command 'python -m swebench.harness.run_evaluation --predictions_path {predictions}'

cargo run -p codex-bench-cli -- report ../artifacts/<campaign-id>
```

### 5. Inspect the results

- campaign report: `artifacts/<campaign-id>/reports/report.txt`
- per-run evidence: `artifacts/<campaign-id>/runs/<instance>/attempt-01/run-evidence.txt`
- per-run linear log: `artifacts/<campaign-id>/runs/<instance>/attempt-01/attempt-log.txt`

For a fuller walkthrough, see [docs/getting-started.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/getting-started.md).

## Evaluation Policy

Benchmark runs are intentionally constrained to reduce contamination and make interpretation cleaner.

Important current policy choices:

- benchmark runs are local-only
- benchmark runs do not use OpenClaw
- the evaluated Codex runtime has web search explicitly disabled
- if Codex emits a web-search event anyway, the benchmark run fails immediately

This keeps the evidence focused on Codex's harness behavior inside the repo-local runtime.

## Published vs Local Artifacts

The repo separates GitHub-visible evidence from machine-local heavy data.

GitHub-visible:

- campaign manifests
- selected dataset snapshots
- architecture maps
- claim catalogs
- `report.txt`
- `run-evidence.txt`
- `attempt-log.txt`
- summary JSON artifacts

Local-only:

- warmed repo caches
- worktrees and heavy prepared workspaces
- full raw JSONL streams unless intentionally curated
- bulky transient files

See [artifacts/README.md](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts/README.md) and [docs/artifacts/artifact-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/artifacts/artifact-contract.md).

## Why Codex Is Vendored

The bench depends on deep Codex-internal observation:

- session/config freeze
- instruction assembly
- compaction and reconstruction
- event/listener translation
- tool mediation
- persistence/resume behavior

Those signals are only visible if the runtime is pinned and locally patchable.

The outer bench owns orchestration, reporting, and extensibility.
Vendored Codex owns runtime behavior and thin study-only probe emission.

See [docs/architecture/bench-architecture.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md).

## Key References

- [docs/references/codex.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/codex.md)
- [docs/references/benchmarks.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/benchmarks.md)
- [DeepWiki Codex](https://deepwiki.com/openai/codex)
- [OpenAI: Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness/)

## Recommended Reading Order

If you are new to the repo:

1. [README.md](/Users/kevinlin/Downloads/CodexPlusClaw/README.md)
2. [docs/getting-started.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/getting-started.md)
3. [docs/architecture/bench-architecture.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)
4. [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
5. [docs/artifacts/artifact-contract.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/artifacts/artifact-contract.md)

If you want to extend the system:

1. [docs/architecture/bench-architecture.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)
2. [docs/extending-the-bench.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/extending-the-bench.md)
3. [docs/probes/probe-taxonomy.md](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
