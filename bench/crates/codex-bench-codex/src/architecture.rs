use std::path::{Path, PathBuf};

use anyhow::Result;
use codex_bench_core::{StudyArchitectureSubsystem, write_json_pretty};

pub fn architecture_map() -> Vec<StudyArchitectureSubsystem> {
    vec![
        StudyArchitectureSubsystem {
            id: "session_config_freeze".to_string(),
            purpose: "Resolve requested model/runtime settings into the effective session configuration that all later turns inherit.".to_string(),
            files: vec![
                "repos/codex/codex-rs/core/src/codex.rs".to_string(),
                "repos/codex/codex-rs/core/src/thread_manager.rs".to_string(),
                "repos/codex/codex-rs/app-server/src/codex_message_processor.rs".to_string(),
            ],
            reference_docs: vec![
                "https://deepwiki.com/openai/codex".to_string(),
                "https://openai.com/index/unlocking-the-codex-harness/".to_string(),
            ],
            visible_events: vec![
                "session_configured".to_string(),
                "study_probe(config_freeze.*)".to_string(),
            ],
            hidden_state: vec![
                "config precedence winners".to_string(),
                "base instruction provenance".to_string(),
                "already-running-thread override suppression".to_string(),
            ],
            probes: vec![
                "config_freeze".to_string(),
                "session_freeze_drift".to_string(),
                "effective_runtime_divergence".to_string(),
            ],
        },
        StudyArchitectureSubsystem {
            id: "instruction_prompt_assembly".to_string(),
            purpose: "Assemble the model-visible instruction stack for each turn, including base, developer, reconstructed, and update-derived context.".to_string(),
            files: vec![
                "repos/codex/codex-rs/core/src/codex.rs".to_string(),
                "repos/codex/codex-rs/core/src/context_manager/updates.rs".to_string(),
                "repos/codex/codex-rs/core/src/config/instructions.rs".to_string(),
            ],
            reference_docs: vec![
                "https://deepwiki.com/openai/codex".to_string(),
                "https://openai.com/index/introducing-upgrades-to-codex/".to_string(),
            ],
            visible_events: vec![
                "user_message".to_string(),
                "study_probe(instruction_channel.*)".to_string(),
            ],
            hidden_state: vec![
                "instruction layering decisions".to_string(),
                "implicit context injection".to_string(),
            ],
            probes: vec![
                "instruction_channel".to_string(),
                "instruction_stratification".to_string(),
                "ambient_state_leakage".to_string(),
            ],
        },
        StudyArchitectureSubsystem {
            id: "turn_lifecycle_dispatch".to_string(),
            purpose: "Create turn contexts, dispatch work into tasks, and handle interruptions, failovers, and completions.".to_string(),
            files: vec![
                "repos/codex/codex-rs/core/src/codex.rs".to_string(),
                "repos/codex/codex-rs/core/src/tasks/regular.rs".to_string(),
                "repos/codex/codex-rs/core/src/tasks/review.rs".to_string(),
            ],
            reference_docs: vec![
                "https://deepwiki.com/openai/codex".to_string(),
                "https://openai.com/index/introducing-codex/".to_string(),
            ],
            visible_events: vec![
                "turn_started".to_string(),
                "turn_complete".to_string(),
                "turn_aborted".to_string(),
                "study_probe(turn_lifecycle.*)".to_string(),
            ],
            hidden_state: vec![
                "pending input drain behavior".to_string(),
                "task dispatch category".to_string(),
            ],
            probes: vec![
                "turn_lifecycle".to_string(),
                "activation_threshold".to_string(),
                "chain_reaction_cycles".to_string(),
            ],
        },
        StudyArchitectureSubsystem {
            id: "context_compaction".to_string(),
            purpose: "Compact and rebuild session history under context pressure, preserving some state while potentially losing detail.".to_string(),
            files: vec![
                "repos/codex/codex-rs/core/src/compact.rs".to_string(),
                "repos/codex/codex-rs/core/src/codex.rs".to_string(),
            ],
            reference_docs: vec![
                "https://deepwiki.com/openai/codex".to_string(),
                "https://openai.com/index/unlocking-the-codex-harness/".to_string(),
            ],
            visible_events: vec![
                "context_compacted".to_string(),
                "study_probe(context_compaction.*)".to_string(),
            ],
            hidden_state: vec![
                "history loss vs preserved actionable state".to_string(),
                "replacement-history shape".to_string(),
            ],
            probes: vec![
                "compaction_continuity".to_string(),
                "context_pressure".to_string(),
                "rediscovery_after_compaction".to_string(),
            ],
        },
        StudyArchitectureSubsystem {
            id: "tool_mediation".to_string(),
            purpose: "Route shell, patch, MCP, and other tools through Codex mediation instead of exposing a single raw shell channel.".to_string(),
            files: vec![
                "repos/codex/codex-rs/core/src/tools/events.rs".to_string(),
                "repos/codex/codex-rs/core/src/mcp_tool_call.rs".to_string(),
                "repos/codex/codex-rs/core/src/tools/mod.rs".to_string(),
            ],
            reference_docs: vec![
                "https://deepwiki.com/openai/codex".to_string(),
                "https://openai.com/index/unlocking-the-codex-harness/".to_string(),
            ],
            visible_events: vec![
                "exec_command_begin".to_string(),
                "exec_command_end".to_string(),
                "mcp_tool_call_begin".to_string(),
                "mcp_tool_call_end".to_string(),
                "patch_apply_begin".to_string(),
                "patch_apply_end".to_string(),
            ],
            hidden_state: vec![
                "tool routing decisions".to_string(),
                "approval path".to_string(),
                "structured output mediation".to_string(),
            ],
            probes: vec![
                "tool_mediation".to_string(),
                "tool_mediation_tax".to_string(),
            ],
        },
        StudyArchitectureSubsystem {
            id: "persistence_resume".to_string(),
            purpose: "Persist rollout state, reconstruct prior sessions, and maintain continuity across resume and listener attachment.".to_string(),
            files: vec![
                "repos/codex/codex-rs/core/src/rollout".to_string(),
                "repos/codex/codex-rs/core/src/state_db.rs".to_string(),
                "repos/codex/codex-rs/app-server/src/codex_message_processor.rs".to_string(),
            ],
            reference_docs: vec![
                "https://deepwiki.com/openai/codex".to_string(),
                "https://openai.com/index/unlocking-the-codex-harness/".to_string(),
            ],
            visible_events: vec![
                "session_configured".to_string(),
                "study_probe(persistence_reconstruction.*)".to_string(),
            ],
            hidden_state: vec![
                "state DB contention".to_string(),
                "baseline invalidation".to_string(),
                "listener generation behavior".to_string(),
            ],
            probes: vec![
                "persistence_continuity".to_string(),
                "resume_staleness".to_string(),
                "listener_attach_friction".to_string(),
            ],
        },
        StudyArchitectureSubsystem {
            id: "app_server_translation".to_string(),
            purpose: "Translate between typed client requests, app-server notifications, and the legacy core event bridge.".to_string(),
            files: vec![
                "repos/codex/codex-rs/app-server/src/codex_message_processor.rs".to_string(),
                "repos/codex/codex-rs/app-server/src/bespoke_event_handling.rs".to_string(),
                "repos/codex/codex-rs/app-server-client/src/lib.rs".to_string(),
            ],
            reference_docs: vec![
                "https://deepwiki.com/openai/codex".to_string(),
                "https://openai.com/index/unlocking-the-codex-harness/".to_string(),
            ],
            visible_events: vec![
                "codex/event/*".to_string(),
                "turn/* notifications".to_string(),
            ],
            hidden_state: vec![
                "event fanout drop behavior".to_string(),
                "typed-vs-legacy visibility gaps".to_string(),
            ],
            probes: vec![
                "event_architecture_discontinuity".to_string(),
                "listener_backpressure".to_string(),
            ],
        },
        StudyArchitectureSubsystem {
            id: "reliability_surfaces".to_string(),
            purpose: "Capture friction introduced by Codex’s own harness machinery rather than the underlying coding task.".to_string(),
            files: vec![
                "repos/codex/codex-rs/core/src/state_db.rs".to_string(),
                "repos/codex/codex-rs/app-server/src/in_process.rs".to_string(),
                "repos/codex/codex-rs/core/src/error.rs".to_string(),
            ],
            reference_docs: vec!["https://deepwiki.com/openai/codex".to_string()],
            visible_events: vec![
                "warning".to_string(),
                "error".to_string(),
                "stream_error".to_string(),
                "study_probe(harness_friction.*)".to_string(),
            ],
            hidden_state: vec![
                "DB lock contention".to_string(),
                "listener attach failures".to_string(),
                "auth/MCP/runtime mismatch leakage".to_string(),
            ],
            probes: vec![
                "harness_friction".to_string(),
                "containment_integrity".to_string(),
            ],
        },
    ]
}

pub fn write_architecture_map(campaign_dir: &Path) -> Result<PathBuf> {
    let path = campaign_dir.join("codex-architecture-map.json");
    write_json_pretty(&path, &architecture_map())?;
    Ok(path)
}

