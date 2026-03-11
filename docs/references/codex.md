# Codex References

This bench is grounded in two things:

- the local vendored Codex source under [repos/codex](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex)
- a small set of external architecture references used to orient probe design

## Primary External References

- [DeepWiki Codex](https://deepwiki.com/openai/codex)
- [OpenAI: Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness/)
- [OpenAI: Introducing upgrades to Codex](https://openai.com/index/introducing-upgrades-to-codex/)
- [OpenAI: Introducing Codex](https://openai.com/index/introducing-codex/)

## How To Use These References Correctly

These references are architecture aids, not substitutes for the local source.

Use them to:

- orient yourself to App Server concepts
- identify likely seams for compaction, session lifecycle, and tool orchestration
- cross-check whether a probe direction matches the documented architecture at a high level

Do not use them to:

- override what the local vendored source actually does
- assume that every described subsystem behaves identically in this pinned runtime
- claim behavior without local artifact evidence

## Why DeepWiki Is In The Repo Docs

DeepWiki is the fastest high-level orientation layer for:

- session lifecycle
- App Server request/response shape
- tool orchestration
- context management and compaction
- persistence/resume structure

That makes it useful when designing new probes or mapping local source files into the generated `codex-architecture-map.json`.

## Bench Rule Of Thumb

When there is tension between:

- external docs
- local vendored source
- actual run artifacts

the priority order is:

1. actual local run artifacts
2. local vendored source
3. external reference docs

## How These References Map To The Bench

The bench uses these references to shape:

- the architecture map
- raw probe families
- claim catalogs
- the “Codex under observation” sections of `report.txt`

Typical local seams studied include:

- session/config freeze
- instruction and prompt assembly
- turn lifecycle dispatch
- context compaction and reconstruction
- tool mediation
- persistence/resume
- App Server translation/listener path
- reliability surfaces

## Related Internal Research Direction

The bench is not trying to prove the references right.

It uses them to ask better questions about:

- whether Codex behaves more like layered state than flat transcript accumulation
- whether compaction preserves actionable state or creates rediscovery loops
- how much of Codex’s apparent behavior is model-driven versus harness-mediated
- where Codex’s architecture supports or departs from long-horizon token-scheduling intuitions
