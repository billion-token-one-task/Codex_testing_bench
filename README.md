# Codex Testing Bench

Codex Testing Bench is a Codex-only research bench for probing how the vendored Codex runtime behaves on real coding tasks such as SWE-bench Verified.

The repo is organized so the research bench is easy to read on GitHub:

- [`/Users/kevinlin/Downloads/CodexPlusClaw/bench`](/Users/kevinlin/Downloads/CodexPlusClaw/bench) contains the active Rust-first bench workspace.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex`](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex) contains a pinned vendored Codex runtime with only the runtime and probe-hook patches needed for this study.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/docs`](/Users/kevinlin/Downloads/CodexPlusClaw/docs) contains architecture, reference, and probe documentation.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/studies`](/Users/kevinlin/Downloads/CodexPlusClaw/studies) contains reusable claim catalogs and task presets.
- [`/Users/kevinlin/Downloads/CodexPlusClaw/.artifacts`](/Users/kevinlin/Downloads/CodexPlusClaw/.artifacts) is the local-only output root for campaigns and run evidence.

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
- `codex-bench-probes`: raw-to-derived probe logic and claim evidence derivation
- `codex-bench-report`: `report.txt`, `run-evidence.txt`, and replay rendering
- `codex-bench-cli`: the user-facing CLI

## Run The SWE-bench Study

From [`/Users/kevinlin/Downloads/CodexPlusClaw/bench`](/Users/kevinlin/Downloads/CodexPlusClaw/bench):

```bash
cargo run -p codex-bench-cli -- prepare \
  --campaign-root ../.artifacts \
  --sample-size 1 \
  --seed codex-study

cargo run -p codex-bench-cli -- run ../.artifacts/<campaign-id>

cargo run -p codex-bench-cli -- grade ../.artifacts/<campaign-id> \
  --command 'python -m swebench.harness.run_evaluation --predictions_path {predictions}'

cargo run -p codex-bench-cli -- report ../.artifacts/<campaign-id>
```

## Reference Material

- [`docs/references/codex.md`](/Users/kevinlin/Downloads/CodexPlusClaw/docs/references/codex.md)
- [`docs/architecture/bench-architecture.md`](/Users/kevinlin/Downloads/CodexPlusClaw/docs/architecture/bench-architecture.md)
- [`docs/probes/probe-taxonomy.md`](/Users/kevinlin/Downloads/CodexPlusClaw/docs/probes/probe-taxonomy.md)

