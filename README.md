# Codex Testing Bench

Codex Testing Bench is a Codex-only research bench for probing how the vendored Codex runtime behaves on real coding tasks such as SWE-bench Verified.

The repo is organized so the research bench is easy to read on GitHub:

- [`/Users/kevinlin/Downloads/CodexPlusClaw/bench`](/Users/kevinlin/Downloads/CodexPlusClaw/bench) contains the active Rust-first bench workspace.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex`](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex) contains a pinned vendored Codex runtime with only the runtime and probe-hook patches needed for this study.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks`](/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks) contains vendored external benchmark sources such as NewtonBench and NL2RepoBench.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/docs`](/Users/kevinlin/Downloads/CodexPlusClaw/docs) contains architecture, reference, and probe documentation.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/studies`](/Users/kevinlin/Downloads/CodexPlusClaw/studies) contains reusable claim catalogs and task presets.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/artifacts`](/Users/kevinlin/Downloads/CodexPlusClaw/artifacts) is the publishable output root for campaign reports and curated run evidence.

## What This Repo Studies

The bench is designed to answer questions like:

- How does Codex actually freeze runtime/session configuration at thread start?
- How does Codex layer instructions, developer context, and reconstructed state?
- What does Codex compaction preserve, compress, or forget?
- How much of Codex’s behavior comes from internal tool mediation and event plumbing rather than the underlying model alone?
- Which directions from the attached token-budget and long-horizon scheduling papers appear in Codex behavior, and which do not?

The main deliverable is not a dashboard or a polished paper. It is a rich evidence dossier:

- campaign-level `report.txt`
- per-run `run-evidence.txt`
- raw local artifacts and normalized JSON/JSONL telemetry

## Bench Workspace

The active bench crates live under [`/Users/kevinlin/Downloads/CodexPlusClaw/bench/crates`](/Users/kevinlin/Downloads/CodexPlusClaw/bench/crates):

- `codex-bench-core`: manifests, artifacts, traits, and shared types
- `codex-bench-codex`: direct Codex App Server integration and architecture mapping
- `codex-bench-swebench`: SWE-bench dataset/workspace/grading adapter
- `codex-bench-nl2repo`: NL2RepoBench adapter for from-scratch repository delivery tasks
- `codex-bench-newtonbench`: NewtonBench adapter for interactive scientific discovery tasks
- `codex-bench-probes`: raw-to-derived probe logic and claim evidence derivation
- `codex-bench-report`: `report.txt`, `run-evidence.txt`, and replay rendering
- `codex-bench-cli`: the user-facing CLI

## Study Presets And Benchmark Growth

The bench now has an explicit study-preset layer so campaigns can carry:

- benchmark identity and adapter choice
- staged sample sizes
- task-class coverage targets
- probe and report profiles
- forward-looking benchmark catalog targets

Preset assets live under [`/Users/kevinlin/Downloads/CodexPlusClaw/studies/task-presets`](/Users/kevinlin/Downloads/CodexPlusClaw/studies/task-presets), and the evolving benchmark catalog lives in [`/Users/kevinlin/Downloads/CodexPlusClaw/studies/benchmarks/2026-benchmark-catalog.json`](/Users/kevinlin/Downloads/CodexPlusClaw/studies/benchmarks/2026-benchmark-catalog.json).

Built-in active benchmark lanes now include:

- SWE-bench Verified
- NL2RepoBench
- NewtonBench

The bench can also run any benchmark that can be normalized into the repo-patch JSONL schema:

- `instance_id`
- `repo`
- `base_commit`
- `problem_statement`
- optional hints/tests/metadata

That gives the same Codex runtime, probe stack, and evidence reporting a reusable path for other benchmark families.

## Run The Study

From [`/Users/kevinlin/Downloads/CodexPlusClaw/bench`](/Users/kevinlin/Downloads/CodexPlusClaw/bench):

```bash
cargo run -p codex-bench-cli -- bootstrap-local \
  --campaign-dir ../artifacts/<campaign-id>

cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../artifacts \
  --preset-path ../studies/task-presets/swebench-v1.json \
  --stage architecture-validation \
  --seed codex-study

cargo run -p codex-bench-cli -- run ../artifacts/<campaign-id>

cargo run -p codex-bench-cli -- grade ../artifacts/<campaign-id> \
  --command 'python -m swebench.harness.run_evaluation --predictions_path {predictions}'

cargo run -p codex-bench-cli -- report ../artifacts/<campaign-id>

cargo run -p codex-bench-cli -- list-presets
```

`bootstrap-local` is the preferred way to reduce end-to-end runtime before a real campaign. It:

- builds `codex-bench-cli` into the repo-local Cargo target directory
- hydrates a local SWE-bench Verified JSONL snapshot under [`/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/swebench-verified`](/Users/kevinlin/Downloads/CodexPlusClaw/vendor-benchmarks/swebench-verified)
- warms the shared repo object cache under [`/Users/kevinlin/Downloads/CodexPlusClaw/.local-cache/repos/swebench`](/Users/kevinlin/Downloads/CodexPlusClaw/.local-cache/repos/swebench) for the selected campaign

If you already have a prepared campaign and only want the git object cache warmed:

```bash
cargo run -p codex-bench-cli -- warm-cache ../artifacts/<campaign-id>
```

For the new benchmark families:

```bash
cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../artifacts \
  --preset-path ../studies/task-presets/nl2repo-v0.json \
  --stage architecture-validation \
  --seed codex-study

cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../artifacts \
  --preset-path ../studies/task-presets/newtonbench-v0.json \
  --stage architecture-validation \
  --seed codex-study
```

## Reference Material

- [`docs/references/codex.md`](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/codex.md)
- [`docs/references/benchmarks.md`](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/benchmarks.md)
- [`docs/architecture/bench-architecture.md`](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)
- [`docs/probes/probe-taxonomy.md`](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)
