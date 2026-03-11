use std::path::{Path, PathBuf};

use anyhow::Result;
use codex_bench_core::{ClaimCatalogEntry, write_json_pretty};

pub fn grounding_claims(token_budget_doc: &Path, scheduler_doc: &Path) -> Vec<ClaimCatalogEntry> {
    vec![
        ClaimCatalogEntry {
            id: "grounding.activation_threshold".to_string(),
            source: token_budget_doc.display().to_string(),
            text: "Useful work has an activation threshold rather than rising linearly from zero budget.".to_string(),
            operationalization: "Measure tokens/time to first meaningful edit, first verification, first retained patch, and compare across runs and task classes.".to_string(),
            required_evidence: vec![
                "activation.first_meaningful_edit".to_string(),
                "activation.first_verification".to_string(),
                "activation.first_patch".to_string(),
            ],
            caveats: vec!["Single-run evidence cannot establish the full curve shape.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.anti_regularization".to_string(),
            source: token_budget_doc.display().to_string(),
            text: "Higher token budgets can anti-regularize the agent and increase redundancy, anxiety loops, or cleanup behavior.".to_string(),
            operationalization: "Track repeated verification, git inspection loops, cleanup-only work, and post-submit activity across task classes.".to_string(),
            required_evidence: vec![
                "verification.retry_loop".to_string(),
                "redundancy.git_loop".to_string(),
                "redundancy.post_submit".to_string(),
                "redundancy.cleanup_only".to_string(),
            ],
            caveats: vec!["Without explicit budget sweeps, support is directional rather than causal.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.flat_history_tax".to_string(),
            source: token_budget_doc.display().to_string(),
            text: "Flat-history reread imposes a major token tax on agent execution.".to_string(),
            operationalization: "Count repeated file reads without intervening edits, cache-read ratio, prompt growth, and rediscovery after compaction.".to_string(),
            required_evidence: vec![
                "redundancy.repeated_read".to_string(),
                "context.cache_read_ratio".to_string(),
                "context.prompt_growth".to_string(),
                "context.rediscovery".to_string(),
            ],
            caveats: vec!["Repeated reads can be strategic rather than purely wasteful.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.state_compression_loss".to_string(),
            source: scheduler_doc.display().to_string(),
            text: "State compression is useful but lossy; long-horizon tasks need mechanisms that preserve actionable state across compression boundaries.".to_string(),
            operationalization: "Inspect compaction counts, replacement-history shape, post-compaction rediscovery, and continuity failures.".to_string(),
            required_evidence: vec![
                "context.compaction".to_string(),
                "context.rediscovery".to_string(),
                "containment.breach".to_string(),
            ],
            caveats: vec!["Some losses are silent and therefore only partially observable.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.layered_state".to_string(),
            source: scheduler_doc.display().to_string(),
            text: "Layered state/context is preferable to forcing a single flat session to carry all task state.".to_string(),
            operationalization: "Inspect instruction channels, compaction rebuilds, resume continuity, and whether Codex preserves actionable state outside the immediate turn transcript.".to_string(),
            required_evidence: vec![
                "instruction.channel_mix".to_string(),
                "context.compaction".to_string(),
                "persistence.resume_path".to_string(),
                "persistence.continuity".to_string(),
            ],
            caveats: vec!["A system can approximate layered state without naming it explicitly.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.verification_pressure".to_string(),
            source: scheduler_doc.display().to_string(),
            text: "External verification pressure improves effective work and stabilizes search.".to_string(),
            operationalization: "Measure edit-to-verification closure, changed test states, verification retry loops, and useful-work proxies on verification-heavy tasks.".to_string(),
            required_evidence: vec![
                "verification.edit_closure".to_string(),
                "verification.changed_test_state".to_string(),
                "useful_work.proxy".to_string(),
            ],
            caveats: vec!["Verification quality depends on the available test oracle and task setup fidelity.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.context_vs_budget".to_string(),
            source: scheduler_doc.display().to_string(),
            text: "Context window and total task token budget are distinct constraints and should not be conflated.".to_string(),
            operationalization: "Track compaction thresholds, context-window-related probe data, and total token usage separately from wall time.".to_string(),
            required_evidence: vec![
                "context.compaction".to_string(),
                "tokens.total".to_string(),
                "context.window".to_string(),
            ],
            caveats: vec!["Requires enough token-count coverage to estimate context pressure reliably.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.externalized_coordination".to_string(),
            source: scheduler_doc.display().to_string(),
            text: "Scaling beyond a single session often requires externalized coordination rather than only enlarging the immediate working context.".to_string(),
            operationalization: "Inspect whether Codex relies on persistence, rollout state, resumability, and reconstructed context rather than pure flat transcript continuation.".to_string(),
            required_evidence: vec![
                "persistence.resume_path".to_string(),
                "instruction.channel_mix".to_string(),
                "context.compaction".to_string(),
            ],
            caveats: vec!["This study observes Codex behavior; it does not test multi-session schedulers directly.".to_string()],
        },
        ClaimCatalogEntry {
            id: "grounding.control_regulation".to_string(),
            source: scheduler_doc.display().to_string(),
            text: "Long-horizon systems need explicit regulation layers that stabilize search and bound loss, rather than relying on unconstrained continuation.".to_string(),
            operationalization: "Inspect compaction, config freeze, persistence, approval, and listener behaviors as control-rod-like regulation mechanisms.".to_string(),
            required_evidence: vec![
                "control_rod.compaction_regulation".to_string(),
                "control_rod.config_freeze".to_string(),
                "control_rod.persistence".to_string(),
            ],
            caveats: vec!["Some regulation layers may stabilize runs while also introducing distortion or suppression.".to_string()],
        },
    ]
}

pub fn codex_unique_claims() -> Vec<ClaimCatalogEntry> {
    vec![
        ClaimCatalogEntry {
            id: "codex.compaction_rebuild".to_string(),
            source: "DeepWiki + Codex source".to_string(),
            text: "Codex is not a pure flat-loop harness because it performs native compaction and history rebuild.".to_string(),
            operationalization: "Inspect context-compaction probes, replacement history shape, and post-compaction rediscovery patterns.".to_string(),
            required_evidence: vec![
                "context.compaction".to_string(),
                "context.rediscovery".to_string(),
                "context.replacement_history".to_string(),
            ],
            caveats: vec!["A compaction mechanism can still be lossy or brittle.".to_string()],
        },
        ClaimCatalogEntry {
            id: "codex.config_freeze".to_string(),
            source: "Codex App Server + core".to_string(),
            text: "Codex behavior is materially shaped by session/config freeze at thread start.".to_string(),
            operationalization: "Compare requested thread-start params against effective SessionConfigured values and turn-level instruction makeup.".to_string(),
            required_evidence: vec![
                "config.requested_vs_effective".to_string(),
                "instruction.channel_mix".to_string(),
            ],
            caveats: vec!["Some drift reflects provider normalization rather than hidden harness behavior.".to_string()],
        },
        ClaimCatalogEntry {
            id: "codex.tool_mediation".to_string(),
            source: "Codex core tool orchestration".to_string(),
            text: "Codex tool use is mediated by an internal routing/orchestration layer rather than being equivalent to raw shell execution.".to_string(),
            operationalization: "Track shell vs patch vs MCP tool paths, approval routing, and structured tool-event boundaries.".to_string(),
            required_evidence: vec![
                "tools.routing".to_string(),
                "tools.approval_path".to_string(),
                "tools.mediation_tax".to_string(),
            ],
            caveats: vec!["A single task may exercise only a subset of tool paths.".to_string()],
        },
        ClaimCatalogEntry {
            id: "codex.persistence_continuity".to_string(),
            source: "Codex rollout/state DB path".to_string(),
            text: "Codex persistence and resume semantics create both continuity benefits and mismatch risks.".to_string(),
            operationalization: "Track resume/reconstruction probes, listener attach behavior, and stale-state mismatch incidents.".to_string(),
            required_evidence: vec![
                "persistence.resume_path".to_string(),
                "persistence.continuity".to_string(),
                "harness.listener_attach".to_string(),
            ],
            caveats: vec!["Needs resumed or reconstructed sessions for the strongest evidence.".to_string()],
        },
        ClaimCatalogEntry {
            id: "codex.event_architecture".to_string(),
            source: "Codex App Server event bridge".to_string(),
            text: "Codex’s queue/listener/event bridge creates visibility and discontinuity patterns that wrapper-only telemetry would miss.".to_string(),
            operationalization: "Compare raw legacy event streams, typed notifications, study probes, and lagged/backpressure diagnostics.".to_string(),
            required_evidence: vec![
                "events.legacy_vs_typed".to_string(),
                "harness.listener_attach".to_string(),
                "events.backpressure".to_string(),
            ],
            caveats: vec!["Some discontinuities are observability artifacts rather than reasoning artifacts.".to_string()],
        },
        ClaimCatalogEntry {
            id: "codex.harness_overhead_tax".to_string(),
            source: "Codex runtime orchestration".to_string(),
            text: "A non-trivial share of Codex runtime cost comes from internal harness orchestration, not direct task progress.".to_string(),
            operationalization: "Estimate friction-token, friction-step, and harness-specific incident counts relative to direct edit/verify cycles.".to_string(),
            required_evidence: vec![
                "harness.friction".to_string(),
                "useful_work.proxy".to_string(),
                "tools.mediation_tax".to_string(),
            ],
            caveats: vec!["Overhead estimates are partly inferred rather than exact.".to_string()],
        },
        ClaimCatalogEntry {
            id: "codex.instruction_stratification".to_string(),
            source: "Codex instruction assembly path".to_string(),
            text: "Codex behaves more like a layered instruction stack than a single undifferentiated transcript.".to_string(),
            operationalization: "Track instruction-channel probe events, model-native instruction preservation, and reconstructed-context shifts after compaction or resume.".to_string(),
            required_evidence: vec![
                "instruction.channel_mix".to_string(),
                "instruction.stratification".to_string(),
                "persistence.externalized_state".to_string(),
            ],
            caveats: vec!["The exact effect of each layer remains partly hidden because internal reasoning is not exposed.".to_string()],
        },
        ClaimCatalogEntry {
            id: "codex.control_rods".to_string(),
            source: "Codex runtime control surfaces".to_string(),
            text: "Codex contains harness-native regulation layers that act like control rods, sometimes stabilizing and sometimes throttling the reaction.".to_string(),
            operationalization: "Track control-rod probe families across compaction, config freeze, persistence, and listener/approval boundaries.".to_string(),
            required_evidence: vec![
                "control_rod.compaction_regulation".to_string(),
                "control_rod.config_freeze".to_string(),
                "control_rod.persistence".to_string(),
                "containment.heat_leak".to_string(),
            ],
            caveats: vec!["This is a harness-level interpretation grounded in observable behavior, not an implementation claim from upstream docs.".to_string()],
        },
    ]
}

pub fn write_claim_catalog_assets(
    campaign_dir: &Path,
    token_budget_doc: &Path,
    scheduler_doc: &Path,
) -> Result<(PathBuf, PathBuf)> {
    let grounding = grounding_claims(token_budget_doc, scheduler_doc);
    let codex_unique = codex_unique_claims();
    let grounding_path = campaign_dir.join("grounding-claims.json");
    let codex_path = campaign_dir.join("codex-unique-claims.json");
    write_json_pretty(&grounding_path, &grounding)?;
    write_json_pretty(&codex_path, &codex_unique)?;
    Ok((grounding_path, codex_path))
}
