# Probe Taxonomy

## Goal

The probe program exists to generate evidence about how Codex works, not just whether it solved a task.

The design has two layers:

- raw probes emitted by study-tagged Codex runtime code
- derived probes computed from local artifacts after the run

## Raw Codex Probe Families

These are emitted from inside vendored Codex only for study-tagged runs.

### Config Freeze

Examples:

- requested vs effective runtime
- config precedence winner
- model-native instruction preservation or displacement
- override suppression on already-running threads
- exact point where the effective session config becomes fixed

### Instruction Channels

Examples:

- base vs developer vs model-native vs reconstructed context
- ambient skill or session leakage vs suppression
- instruction makeup changes after compaction or resume

### Turn Lifecycle

Examples:

- session spawn
- turn start / end
- active-turn registration / cleanup
- timeout, interruption, and failover boundaries

### Context And Compaction

Examples:

- compaction trigger token level
- pre/post history size
- compaction mode and reason
- reinjected state after reconstruction
- post-compaction rediscovery markers

### Tool Mediation

Examples:

- shell vs patch vs MCP routing
- approval and sandbox path
- structured vs raw-style result propagation
- tool begin / end boundaries

### Persistence And Reconstruction

Examples:

- rollout recording mode
- state DB usage
- resume / fork / rebuild path
- listener attach behavior

### Harness Friction

Examples:

- state DB contention
- listener attach failure
- rollout writer failure
- runtime mismatch warnings that materially affect the run

## Derived Probe Families

These are computed outside vendored Codex from raw artifacts.

## Paper-Aligned Families

### Activation Threshold

- tokens/time to first meaningful edit
- tokens/time to first verification
- tokens/time to first retained patch
- tokens/time to final patch

### Redundancy

- repeated read without edit
- repeated verification without code change
- repeated git inspection
- post-submit activity
- cleanup-only work

### Context Pressure

- prompt growth
- cache-read ratio
- compaction count and intervals
- history growth slope
- rediscovery after compaction

### Verification Structure

- edit-to-verification closure
- changed verification outcomes after edits
- verification retry loops
- externally verified work fraction

### Useful Work vs Friction

- useful-step proxy
- useful-token proxy
- friction-token proxy
- retained-edit ratio
- reverted-work ratio

## Codex-Native Families

These are the probes most likely to generate genuinely Codex-specific conclusions.

### Fission / Ignition

- first meaningful retained work
- time/tokens from prompt submission to first controlled code change
- ignition mode: shell search vs patch apply vs tool-mediated edit

### Chain Reaction

- edit -> verify -> edit -> verify propagation depth
- productive reaction cycles before termination
- whether the run becomes self-sustaining or stalls

### Control Rod

- compaction as regulation
- config freeze as regulation
- persistence/resume as regulation
- approval/listener boundaries as regulation

### Containment

- state drift
- coherence breaks
- heat leakage into orchestration overhead
- failure modes where the harness absorbs budget without producing progress

### Instruction Stratification

- whether Codex behaves like layered state
- when model-native instructions dominate
- when reconstructed context or developer instructions dominate

### Tool Mediation Tax

- where orchestration helps
- where it adds latency or duplicated work
- where tool routing differs from naive raw shell expectations

### Persistence Half-Life

- how long useful state survives after compaction or reconstruction
- when remembered state decays into rediscovery

### Event-Architecture Discontinuity

- gaps between typed events, legacy notifications, and probe streams
- visibility loss due to listener or translation effects

### Externalized Coordination

- evidence that Codex preserves and reuses state across regulation layers
- evidence that Codex behaves more like layered coordination than a pure flat transcript consumer

## New Human-Oriented Telemetry

The bench now emits extra human-usable attempt artifacts:

- `turn-metrics.jsonl`
  - token deltas per turn
  - command/tool/skill counts per turn
- `skill-events.jsonl`
  - explicit and inferred skill usage events
- `attempt-log.txt`
  - a single chronological view of lifecycle, command, tool, skill, and anomaly events

These sit alongside:

- `run-evidence.txt`
- `report.txt`

## Classification Labels

Every derived row should declare one of:

- `exact`
- `inferred`
- `estimated`

## Claim Evidence Labels

Claim scoring is intentionally evidentiary:

- `evidence_consistent`
- `evidence_mixed`
- `evidence_inconclusive`
- `evidence_against`
- `not_observable_with_current_probes`

The bench should never silently blur raw observation and interpretive scoring.
