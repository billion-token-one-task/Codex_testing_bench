# Codex References

This bench is grounded in the local vendored Codex source and these external architecture references:

- [DeepWiki Codex](https://deepwiki.com/openai/codex)
- [OpenAI: Unlocking the Codex harness](https://openai.com/index/unlocking-the-codex-harness/)
- [OpenAI: Introducing upgrades to Codex](https://openai.com/index/introducing-upgrades-to-codex/)
- [OpenAI: Introducing Codex](https://openai.com/index/introducing-codex/)

## How We Use These References

- DeepWiki is the quickest architecture orientation layer for session lifecycle, App Server behavior, context management, and tool orchestration.
- The OpenAI engineering posts are used to anchor claims about agent loop behavior, App Server design, and compaction/context-management direction.
- The local file-of-record remains the vendored source in [`/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex`](/Users/kevinlin/Downloads/CodexPlusClaw/repos/codex).

## Mapping To The Local Architecture Map

The generated `codex-architecture-map.json` and the campaign `report.txt` map these references onto concrete local seams such as:

- `session_config_freeze`
- `instruction_prompt_assembly`
- `turn_lifecycle_dispatch`
- `context_compaction`
- `tool_mediation`
- `persistence_resume`
- `app_server_translation`
- `reliability_surfaces`

