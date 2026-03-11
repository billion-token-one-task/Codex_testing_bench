use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use codex_app_server_protocol::JSONRPCNotification;
use codex_bench_codex::decode_legacy_notification;
use codex_bench_core::{
    BenchmarkResearchProfile, CampaignManifest, ClaimCatalogEntry, ClaimEvidence, DatasetRecord,
    ProbeSummary, RunSummary, SelectedInstance, StudyArchitectureSubsystem,
    artifact_inventory_for_attempt,
    artifact_map_for_attempt, read_json, read_jsonl_values, write_json_pretty,
};
use codex_bench_probes::derive_run_outputs;
use codex_protocol::protocol::{Event, StudyProbeEvent};
use serde_json::Value;

const REPORT_OBSERVABILITY_CONTRACT_VERSION: &str = "codex-observability.v3";

#[derive(Debug, Clone)]
struct ClaimDescriptor {
    source: String,
    text: String,
    operationalization: String,
}

#[derive(Debug, Clone)]
struct RunReportBundle {
    selected: SelectedInstance,
    record: DatasetRecord,
    summary: RunSummary,
    probe_summary: ProbeSummary,
    claim_evidence: Vec<ClaimEvidence>,
    artifact_paths: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Clone, Default)]
struct OfficialGradingOverview {
    status: String,
    resolved_instances: usize,
    unresolved_instances: usize,
    error_instances: usize,
    completed_instances: usize,
    summary_path: Option<PathBuf>,
}

pub fn render_campaign_report(campaign_dir: &Path) -> Result<PathBuf> {
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    let architecture_map: Vec<StudyArchitectureSubsystem> =
        read_json(&campaign_dir.join("codex-architecture-map.json"))?;
    let grounding_claims: Vec<ClaimCatalogEntry> =
        read_json(&campaign_dir.join("grounding-claims.json"))?;
    let codex_claims: Vec<ClaimCatalogEntry> =
        read_json(&campaign_dir.join("codex-unique-claims.json"))?;
    let benchmark_research_profile: Option<BenchmarkResearchProfile> = manifest
        .benchmark_research_profile_path
        .as_ref()
        .filter(|path| path.exists())
        .and_then(|path| read_json(path).ok());

    let mut bundles = Vec::new();
    for selected in &manifest.selected_instances {
        let attempt_dir = selected.run_dir.join("attempt-01");
        let summary_path = attempt_dir.join("run-summary.json");
        if !summary_path.exists() {
            continue;
        }
        let record: DatasetRecord = read_json(&selected.run_dir.join("record.json"))?;
        let summary: RunSummary = ensure_attempt_derivations(
            &attempt_dir,
            &record,
            &selected.task_class,
            &format!("{}-attempt-01", selected.instance_id),
        )?;
        let _ = render_run_evidence(&attempt_dir, &record, &summary)?;
        let refreshed_summary: RunSummary = read_json(&summary_path)?;
        let probe_summary: ProbeSummary = read_json(&attempt_dir.join("probe-summary.json"))?;
        let claim_evidence: Vec<ClaimEvidence> =
            read_json(&attempt_dir.join("claim-evidence.json")).unwrap_or_default();
        let artifact_paths = artifact_map_for_attempt(&attempt_dir);
        bundles.push(RunReportBundle {
            selected: selected.clone(),
            record,
            summary: refreshed_summary,
            probe_summary,
            claim_evidence,
            artifact_paths,
        });
    }
    bundles.sort_by(|a, b| {
        severity_rank(&a.summary.status)
            .cmp(&severity_rank(&b.summary.status))
            .then_with(|| a.selected.instance_id.cmp(&b.selected.instance_id))
    });

    let report = render_campaign_report_text(
        campaign_dir,
        &manifest,
        &architecture_map,
        benchmark_research_profile.as_ref(),
        &grounding_claims,
        &codex_claims,
        &load_official_grading_overview(campaign_dir),
        &bundles,
    );
    let report_path = campaign_dir.join("reports").join("report.txt");
    fs::create_dir_all(report_path.parent().expect("report path has parent"))?;
    fs::write(&report_path, report)?;
    write_supporting_reports(
        campaign_dir,
        &manifest,
        benchmark_research_profile.as_ref(),
        &bundles,
    )?;
    write_datasets(campaign_dir, &manifest, &bundles)?;
    Ok(report_path)
}

fn ensure_attempt_derivations(
    attempt_dir: &Path,
    record: &DatasetRecord,
    task_class: &str,
    run_id: &str,
) -> Result<RunSummary> {
    let summary_path = attempt_dir.join("run-summary.json");
    let current_summary: RunSummary = read_json(&summary_path)?;
    let turn_metrics_missing = !attempt_dir.join("turn-metrics.jsonl").exists();
    let skill_events_missing = !attempt_dir.join("skill-events.jsonl").exists();
    let message_metrics_missing = !attempt_dir.join("message-metrics.jsonl").exists();
    let coupling_missing = !attempt_dir.join("verbosity-tool-coupling.jsonl").exists();
    let message_rows =
        read_jsonl_values(&attempt_dir.join("message-metrics.jsonl")).unwrap_or_default();
    let message_schema_outdated = message_rows
        .iter()
        .find(|value| value.get("messageId").is_some())
        .map(|value| value.get("hedgingScoreBps").is_none())
        .unwrap_or(true);
    let tool_rows = read_jsonl_values(&attempt_dir.join("tool-events.jsonl")).unwrap_or_default();
    let tool_schema_outdated = tool_rows
        .iter()
        .find(|value| value.get("phase").and_then(Value::as_str) == Some("begin"))
        .map(|value| value.get("toolRoute").is_none())
        .unwrap_or(true);
    let observability_version_outdated =
        current_summary.observability_contract_version.as_deref()
            != Some(REPORT_OBSERVABILITY_CONTRACT_VERSION);

    if !turn_metrics_missing
        && !skill_events_missing
        && !message_metrics_missing
        && !coupling_missing
        && !message_schema_outdated
        && !tool_schema_outdated
        && !observability_version_outdated
    {
        return Ok(current_summary);
    }

    let raw_agent_rows = read_jsonl_values(&attempt_dir.join("raw-agent-events.jsonl"))?;
    let mut decoded_events = Vec::<Event>::new();
    for row in raw_agent_rows {
        let notification: JSONRPCNotification = serde_json::from_value(row)?;
        if let Some(decoded) = decode_legacy_notification(notification)? {
            decoded_events.push(decoded);
        }
    }

    let raw_probe_rows = read_jsonl_values(&attempt_dir.join("codex-probe-events.jsonl")).unwrap_or_default();
    let probe_events = raw_probe_rows
        .into_iter()
        .map(serde_json::from_value::<StudyProbeEvent>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let raw_diagnostics = read_jsonl_values(&attempt_dir.join("raw-diagnostics.jsonl")).unwrap_or_default();
    let patch_text = fs::read(attempt_dir.join("patch.diff")).unwrap_or_default();

    derive_run_outputs(
        attempt_dir,
        run_id,
        task_class,
        record,
        &decoded_events,
        &probe_events,
        &raw_diagnostics,
        &patch_text,
    )
}

pub fn render_single_run_replay(run_dir: &Path) -> Result<PathBuf> {
    let attempt_dir = if run_dir.ends_with("attempt-01") {
        run_dir.to_path_buf()
    } else {
        run_dir.join("attempt-01")
    };
    let record: DatasetRecord = read_json(
        &attempt_dir
            .parent()
            .ok_or_else(|| anyhow!("attempt dir had no parent"))?
            .join("record.json"),
    )?;
    let summary: RunSummary = read_json(&attempt_dir.join("run-summary.json"))?;
    let probe_summary: ProbeSummary = read_json(&attempt_dir.join("probe-summary.json"))?;
    let path = attempt_dir.join("replay.txt");
    let mut lines = Vec::new();
    lines.push("Run Replay".to_string());
    lines.push("==========".to_string());
    lines.push(format!("Instance: {}", record.instance_id));
    lines.push(format!("Repo: {}", record.repo));
    lines.push(format!("Status: {}", summary.status));
    lines.push(format!(
        "Tokens: input={} output={} cache_read={} total={}",
        summary.total_input_tokens.unwrap_or_default(),
        summary.total_output_tokens.unwrap_or_default(),
        summary.total_cache_read_tokens.unwrap_or_default(),
        summary.total_tokens.unwrap_or_default()
    ));
    lines.push(format!(
        "Patch SHA256: {}",
        summary.patch_sha256.clone().unwrap_or_else(|| "-".to_string())
    ));
    lines.push(String::new());
    lines.push("Probe Highlights".to_string());
    lines.push("----------------".to_string());
    lines.push(format!(
        "first_meaningful_edit_tokens={:?}",
        probe_summary.first_meaningful_edit_tokens
    ));
    lines.push(format!(
        "first_verification_tokens={:?}",
        probe_summary.first_verification_tokens
    ));
    lines.push(format!("compaction_count={}", probe_summary.compaction_count));
    lines.push(format!(
        "config_freeze_drift_count={}",
        probe_summary.config_freeze_drift_count
    ));
    lines.push(String::new());
    lines.push("Artifacts".to_string());
    lines.push("---------".to_string());
    for (name, path_ref) in artifact_map_for_attempt(&attempt_dir) {
        lines.push(format!("{name}: {}", path_ref.display()));
    }
    fs::write(&path, lines.join("\n"))?;
    Ok(path)
}

pub fn render_run_evidence(
    attempt_dir: &Path,
    record: &DatasetRecord,
    summary: &RunSummary,
) -> Result<PathBuf> {
    let probe_summary: ProbeSummary = read_json(&attempt_dir.join("probe-summary.json"))?;
    let claim_evidence: Vec<ClaimEvidence> = read_json(&attempt_dir.join("claim-evidence.json"))?;
    let grade_events = read_jsonl_values(&attempt_dir.join("grade-events.jsonl")).unwrap_or_default();
    let turn_metrics = read_jsonl_values(&attempt_dir.join("turn-metrics.jsonl")).unwrap_or_default();
    let message_metrics =
        read_jsonl_values(&attempt_dir.join("message-metrics.jsonl")).unwrap_or_default();
    let token_snapshots =
        read_jsonl_values(&attempt_dir.join("token-snapshots.jsonl")).unwrap_or_default();
    let command_events =
        read_jsonl_values(&attempt_dir.join("command-events.jsonl")).unwrap_or_default();
    let tool_events = read_jsonl_values(&attempt_dir.join("tool-events.jsonl")).unwrap_or_default();
    let skill_events =
        read_jsonl_values(&attempt_dir.join("skill-events.jsonl")).unwrap_or_default();
    let probe_events = read_jsonl_values(&attempt_dir.join("probe-events.jsonl")).unwrap_or_default();
    let verbosity_tool_coupling =
        read_jsonl_values(&attempt_dir.join("verbosity-tool-coupling.jsonl")).unwrap_or_default();
    let lifecycle_events =
        read_jsonl_values(&attempt_dir.join("lifecycle-events.jsonl")).unwrap_or_default();
    let anomaly_events = read_jsonl_values(&attempt_dir.join("anomalies.jsonl")).unwrap_or_default();
    let mut lines = Vec::new();
    lines.push("Run Summary".to_string());
    lines.push("===========".to_string());
    lines.push(format!("Instance: {}", record.instance_id));
    lines.push(format!("Repo: {}", record.repo));
    lines.push(format!(
        "Model / Provider: {} / {}",
        summary.model.clone().unwrap_or_else(|| "-".to_string()),
        summary.provider.clone().unwrap_or_else(|| "-".to_string())
    ));
    lines.push(format!(
        "Cohort / Personality / PromptStyle: {} / {} / {}",
        summary.cohort_id.clone().unwrap_or_else(|| "-".to_string()),
        summary
            .personality_mode
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        summary.prompt_style.clone().unwrap_or_else(|| "-".to_string())
    ));
    lines.push(format!("Task class: {}", summary.task_class));
    lines.push(format!("Status: {}", summary.status));
    lines.push(format!("Grading status: {}", summary.grading_status));
    lines.push(format!("Turns observed: {}", summary.turn_count));
    lines.push(format!(
        "Tokens: input={} output={} cache_read={} total={}",
        summary.total_input_tokens.unwrap_or_default(),
        summary.total_output_tokens.unwrap_or_default(),
        summary.total_cache_read_tokens.unwrap_or_default(),
        summary.total_tokens.unwrap_or_default()
    ));
    lines.push(format!(
        "Patch SHA256: {}",
        summary.patch_sha256.clone().unwrap_or_else(|| "-".to_string())
    ));
    lines.push(String::new());
    lines.push("Human-Oriented Overview".to_string());
    lines.push("=======================".to_string());
    lines.push(format!(
        "Commands={} | Tools={} | SkillEvents={} | MessageMetrics={} | TokenSnapshots={} | Anomalies={}",
        summary.command_count,
        summary.tool_count,
        summary.skill_event_count,
        summary.message_metric_count,
        summary.token_snapshot_count,
        summary.anomaly_count
    ));
    lines.push(format!(
        "Tool kinds: {}",
        render_count_map(&summary.tool_kind_counts)
    ));
    lines.push(format!(
        "Observed skills: {}",
        render_count_map(&summary.skill_name_counts)
    ));
    lines.push(String::new());
    lines.push("Visible Output Summary".to_string());
    lines.push("======================".to_string());
    lines.push(format!(
        "visible_output_total_chars={} | visible_output_total_tokens_est={} | sentence_count={} | paragraph_count={} | bullet_count={} | codeblock_count={}",
        summary.visible_output_total_chars,
        summary.visible_output_total_tokens_est,
        summary.visible_output_sentence_count,
        summary.visible_output_paragraph_count,
        summary.visible_output_bullet_count,
        summary.visible_output_codeblock_count
    ));
    lines.push(format!(
        "per_turn_tokens_est={:?} | per_tool_call_tokens_est={:?} | per_patch_event_tokens_est={:?} | per_verification_event_tokens_est={:?}",
        summary.visible_output_per_turn_tokens_est,
        summary.visible_output_per_tool_call_tokens_est,
        summary.visible_output_per_patch_event_tokens_est,
        summary.visible_output_per_verification_event_tokens_est
    ));
    lines.push(format!(
        "actionable_ratio_bps={:?} | tool_grounded_ratio_bps={:?} | verification_grounded_ratio_bps={:?} | restatement_ratio_bps={:?} | redundant_ratio_bps={:?} | social_tone_ratio_bps={:?}",
        probe_summary.actionable_commentary_ratio_bps,
        probe_summary.tool_grounded_commentary_ratio_bps,
        probe_summary.verification_grounded_commentary_ratio_bps,
        probe_summary.restatement_ratio_bps,
        probe_summary.redundant_commentary_ratio_bps,
        probe_summary.social_tone_ratio_bps
    ));
    lines.push(String::new());
    lines.push("Probe Summary".to_string());
    lines.push("=============".to_string());
    lines.push(format!(
        "first_controlled_change_tokens={:?}",
        probe_summary.first_controlled_change_tokens
    ));
    lines.push(format!(
        "ignition_shell_search_count={}",
        probe_summary.ignition_shell_search_count
    ));
    lines.push(format!(
        "ignition_patch_apply_count={}",
        probe_summary.ignition_patch_apply_count
    ));
    lines.push(format!(
        "ignition_tool_mediated_count={}",
        probe_summary.ignition_tool_mediated_count
    ));
    lines.push(format!(
        "control_rod_compaction_count={}",
        probe_summary.control_rod_compaction_count
    ));
    lines.push(format!(
        "control_rod_config_freeze_count={}",
        probe_summary.control_rod_config_freeze_count
    ));
    lines.push(format!(
        "control_rod_persistence_count={}",
        probe_summary.control_rod_persistence_count
    ));
    lines.push(format!(
        "persistence_continuity_count={}",
        probe_summary.persistence_continuity_count
    ));
    lines.push(format!(
        "persistence_staleness_risk_count={}",
        probe_summary.persistence_staleness_risk_count
    ));
    lines.push(format!(
        "externalized_coordination_count={}",
        probe_summary.externalized_coordination_count
    ));
    lines.push(format!(
        "event_discontinuity_count={}",
        probe_summary.event_discontinuity_count
    ));
    lines.push(format!(
        "containment_heat_leak_count={}",
        probe_summary.containment_heat_leak_count
    ));
    lines.push(format!(
        "verification_closure_count={}",
        probe_summary.verification_closure_count
    ));
    lines.push(format!(
        "useful_token_proxy_bps={:?}",
        probe_summary.useful_token_proxy_bps
    ));
    lines.push(format!(
        "friction_token_proxy_bps={:?}",
        probe_summary.friction_token_proxy_bps
    ));
    lines.push(format!(
        "harness_overhead_proxy_bps={:?}",
        probe_summary.harness_overhead_proxy_bps
    ));
    lines.push(format!(
        "tool_burst_count={} | silent_tool_burst_count={} | micro_narrated_tool_burst_count={}",
        probe_summary.tool_burst_count,
        probe_summary.silent_tool_burst_count,
        probe_summary.micro_narrated_tool_burst_count
    ));
    lines.push(String::new());
    lines.push("Turn Metrics".to_string());
    lines.push("============".to_string());
    if turn_metrics.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for turn in &turn_metrics {
            lines.push(format_turn_metric(turn));
        }
    }
    lines.push(String::new());
    lines.push("Message Metrics".to_string());
    lines.push("===============".to_string());
    if message_metrics.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for message in &message_metrics {
            lines.push(format_message_metric(message));
        }
    }
    lines.push(String::new());
    lines.push("Skill Usage".to_string());
    lines.push("===========".to_string());
    if skill_events.is_empty() {
        lines.push("<none observed>".to_string());
    } else {
        for skill in &skill_events {
            lines.push(format_skill_event(skill));
        }
    }
    lines.push(String::new());
    lines.push("Tool Usage".to_string());
    lines.push("==========".to_string());
    if tool_events.is_empty() {
        lines.push("<none observed>".to_string());
    } else {
        for tool in &tool_events {
            lines.push(format_tool_event(tool));
        }
    }
    lines.push(String::new());
    lines.push("Verbosity × Tool Coupling".to_string());
    lines.push("=========================".to_string());
    if verbosity_tool_coupling.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for row in &verbosity_tool_coupling {
            lines.push(format_coupling_row(row));
        }
    }
    lines.push(String::new());
    lines.push("Token Timeline".to_string());
    lines.push("==============".to_string());
    if token_snapshots.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for snapshot in &token_snapshots {
            lines.push(format_token_snapshot(snapshot));
        }
    }
    lines.push(String::new());
    lines.push("Command Timeline".to_string());
    lines.push("================".to_string());
    if command_events.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for command in &command_events {
            lines.push(format_command_event(command));
        }
    }
    lines.push(String::new());
    lines.push("Session / Config Probe Highlights".to_string());
    lines.push("=================================".to_string());
    lines.push(format!("config_freeze_drift_count={}", probe_summary.config_freeze_drift_count));
    lines.push(format!("instruction_shift_count={}", probe_summary.instruction_shift_count));
    lines.push(format!("harness_friction_count={}", probe_summary.harness_friction_count));
    lines.push(String::new());
    lines.push("Instruction Assembly Summary".to_string());
    lines.push("============================".to_string());
    for (subsystem, count) in &summary.probe_subsystem_counts {
        lines.push(format!("{subsystem}: {count}"));
    }
    lines.push(String::new());
    lines.push("Turn and Phase Timeline".to_string());
    lines.push("=======================".to_string());
    if lifecycle_events.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for event in &lifecycle_events {
            lines.push(format_lifecycle_event(event));
        }
    }
    lines.push(String::new());
    lines.push("Compaction / Reconstruction Timeline".to_string());
    lines.push("===================================".to_string());
    lines.push(format!("compaction_count={}", probe_summary.compaction_count));
    lines.push(format!("compaction_rediscovery_count={}", probe_summary.compaction_rediscovery_count));
    lines.push(format!("peak_context_utilization_bps={:?}", probe_summary.peak_context_utilization_bps));
    lines.push(String::new());
    lines.push("Derived Probe Events".to_string());
    lines.push("====================".to_string());
    if probe_events.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for probe in &probe_events {
            lines.push(format_probe_event(probe));
        }
    }
    lines.push(String::new());
    lines.push("Redundancy Incidents".to_string());
    lines.push("====================".to_string());
    lines.push(format!("repeated_read_count={}", probe_summary.repeated_read_count));
    lines.push(format!("repeated_verification_count={}", probe_summary.repeated_verification_count));
    lines.push(format!("repeated_git_inspection_count={}", probe_summary.repeated_git_inspection_count));
    lines.push(format!("post_submit_activity_count={}", probe_summary.post_submit_activity_count));
    lines.push(String::new());
    lines.push("Verification Chain".to_string());
    lines.push("==================".to_string());
    lines.push(format!("first_meaningful_edit_tokens={:?}", probe_summary.first_meaningful_edit_tokens));
    lines.push(format!("first_verification_tokens={:?}", probe_summary.first_verification_tokens));
    lines.push(format!("first_patch_tokens={:?}", probe_summary.first_patch_tokens));
    lines.push(format!("final_patch_tokens={:?}", probe_summary.final_patch_tokens));
    lines.push(format!("useful_step_proxy={}/{}", probe_summary.useful_step_proxy_num, probe_summary.useful_step_proxy_den));
    lines.push(format!("useful_token_proxy={}/{}", probe_summary.useful_token_proxy_num, probe_summary.useful_token_proxy_den));
    lines.push(String::new());
    lines.push("Anomalies".to_string());
    lines.push("=========".to_string());
    if anomaly_events.is_empty() {
        lines.push("<none>".to_string());
    } else {
        for anomaly in &anomaly_events {
            lines.push(format_anomaly_event(anomaly));
        }
    }
    lines.push(String::new());
    lines.push("Official Grading".to_string());
    lines.push("================".to_string());
    if grade_events.is_empty() {
        lines.push("<missing>".to_string());
    } else {
        for event in &grade_events {
            lines.push(format_grade_event(event));
        }
    }
    lines.push(String::new());
    lines.push("Failure Or Success Narrative".to_string());
    lines.push("============================".to_string());
    lines.push(format!(
        "anomaly_count={} raw_event_count={} raw_probe_count={} raw_diagnostic_count={}",
        summary.anomaly_count, summary.raw_event_count, summary.raw_probe_count, summary.raw_diagnostic_count
    ));
    lines.push(format!(
        "chain_reaction_cycle_count={} containment_breach_count={}",
        probe_summary.chain_reaction_cycle_count, probe_summary.containment_breach_count
    ));
    lines.push(String::new());
    lines.push("Claim Evidence".to_string());
    lines.push("==============".to_string());
    for claim in &claim_evidence {
        lines.push(format!("{} | {}", claim.claim_id, claim.label));
        if !claim.supporting_evidence.is_empty() {
            lines.push(format!("  support: {}", claim.supporting_evidence.join("; ")));
        }
        if !claim.conflicting_evidence.is_empty() {
            lines.push(format!("  conflict: {}", claim.conflicting_evidence.join("; ")));
        }
        if !claim.caveats.is_empty() {
            lines.push(format!("  caveats: {}", claim.caveats.join("; ")));
        }
    }
    lines.push(String::new());
    lines.push("Artifact Paths".to_string());
    lines.push("==============".to_string());
    for (name, path_ref) in artifact_map_for_attempt(attempt_dir) {
        lines.push(format!("{name}: {}", path_ref.display()));
    }
    let path = attempt_dir.join("run-evidence.txt");
    fs::write(&path, lines.join("\n"))?;
    let _ = render_attempt_log(
        attempt_dir,
        record,
        summary,
        &turn_metrics,
        &command_events,
        &tool_events,
        &skill_events,
        &lifecycle_events,
        &anomaly_events,
    )?;
    let mut refreshed = summary.clone();
    refreshed.artifact_inventory = artifact_inventory_for_attempt(attempt_dir);
    write_json_pretty(&attempt_dir.join("run-summary.json"), &refreshed)?;
    Ok(path)
}

fn render_campaign_report_text(
    campaign_dir: &Path,
    manifest: &CampaignManifest,
    architecture_map: &[StudyArchitectureSubsystem],
    benchmark_research_profile: Option<&BenchmarkResearchProfile>,
    grounding_claims: &[ClaimCatalogEntry],
    codex_claims: &[ClaimCatalogEntry],
    grading_overview: &OfficialGradingOverview,
    bundles: &[RunReportBundle],
) -> String {
    let mut lines = Vec::new();
    lines.push("Study Header".to_string());
    lines.push("============".to_string());
    lines.push(format!("Campaign: {}", manifest.campaign_id));
    lines.push(format!("Campaign status: {}", manifest.campaign_status));
    lines.push(format!(
        "Report stage: {}",
        if manifest.campaign_status.contains("graded") {
            "已评分报告"
        } else {
            "求解后报告"
        }
    ));
    lines.push(format!(
        "Experiment: {} ({})",
        display_or(&manifest.experiment_name, &manifest.campaign_id),
        display_or(&manifest.experiment_id, "legacy-campaign")
    ));
    lines.push(format!("Created: {}", manifest.created_at));
    lines.push(format!(
        "Benchmark: {} ({})",
        manifest.benchmark_name, manifest.benchmark_adapter
    ));
    lines.push(format!(
        "Preset: {} [{}]",
        manifest.preset_name,
        manifest
            .stage_name
            .clone()
            .unwrap_or_else(|| "stage-unspecified".to_string())
    ));
    lines.push(format!("Default model/provider: {} via {}", manifest.model, manifest.provider));
    lines.push(format!(
        "Comparison axes: {}",
        if manifest.comparison_axes.is_empty() {
            "-".to_string()
        } else {
            manifest.comparison_axes.join(", ")
        }
    ));
    lines.push("Cohorts:".to_string());
    for cohort in &manifest.cohorts {
        lines.push(format!(
            "- {} | {} | model={} provider={} personality={} prompt_style={}",
            cohort.cohort_id,
            cohort.label,
            cohort.model,
            cohort.provider,
            cohort
                .personality_mode
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            cohort.prompt_style.clone().unwrap_or_else(|| "-".to_string())
        ));
    }
    lines.push(format!("Study mode: {}", manifest.study_mode));
    lines.push(format!("Probe profile: {}", manifest.probe_profile));
    lines.push(format!("Report profile: {}", manifest.report_profile));
    lines.push(format!("Artifact root: {}", campaign_dir.display()));
    lines.push("Grounding docs:".to_string());
    for doc in &manifest.grounding_documents {
        lines.push(format!("- {doc}"));
    }
    lines.push("Reference docs:".to_string());
    for doc in &manifest.reference_documents {
        lines.push(format!("- {doc}"));
    }
    if let Some(profile) = benchmark_research_profile {
        lines.push(format!(
            "Benchmark research profile: {}",
            manifest
                .benchmark_research_profile_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string())
        ));
        lines.push(format!("Benchmark profile summary: {}", profile.summary));
    }
    lines.push(String::new());

    lines.push("Codex Architecture Under Observation".to_string());
    lines.push("===================================".to_string());
    for subsystem in architecture_map {
        lines.push(format!("{}: {}", subsystem.id, subsystem.purpose));
        lines.push(format!("  files: {}", subsystem.files.join(", ")));
        lines.push(format!("  reference_docs: {}", subsystem.reference_docs.join(", ")));
        lines.push(format!("  visible_events: {}", subsystem.visible_events.join(", ")));
        lines.push(format!("  hidden_state: {}", subsystem.hidden_state.join(", ")));
        lines.push(format!("  probes: {}", subsystem.probes.join(", ")));
    }
    lines.push(String::new());

    lines.push("Experimental Setup".to_string());
    lines.push("==================".to_string());
    lines.push(format!("Sample size: {}", manifest.sample_size));
    lines.push(format!("Seed: {}", manifest.seed));
    lines.push(format!("Preset path: {}", manifest.preset_path.display()));
    lines.push(format!(
        "Required task classes: {}",
        if manifest.required_task_classes.is_empty() {
            "-".to_string()
        } else {
            manifest.required_task_classes.join(", ")
        }
    ));
    lines.push(format!(
        "Preferred task classes: {}",
        if manifest.preferred_task_classes.is_empty() {
            "-".to_string()
        } else {
            manifest.preferred_task_classes.join(", ")
        }
    ));
    lines.push(format!(
        "Future benchmark targets: {}",
        if manifest.future_benchmarks.is_empty() {
            "-".to_string()
        } else {
            manifest.future_benchmarks.join(", ")
        }
    ));
    lines.push(format!(
        "Task classes: {}",
        bundles
            .iter()
            .map(|bundle| bundle.selected.task_class.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ")
    ));
    lines.push("macOS constraints: this study path is intentionally local-only and assumes a Mac-hosted Codex workspace.".to_string());
    lines.push("Validity note: SWE-bench tasks are used as live stimuli for Codex behavior rather than as the sole endpoint metric.".to_string());
    if let Some(profile) = benchmark_research_profile {
        lines.push("Benchmark research notes:".to_string());
        for note in &profile.benchmark_notes {
            lines.push(format!("- {note}"));
        }
    }
    lines.push(String::new());

    let mut total_input = 0i64;
    let mut total_output = 0i64;
    let mut total_cache = 0i64;
    let mut total_commands = 0usize;
    let mut total_tools = 0usize;
    let mut total_turns = 0usize;
    let mut total_skill_events = 0usize;
    let mut total_messages = 0usize;
    let mut total_anomalies = 0usize;
    let mut total_visible_chars = 0usize;
    let mut total_visible_tokens_est = 0i64;
    let mut artifact_missing = BTreeMap::<String, usize>::new();
    let mut aggregate_probe_codes = BTreeMap::<String, usize>::new();
    let mut aggregate_subsystems = BTreeMap::<String, usize>::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut task_classes = BTreeMap::<String, usize>::new();
    let mut aggregate_tool_kinds = BTreeMap::<String, usize>::new();
    let mut aggregate_skill_names = BTreeMap::<String, usize>::new();
    let mut aggregate_message_categories = BTreeMap::<String, usize>::new();
    let mut aggregate_cohorts = BTreeMap::<String, usize>::new();
    let mut task_class_probe_rows = BTreeMap::<String, Vec<String>>::new();
    let mut total_control_rods = 0usize;
    let mut total_externalized_coordination = 0usize;
    let mut total_event_discontinuities = 0usize;
    let mut total_containment_heat = 0usize;
    let mut total_persistence_staleness = 0usize;

    for bundle in bundles {
        total_input += bundle.summary.total_input_tokens.unwrap_or_default();
        total_output += bundle.summary.total_output_tokens.unwrap_or_default();
        total_cache += bundle.summary.total_cache_read_tokens.unwrap_or_default();
        total_commands += bundle.summary.command_count;
        total_tools += bundle.summary.tool_count;
        total_turns += bundle.summary.turn_count;
        total_skill_events += bundle.summary.skill_event_count;
        total_messages += bundle.summary.message_metric_count;
        total_anomalies += bundle.summary.anomaly_count;
        total_visible_chars += bundle.summary.visible_output_total_chars;
        total_visible_tokens_est += bundle.summary.visible_output_total_tokens_est;
        *statuses.entry(bundle.summary.status.clone()).or_default() += 1;
        *task_classes.entry(bundle.summary.task_class.clone()).or_default() += 1;
        *aggregate_cohorts
            .entry(bundle.selected.cohort_id.clone())
            .or_default() += 1;
        total_control_rods += bundle.probe_summary.control_rod_compaction_count
            + bundle.probe_summary.control_rod_config_freeze_count
            + bundle.probe_summary.control_rod_persistence_count;
        total_externalized_coordination += bundle.probe_summary.externalized_coordination_count;
        total_event_discontinuities += bundle.probe_summary.event_discontinuity_count;
        total_containment_heat += bundle.probe_summary.containment_heat_leak_count;
        total_persistence_staleness += bundle.probe_summary.persistence_staleness_risk_count;
        task_class_probe_rows
            .entry(bundle.summary.task_class.clone())
            .or_default()
            .push(format!(
                "{}: tokens={} compactions={} closures={} friction={} useful_bps={:?} friction_bps={:?}",
                bundle.selected.instance_id,
                bundle.summary.total_tokens.unwrap_or_default(),
                bundle.probe_summary.compaction_count,
                bundle.probe_summary.verification_closure_count,
                bundle.probe_summary.harness_friction_count,
                bundle.probe_summary.useful_token_proxy_bps,
                bundle.probe_summary.friction_token_proxy_bps
            ));
        for (name, present) in &bundle.summary.artifact_inventory {
            if !present {
                *artifact_missing.entry(name.clone()).or_default() += 1;
            }
        }
        for (code, count) in &bundle.summary.probe_code_counts {
            *aggregate_probe_codes.entry(code.clone()).or_default() += count;
        }
        for (subsystem, count) in &bundle.summary.probe_subsystem_counts {
            *aggregate_subsystems.entry(subsystem.clone()).or_default() += count;
        }
        for (tool_kind, count) in &bundle.summary.tool_kind_counts {
            *aggregate_tool_kinds.entry(tool_kind.clone()).or_default() += count;
        }
        for (skill_name, count) in &bundle.summary.skill_name_counts {
            *aggregate_skill_names.entry(skill_name.clone()).or_default() += count;
        }
        for (category, count) in &bundle.summary.message_category_counts {
            *aggregate_message_categories.entry(category.clone()).or_default() += count;
        }
    }

    lines.push("Telemetry And Artifact Coverage".to_string());
    lines.push("===============================".to_string());
    lines.push(format!("Run status counts: {}", render_count_map(&statuses)));
    lines.push(format!("Task class counts: {}", render_count_map(&task_classes)));
    lines.push(format!("Token totals: input={} output={} cache_read={}", total_input, total_output, total_cache));
    lines.push(format!(
        "Turn totals: {} | Command totals: {} | Tool totals: {} | Skill events: {} | Message metrics: {} | Anomalies: {}",
        total_turns, total_commands, total_tools, total_skill_events, total_messages, total_anomalies
    ));
    lines.push(format!(
        "Visible output totals: chars={} tokens_est={}",
        total_visible_chars, total_visible_tokens_est
    ));
    lines.push(format!("Cohort counts: {}", render_count_map(&aggregate_cohorts)));
    lines.push(format!("Tool kind totals: {}", render_count_map(&aggregate_tool_kinds)));
    lines.push(format!("Skill name totals: {}", render_count_map(&aggregate_skill_names)));
    lines.push(format!(
        "Message category totals: {}",
        render_count_map(&aggregate_message_categories)
    ));
    if artifact_missing.is_empty() {
        lines.push("Artifact coverage: all expected artifacts present in the latest attempts.".to_string());
    } else {
        lines.push(format!("Artifact coverage gaps: {}", render_count_map(&artifact_missing)));
    }
    lines.push(String::new());

    lines.push("Official Grading Overview".to_string());
    lines.push("=========================".to_string());
    lines.push(format!("grader_status={}", grading_overview.status));
    lines.push(format!(
        "completed={} resolved={} unresolved={} errors={}",
        grading_overview.completed_instances,
        grading_overview.resolved_instances,
        grading_overview.unresolved_instances,
        grading_overview.error_instances
    ));
    lines.push(format!(
        "official_summary_path={}",
        grading_overview
            .summary_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    lines.push(String::new());

    lines.push("Observed Codex Harness Behavior".to_string());
    lines.push("===============================".to_string());
    lines.push(format!("Config/session freezing evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "config.")));
    lines.push(format!("Instruction assembly evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "instruction.")));
    lines.push(format!("Context and compaction evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "context.")));
    lines.push(format!("Tool mediation evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "tools.")));
    lines.push(format!("Persistence/reconstruction evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "persistence.")));
    lines.push(format!("Reliability/contention evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "harness.")));
    lines.push(format!("Subsystem totals: {}", render_count_map(&aggregate_subsystems)));
    lines.push(String::new());

    lines.push("Externalized Coordination Lens".to_string());
    lines.push("=============================".to_string());
    lines.push(format!(
        "externalized_coordination_total={total_externalized_coordination}"
    ));
    lines.push(format!(
        "persistence_staleness_risk_total={total_persistence_staleness}"
    ));
    lines.push("This lens asks whether Codex keeps useful continuity through persistence, compaction, and layered instruction channels rather than relying on a single flat transcript.".to_string());
    lines.push(String::new());

    lines.push("Regulation / Control-Rod Signals".to_string());
    lines.push("===============================".to_string());
    lines.push(format!("control_rod_intervention_total={total_control_rods}"));
    lines.push("These are harness-native regulation surfaces: compaction, config freeze, persistence, approval/listener boundaries, and similar stabilizers.".to_string());
    lines.push(String::new());

    lines.push("Containment And Coherence".to_string());
    lines.push("========================".to_string());
    lines.push(format!(
        "event_discontinuity_total={} | containment_heat_leak_total={}",
        total_event_discontinuities, total_containment_heat
    ));
    lines.push("These counters estimate where the harness leaks effort into orchestration overhead or observability gaps rather than direct task progress.".to_string());
    lines.push(String::new());

    lines.push("Task-Behavior Evidence Across Live Tasks".to_string());
    lines.push("=======================================".to_string());
    for bundle in bundles {
        lines.push(format!(
            "{} | status={} | class={} | tokens={} | patch={} | compactions={} | repeated_git={} | repeated_verify={} | config_drift={} | friction={}",
            format!(
                "{} [{} / {} / {}]",
                bundle.selected.instance_id,
                display_or(&bundle.selected.cohort_id, "default"),
                display_or(&bundle.selected.model, &manifest.model),
                bundle
                    .selected
                    .personality_mode
                    .clone()
                    .unwrap_or_else(|| manifest.personality_mode.clone().unwrap_or_else(|| "-".to_string()))
            ),
            bundle.summary.status,
            bundle.summary.task_class,
            bundle.summary.total_tokens.unwrap_or_default(),
            bundle.summary.patch_sha256.clone().unwrap_or_else(|| "-".to_string()),
            bundle.probe_summary.compaction_count,
            bundle.probe_summary.repeated_git_inspection_count,
            bundle.probe_summary.repeated_verification_count,
            bundle.probe_summary.config_freeze_drift_count,
            bundle.probe_summary.harness_friction_count,
        ));
    }
    lines.push(String::new());

    lines.push("Task-Class Evidence Matrix".to_string());
    lines.push("==========================".to_string());
    for (task_class, rows) in task_class_probe_rows {
        lines.push(format!("{task_class}:"));
        for row in rows {
            lines.push(format!("  {row}"));
        }
    }
    lines.push(String::new());

    let claim_map = grounding_claims
        .iter()
        .chain(codex_claims.iter())
        .map(|claim| {
            (
                claim.id.clone(),
                ClaimDescriptor {
                    source: claim.source.clone(),
                    text: claim.text.clone(),
                    operationalization: claim.operationalization.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    lines.push("Claim-by-Claim Evidence".to_string());
    lines.push("=======================".to_string());
    for (claim_id, descriptor) in claim_map {
        let mut claim_rows = Vec::new();
        for bundle in bundles {
            for claim in &bundle.claim_evidence {
                if claim.claim_id == claim_id {
                    claim_rows.push((bundle.selected.instance_id.clone(), claim.clone()));
                }
            }
        }
        lines.push(claim_id.clone());
        lines.push(format!("  source: {}", descriptor.source));
        lines.push(format!("  text: {}", descriptor.text));
        lines.push(format!("  operationalization: {}", descriptor.operationalization));
        if claim_rows.is_empty() {
            lines.push("  evidence: none captured yet".to_string());
        } else {
            for (instance_id, claim) in claim_rows {
                lines.push(format!("  run: {instance_id} -> {}", claim.label));
                if !claim.supporting_evidence.is_empty() {
                    lines.push(format!("    support: {}", claim.supporting_evidence.join("; ")));
                }
                if !claim.conflicting_evidence.is_empty() {
                    lines.push(format!("    conflict: {}", claim.conflicting_evidence.join("; ")));
                }
                if !claim.caveats.is_empty() {
                    lines.push(format!("    caveats: {}", claim.caveats.join("; ")));
                }
            }
        }
    }
    lines.push(String::new());

    lines.push("Where Codex Looks Similar Or Different".to_string());
    lines.push("=====================================".to_string());
    lines.push(format!(
        "Similar to layered-state expectations when: compaction_count_total={} and instruction_channel_probe_total={}",
        aggregate_probe_codes.get("context.compaction").copied().unwrap_or_default(),
        aggregate_probe_codes.get("instruction.channel_mix").copied().unwrap_or_default()
    ));
    lines.push(format!(
        "Potentially unlike pure flat-history assumptions when: config_freeze_drift_count_total={} and persistence_probe_total={}",
        aggregate_probe_codes.get("config.requested_vs_effective").copied().unwrap_or_default(),
        aggregate_probe_codes.get("persistence.resume_path").copied().unwrap_or_default()
    ));
    lines.push(format!(
        "Codex-native harness overhead evidence: {}",
        render_count_map_filtered(&aggregate_probe_codes, "harness.")
    ));
    lines.push(format!(
        "Control-rod evidence: control_rod_total={} | externalized_coordination_total={}",
        total_control_rods, total_externalized_coordination
    ));
    lines.push(String::new());

    lines.push("Threats To Validity".to_string());
    lines.push("===================".to_string());
    lines.push("macOS-only bias: the study currently assumes a local Mac-hosted Codex runtime.".to_string());
    lines.push("SWE-bench-only bias: live tasks are real but not representative of every future workload.".to_string());
    lines.push("Hidden reasoning observability limits: internal chain-of-thought remains unavailable and some evidence is inferred.".to_string());
    lines.push("Harness noise: listener, DB, and translation events can affect observability in ways that are not identical to reasoning failures.".to_string());
    if !artifact_missing.is_empty() {
        lines.push(format!("Current telemetry gaps: {}", render_count_map(&artifact_missing)));
    }
    lines.push(String::new());

    lines.push("Run Index".to_string());
    lines.push("=========".to_string());
    for bundle in bundles {
        lines.push(format!(
            "{} | {} | {} | cohort={} | personality={} | tokens={} | visible_tokens_est={} | probes={} | anomalies={} | evidence={}",
            bundle.selected.instance_id,
            bundle.summary.status,
            bundle.summary.task_class,
            display_or(&bundle.selected.cohort_id, "default"),
            bundle
                .selected
                .personality_mode
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            bundle.summary.total_tokens.unwrap_or_default(),
            bundle.summary.visible_output_total_tokens_est,
            bundle.summary.raw_probe_count,
            bundle.summary.anomaly_count,
            bundle.selected.run_dir.join("attempt-01").join("run-evidence.txt").display()
        ));
        lines.push(format!(
            "  attempt_log={}",
            bundle.selected.run_dir.join("attempt-01").join("attempt-log.txt").display()
        ));
    }
    lines.push(String::new());

    lines.push("Artifact Appendix".to_string());
    lines.push("=================".to_string());
    for bundle in bundles {
        lines.push(format!("{}:", bundle.selected.instance_id));
        lines.push(format!("  record: {}", bundle.record.instance_id));
        for (name, path_ref) in &bundle.artifact_paths {
            lines.push(format!("  {name}: {}", path_ref.display()));
        }
    }

    lines.join("\n")
}

fn write_supporting_reports(
    campaign_dir: &Path,
    manifest: &CampaignManifest,
    benchmark_research_profile: Option<&BenchmarkResearchProfile>,
    bundles: &[RunReportBundle],
) -> Result<()> {
    let reports_dir = campaign_dir.join("reports");
    fs::create_dir_all(&reports_dir)?;

    let mut model_comparison = Vec::new();
    model_comparison.push("# 模型对比".to_string());
    model_comparison.push(String::new());
    model_comparison.push(format!(
        "实验：{} ({})",
        display_or(&manifest.experiment_name, &manifest.campaign_id),
        display_or(&manifest.experiment_id, "legacy-campaign")
    ));
    model_comparison.push(String::new());
    for bundle in bundles {
        model_comparison.push(format!(
            "- `{}` / `{}` / `{}` / `{}`: total_tokens={}, visible_output_tokens_est={}, tool_count={}, command_count={}, verification_closure_count={}",
            bundle.selected.instance_id,
            display_or(&bundle.selected.cohort_id, "default"),
            display_or(&bundle.selected.model, &manifest.model),
            bundle.selected.personality_mode.clone().unwrap_or_else(|| "-".to_string()),
            bundle.summary.total_tokens.unwrap_or_default(),
            bundle.summary.visible_output_total_tokens_est,
            bundle.summary.tool_count,
            bundle.summary.command_count,
            bundle.probe_summary.verification_closure_count
        ));
    }
    fs::write(reports_dir.join("model-comparison.md"), model_comparison.join("\n"))?;

    let mut verbosity = Vec::new();
    verbosity.push("# 可见输出与冗长度分析".to_string());
    verbosity.push(String::new());
    for bundle in bundles {
        verbosity.push(format!(
            "## {} / {}",
            bundle.selected.instance_id, bundle.selected.cohort_id
        ));
        verbosity.push(format!(
            "- model/personality: {}/{}",
            display_or(&bundle.selected.model, &manifest.model),
            bundle.selected.personality_mode.clone().unwrap_or_else(|| "-".to_string())
        ));
        verbosity.push(format!("- visible_output_total_tokens_est: {}", bundle.summary.visible_output_total_tokens_est));
        verbosity.push(format!(
            "- actionable_commentary_ratio_bps: {:?}",
            bundle.probe_summary.actionable_commentary_ratio_bps
        ));
        verbosity.push(format!(
            "- tool_grounded_commentary_ratio_bps: {:?}",
            bundle.probe_summary.tool_grounded_commentary_ratio_bps
        ));
        verbosity.push(format!(
            "- verification_grounded_commentary_ratio_bps: {:?}",
            bundle.probe_summary.verification_grounded_commentary_ratio_bps
        ));
        verbosity.push(format!(
            "- social_tone_ratio_bps: {:?}",
            bundle.probe_summary.social_tone_ratio_bps
        ));
        verbosity.push(String::new());
    }
    fs::write(reports_dir.join("verbosity-analysis.md"), verbosity.join("\n"))?;

    let mut coupling = Vec::new();
    coupling.push("# 语言-工具耦合分析".to_string());
    coupling.push(String::new());
    for bundle in bundles {
        coupling.push(format!(
            "- `{}` / `{}`: tool_burst_count={}, silent_tool_burst_count={}, micro_narrated_tool_burst_count={}, tokens_before_first_tool={:?}",
            bundle.selected.instance_id,
            display_or(&bundle.selected.cohort_id, "default"),
            bundle.probe_summary.tool_burst_count,
            bundle.probe_summary.silent_tool_burst_count,
            bundle.probe_summary.micro_narrated_tool_burst_count,
            bundle.probe_summary.tokens_before_first_tool
        ));
    }
    fs::write(reports_dir.join("tool-language-coupling.md"), coupling.join("\n"))?;

    fs::write(
        reports_dir.join("personality-analysis.md"),
        render_personality_analysis(manifest, bundles),
    )?;
    fs::write(
        reports_dir.join("task-class-analysis.md"),
        render_task_class_analysis(bundles, benchmark_research_profile),
    )?;
    fs::write(
        reports_dir.join("methods-appendix.md"),
        "# 方法附录\n\n本研究以用户可见输出为“说更多”的主观测面，并结合 Codex 原始 probe、tool/patch/verification 时序构造耦合证据。\n",
    )?;
    fs::write(
        reports_dir.join("tool-inventory.md"),
        render_tool_inventory_report(bundles),
    )?;
    fs::write(
        reports_dir.join("tool-route-analysis.md"),
        render_tool_route_report(bundles),
    )?;
    fs::write(
        reports_dir.join("linguistic-profile.md"),
        render_linguistic_profile_report(bundles),
    )?;
    fs::write(
        reports_dir.join("phrase-and-tone-analysis.md"),
        render_phrase_and_tone_report(bundles),
    )?;
    fs::write(
        reports_dir.join("bridge-language-analysis.md"),
        render_bridge_language_report(bundles),
    )?;
    fs::write(
        reports_dir.join("personality-mechanism-analysis.md"),
        render_personality_mechanism_report(bundles),
    )?;
    fs::write(
        reports_dir.join("instruction-stratification-analysis.md"),
        render_instruction_stratification_report(bundles),
    )?;
    fs::write(
        reports_dir.join("cohort-pair-analysis.md"),
        render_cohort_pair_report(bundles),
    )?;
    Ok(())
}

fn render_personality_analysis(manifest: &CampaignManifest, bundles: &[RunReportBundle]) -> String {
    let mut lines = vec!["# personality 分析".to_string(), String::new()];
    lines.push(format!(
        "实验：{} ({})",
        display_or(&manifest.experiment_name, &manifest.campaign_id),
        display_or(&manifest.experiment_id, "legacy-campaign")
    ));
    lines.push(String::new());
    for bundle in bundles {
        lines.push(format!(
            "## {} / {}",
            bundle.selected.instance_id, bundle.selected.cohort_id
        ));
        lines.push(format!(
            "- model/personality: {}/{}",
            display_or(&bundle.selected.model, &manifest.model),
            bundle.selected.personality_mode.as_deref().unwrap_or("-")
        ));
        lines.push(format!(
            "- visible_output_total_tokens_est={} | social_tone_ratio_bps={:?} | tool_burst_count={} | micro_narrated_tool_burst_count={}",
            bundle.summary.visible_output_total_tokens_est,
            bundle.probe_summary.social_tone_ratio_bps,
            bundle.probe_summary.tool_burst_count,
            bundle.probe_summary.micro_narrated_tool_burst_count
        ));
        lines.push(format!(
            "- instruction_shift_count={} | config_freeze_drift_count={}",
            bundle.probe_summary.instruction_shift_count,
            bundle.probe_summary.config_freeze_drift_count
        ));
        lines.push(String::new());
    }
    lines.join("\n")
}

fn render_task_class_analysis(
    bundles: &[RunReportBundle],
    benchmark_research_profile: Option<&BenchmarkResearchProfile>,
) -> String {
    let mut rollup = BTreeMap::<String, (usize, i64, usize, usize, usize, usize, usize, usize)>::new();
    for bundle in bundles {
        let entry = rollup
            .entry(bundle.summary.task_class.clone())
            .or_insert((0, 0, 0, 0, 0, 0, 0, 0));
        entry.0 += 1;
        entry.1 += bundle.summary.visible_output_total_tokens_est;
        entry.2 += bundle.summary.tool_count;
        entry.3 += bundle.summary.command_count;
        entry.4 += bundle.probe_summary.control_rod_compaction_count
            + bundle.probe_summary.control_rod_config_freeze_count
            + bundle.probe_summary.control_rod_persistence_count;
        entry.5 += bundle
            .summary
            .message_category_counts
            .get("tool_bridge_before")
            .copied()
            .unwrap_or_default()
            + bundle
                .summary
                .message_category_counts
                .get("tool_bridge_after")
                .copied()
                .unwrap_or_default();
        entry.6 += bundle
            .summary
            .message_category_counts
            .get("verification_framing")
            .copied()
            .unwrap_or_default()
            + bundle
                .summary
                .message_category_counts
                .get("result_framing")
                .copied()
                .unwrap_or_default();
        entry.7 += bundle.probe_summary.harness_friction_count;
    }
    let mut lines = vec!["# 任务类型分析".to_string(), String::new()];
    for (task_class, (count, visible_tokens, tools, commands, control_rods, bridge_msgs, verification_msgs, harness_friction)) in rollup {
        lines.push(format!("## {}", task_class));
        lines.push(format!(
            "- run_count={} | visible_output_total_tokens_est={} | tool_count={} | command_count={} | control_rod_events={} | bridge_messages={} | verification_messages={} | harness_friction_events={}",
            count, visible_tokens, tools, commands, control_rods, bridge_msgs, verification_msgs, harness_friction
        ));
        if let Some(profile) = benchmark_research_profile
            .and_then(|profile| profile.task_class_profiles.iter().find(|entry| entry.task_class == task_class))
        {
            lines.push(format!(
                "- benchmark_hints: verification_strength={} | context_pressure={} | bootstrap_risk={} | language_need={}",
                profile.expected_verification_strength,
                profile.expected_context_pressure,
                profile.expected_bootstrap_risk,
                profile.expected_language_need
            ));
            lines.push(format!(
                "- expected_tool_mix={}",
                if profile.expected_tool_mix.is_empty() {
                    "-".to_string()
                } else {
                    profile.expected_tool_mix.join(", ")
                }
            ));
            if let Some(hint) = &profile.language_profile_hint {
                lines.push(format!("- language_profile_hint={hint}"));
            }
            if let Some(hint) = &profile.tool_profile_hint {
                lines.push(format!("- tool_profile_hint={hint}"));
            }
            if let Some(hint) = &profile.interaction_style_hint {
                lines.push(format!("- interaction_style_hint={hint}"));
            }
        }
        lines.push(String::new());
    }
    lines
        .into_iter()
        .chain([
            "请结合 `datasets/task_class_summary.csv` 查看每类任务的更细粒度聚合。".to_string(),
        ])
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_tool_inventory_report(bundles: &[RunReportBundle]) -> String {
    let inventory = aggregate_tool_inventory(bundles);
    let mut lines = vec!["# 工具清单".to_string(), String::new()];
    for ((cohort_id, kind, name), stats) in inventory {
        lines.push(format!(
            "- cohort=`{}` kind=`{}` name=`{}` calls={} successes={} failures={} total_output_size={} median_duration_ms={}",
            cohort_id,
            kind,
            name,
            stats.call_count,
            stats.success_count,
            stats.failure_count,
            stats.total_output_size,
            median_i64(&stats.durations_ms)
        ));
    }
    lines.join("\n")
}

fn render_tool_route_report(bundles: &[RunReportBundle]) -> String {
    let routes = aggregate_tool_routes(bundles);
    let mut lines = vec!["# Tool Route 分析".to_string(), String::new()];
    for ((cohort_id, route), count) in routes {
        lines.push(format!("- cohort=`{}` route=`{}` count={}", cohort_id, route, count));
    }
    lines.join("\n")
}

fn render_linguistic_profile_report(bundles: &[RunReportBundle]) -> String {
    let profiles = aggregate_linguistic_profiles(bundles);
    let mut lines = vec!["# 语言画像".to_string(), String::new()];
    for profile in profiles {
        lines.push(format!("## {}", profile.cohort_id));
        lines.push(format!(
            "- top_words: {}",
            render_ranked_terms(&profile.top_words)
        ));
        lines.push(format!(
            "- top_bigrams: {}",
            render_ranked_terms(&profile.top_bigrams)
        ));
        lines.push(format!(
            "- top_trigrams: {}",
            render_ranked_terms(&profile.top_trigrams)
        ));
        lines.push(format!(
            "- discourse_counts: {}",
            render_count_map(&profile.discourse_counts)
        ));
        lines.push(String::new());
    }
    lines.join("\n")
}

fn render_phrase_and_tone_report(bundles: &[RunReportBundle]) -> String {
    let profiles = aggregate_linguistic_profiles(bundles);
    let mut lines = vec!["# 短语与语气分析".to_string(), String::new()];
    for profile in profiles {
        lines.push(format!("## {}", profile.cohort_id));
        lines.push(format!("- social_tone_messages={}", profile.social_tone_messages));
        lines.push(format!("- verification_messages={}", profile.verification_messages));
        lines.push(format!("- tool_bridge_messages={}", profile.tool_bridge_messages));
        lines.push(format!("- planning_messages={}", profile.planning_messages));
        lines.push(String::new());
    }
    lines.join("\n")
}

fn render_bridge_language_report(bundles: &[RunReportBundle]) -> String {
    let profiles = aggregate_linguistic_profiles(bundles);
    let mut lines = vec!["# 桥接语言分析".to_string(), String::new()];
    for profile in profiles {
        lines.push(format!("## {}", profile.cohort_id));
        lines.push(format!(
            "- tool_bridge_before={} | tool_bridge_after={} | verification_framing={} | result_framing={} | decision_explanation={}",
            profile.discourse_counts.get("tool_bridge_before").copied().unwrap_or_default(),
            profile.discourse_counts.get("tool_bridge_after").copied().unwrap_or_default(),
            profile.discourse_counts.get("verification_framing").copied().unwrap_or_default(),
            profile.discourse_counts.get("result_framing").copied().unwrap_or_default(),
            profile.discourse_counts.get("decision_explanation").copied().unwrap_or_default()
        ));
        lines.push(String::new());
    }
    lines.join("\n")
}

fn render_personality_mechanism_report(bundles: &[RunReportBundle]) -> String {
    let mut lines = vec!["# Personality 机制分析".to_string(), String::new()];
    for bundle in bundles {
        lines.push(format!(
            "- `{}` / `{}`: social_tone_ratio_bps={:?}, instruction_shift_count={}, tool_grounded_commentary_ratio_bps={:?}, verification_grounded_commentary_ratio_bps={:?}",
            bundle.selected.instance_id,
            bundle.selected.cohort_id,
            bundle.probe_summary.social_tone_ratio_bps,
            bundle.probe_summary.instruction_shift_count,
            bundle.probe_summary.tool_grounded_commentary_ratio_bps,
            bundle.probe_summary.verification_grounded_commentary_ratio_bps
        ));
    }
    lines.join("\n")
}

fn render_instruction_stratification_report(bundles: &[RunReportBundle]) -> String {
    let mut lines = vec!["# 指令分层分析".to_string(), String::new()];
    for bundle in bundles {
        lines.push(format!(
            "- `{}` / `{}`: instruction_shift_count={}, instruction_stratification_count={}, config_freeze_drift_count={}",
            bundle.selected.instance_id,
            bundle.selected.cohort_id,
            bundle.probe_summary.instruction_shift_count,
            bundle.probe_summary.instruction_stratification_count,
            bundle.probe_summary.config_freeze_drift_count
        ));
    }
    lines.join("\n")
}

fn render_cohort_pair_report(bundles: &[RunReportBundle]) -> String {
    let mut by_pair = BTreeMap::<String, Vec<&RunReportBundle>>::new();
    for bundle in bundles {
        by_pair
            .entry(bundle.selected.paired_instance_key.clone())
            .or_default()
            .push(bundle);
    }
    let mut lines = vec!["# Cohort 成对差分".to_string(), String::new()];
    for (pair_key, mut items) in by_pair {
        items.sort_by(|a, b| a.selected.cohort_id.cmp(&b.selected.cohort_id));
        lines.push(format!("## {}", pair_key));
        for bundle in items {
            lines.push(format!(
                "- {}: visible_output_total_tokens_est={} | tool_count={} | command_count={} | social_tone_ratio_bps={:?}",
                bundle.selected.cohort_id,
                bundle.summary.visible_output_total_tokens_est,
                bundle.summary.tool_count,
                bundle.summary.command_count,
                bundle.probe_summary.social_tone_ratio_bps
            ));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

#[derive(Debug, Clone, Default)]
struct ToolInventoryStats {
    call_count: usize,
    success_count: usize,
    failure_count: usize,
    total_output_size: usize,
    durations_ms: Vec<i64>,
}

#[derive(Debug, Clone, Default)]
struct LinguisticProfile {
    cohort_id: String,
    top_words: Vec<(String, usize)>,
    top_bigrams: Vec<(String, usize)>,
    top_trigrams: Vec<(String, usize)>,
    discourse_counts: BTreeMap<String, usize>,
    social_tone_messages: usize,
    verification_messages: usize,
    tool_bridge_messages: usize,
    planning_messages: usize,
}

#[derive(Debug, Clone)]
struct PairPhraseDeltaRow {
    pair_kind: String,
    left_cohort: String,
    right_cohort: String,
    grouping_key: String,
    term_type: String,
    term: String,
    left_count: usize,
    right_count: usize,
    delta: i64,
}

fn aggregate_tool_inventory(
    bundles: &[RunReportBundle],
) -> BTreeMap<(String, String, String), ToolInventoryStats> {
    let mut inventory = BTreeMap::<(String, String, String), ToolInventoryStats>::new();
    for bundle in bundles {
        let path = bundle.selected.run_dir.join("attempt-01").join("tool-events.jsonl");
        for value in read_jsonl_values(&path).unwrap_or_default() {
            if value.get("phase").and_then(Value::as_str) == Some("begin") {
                let key = (
                    bundle.selected.cohort_id.clone(),
                    value.get("kind").and_then(Value::as_str).unwrap_or("-").to_string(),
                    value.get("name").and_then(Value::as_str).unwrap_or("-").to_string(),
                );
                let stats = inventory.entry(key).or_default();
                stats.call_count += 1;
                if let Some(success) = value.get("success").and_then(Value::as_bool) {
                    if success {
                        stats.success_count += 1;
                    } else {
                        stats.failure_count += 1;
                    }
                }
            } else if value.get("phase").and_then(Value::as_str) == Some("end") {
                let key = (
                    bundle.selected.cohort_id.clone(),
                    value.get("kind").and_then(Value::as_str).unwrap_or("-").to_string(),
                    value.get("name").and_then(Value::as_str).unwrap_or("-").to_string(),
                );
                let stats = inventory.entry(key).or_default();
                if let Some(success) = value.get("success").and_then(Value::as_bool) {
                    if success {
                        stats.success_count += 1;
                    } else {
                        stats.failure_count += 1;
                    }
                }
                stats.total_output_size += value.get("outputSize").and_then(Value::as_u64).unwrap_or_default() as usize;
                if let Some(duration) = value.get("durationMs").and_then(Value::as_i64) {
                    stats.durations_ms.push(duration);
                }
            }
        }
    }
    inventory
}

fn aggregate_tool_routes(bundles: &[RunReportBundle]) -> BTreeMap<(String, String), usize> {
    let mut routes = BTreeMap::new();
    for bundle in bundles {
        let path = bundle.selected.run_dir.join("attempt-01").join("tool-events.jsonl");
        for value in read_jsonl_values(&path).unwrap_or_default() {
            if value.get("phase").and_then(Value::as_str) != Some("begin")
                && value.get("phase").and_then(Value::as_str) != Some("call")
            {
                continue;
            }
            let key = (
                bundle.selected.cohort_id.clone(),
                value.get("toolRoute").and_then(Value::as_str).unwrap_or("-").to_string(),
            );
            *routes.entry(key).or_insert(0usize) += 1;
        }
    }
    routes
}

fn aggregate_linguistic_profiles(bundles: &[RunReportBundle]) -> Vec<LinguisticProfile> {
    let mut by_cohort_words = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut by_cohort_bigrams = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut by_cohort_trigrams = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut by_cohort_discourse = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut by_cohort_social = BTreeMap::<String, usize>::new();
    let mut by_cohort_verification = BTreeMap::<String, usize>::new();
    let mut by_cohort_tool_bridge = BTreeMap::<String, usize>::new();
    let mut by_cohort_planning = BTreeMap::<String, usize>::new();

    for bundle in bundles {
        let path = bundle.selected.run_dir.join("attempt-01").join("message-metrics.jsonl");
        for value in read_jsonl_values(&path).unwrap_or_default() {
            let cohort_id = bundle.selected.cohort_id.clone();
            let text = value.get("message").and_then(Value::as_str).unwrap_or("");
            let tokens = tokenize_for_research(text);
            for token in &tokens {
                *by_cohort_words
                    .entry(cohort_id.clone())
                    .or_default()
                    .entry(token.clone())
                    .or_insert(0) += 1;
            }
            for gram in make_ngrams(&tokens, 2) {
                *by_cohort_bigrams
                    .entry(cohort_id.clone())
                    .or_default()
                    .entry(gram)
                    .or_insert(0) += 1;
            }
            for gram in make_ngrams(&tokens, 3) {
                *by_cohort_trigrams
                    .entry(cohort_id.clone())
                    .or_default()
                    .entry(gram)
                    .or_insert(0) += 1;
            }
            for category in value
                .get("categories")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                *by_cohort_discourse
                    .entry(cohort_id.clone())
                    .or_default()
                    .entry(category.to_string())
                    .or_insert(0) += 1;
                match category {
                    "social_tone" => *by_cohort_social.entry(cohort_id.clone()).or_insert(0) += 1,
                    "verification_framing" | "result_framing" => {
                        *by_cohort_verification.entry(cohort_id.clone()).or_insert(0) += 1
                    }
                    "tool_bridge_before" | "tool_bridge_after" => {
                        *by_cohort_tool_bridge.entry(cohort_id.clone()).or_insert(0) += 1
                    }
                    "planning" => *by_cohort_planning.entry(cohort_id.clone()).or_insert(0) += 1,
                    _ => {}
                }
            }
        }
    }

    let cohort_ids = by_cohort_words
        .keys()
        .chain(by_cohort_discourse.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    cohort_ids
        .into_iter()
        .map(|cohort_id| LinguisticProfile {
            cohort_id: cohort_id.clone(),
            top_words: top_n_terms(by_cohort_words.remove(&cohort_id).unwrap_or_default(), 20),
            top_bigrams: top_n_terms(by_cohort_bigrams.remove(&cohort_id).unwrap_or_default(), 12),
            top_trigrams: top_n_terms(by_cohort_trigrams.remove(&cohort_id).unwrap_or_default(), 8),
            discourse_counts: by_cohort_discourse.remove(&cohort_id).unwrap_or_default(),
            social_tone_messages: by_cohort_social.remove(&cohort_id).unwrap_or_default(),
            verification_messages: by_cohort_verification.remove(&cohort_id).unwrap_or_default(),
            tool_bridge_messages: by_cohort_tool_bridge.remove(&cohort_id).unwrap_or_default(),
            planning_messages: by_cohort_planning.remove(&cohort_id).unwrap_or_default(),
        })
        .collect()
}

fn top_term_map(terms: &[(String, usize)]) -> BTreeMap<String, usize> {
    terms.iter().cloned().collect()
}

fn compute_phrase_deltas(
    bundles: &[RunReportBundle],
    mode: &str,
) -> Vec<PairPhraseDeltaRow> {
    let profiles = aggregate_linguistic_profiles(bundles)
        .into_iter()
        .map(|profile| (profile.cohort_id.clone(), profile))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();

    let mut grouped = BTreeMap::<String, Vec<&RunReportBundle>>::new();
    for bundle in bundles {
        let key = match mode {
            "model" => format!(
                "{}::{}",
                bundle.selected.instance_id,
                bundle
                    .selected
                    .personality_mode
                    .clone()
                    .unwrap_or_else(|| "none".to_string())
            ),
            "personality" => {
                format!("{}::{}", bundle.selected.instance_id, bundle.selected.model)
            }
            _ => continue,
        };
        grouped.entry(key).or_default().push(bundle);
    }

    for (grouping_key, items) in grouped {
        for left_idx in 0..items.len() {
            for right_idx in (left_idx + 1)..items.len() {
                let left = items[left_idx];
                let right = items[right_idx];
                let comparable = match mode {
                    "model" => left.selected.model != right.selected.model
                        && left.selected.personality_mode == right.selected.personality_mode,
                    "personality" => left.selected.model == right.selected.model
                        && left.selected.personality_mode != right.selected.personality_mode,
                    _ => false,
                };
                if !comparable {
                    continue;
                }

                let Some(left_profile) = profiles.get(&left.selected.cohort_id) else {
                    continue;
                };
                let Some(right_profile) = profiles.get(&right.selected.cohort_id) else {
                    continue;
                };

                let lexical_families = vec![
                    ("word", top_term_map(&left_profile.top_words), top_term_map(&right_profile.top_words)),
                    ("bigram", top_term_map(&left_profile.top_bigrams), top_term_map(&right_profile.top_bigrams)),
                    ("trigram", top_term_map(&left_profile.top_trigrams), top_term_map(&right_profile.top_trigrams)),
                ];

                for (term_type, left_map, right_map) in lexical_families {
                    let terms = left_map
                        .keys()
                        .chain(right_map.keys())
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    for term in terms {
                        let left_count = left_map.get(&term).copied().unwrap_or_default();
                        let right_count = right_map.get(&term).copied().unwrap_or_default();
                        if left_count == 0 && right_count == 0 {
                            continue;
                        }
                        rows.push(PairPhraseDeltaRow {
                            pair_kind: mode.to_string(),
                            left_cohort: left.selected.cohort_id.clone(),
                            right_cohort: right.selected.cohort_id.clone(),
                            grouping_key: grouping_key.clone(),
                            term_type: term_type.to_string(),
                            term,
                            left_count,
                            right_count,
                            delta: right_count as i64 - left_count as i64,
                        });
                    }
                }
            }
        }
    }

    rows.sort_by(|a, b| {
        a.grouping_key
            .cmp(&b.grouping_key)
            .then_with(|| a.left_cohort.cmp(&b.left_cohort))
            .then_with(|| a.right_cohort.cmp(&b.right_cohort))
            .then_with(|| a.term_type.cmp(&b.term_type))
            .then_with(|| b.delta.abs().cmp(&a.delta.abs()))
            .then_with(|| a.term.cmp(&b.term))
    });
    rows
}

fn tokenize_for_research(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| token.len() >= 3)
        .filter(|token| !token.chars().any(|ch| ch.is_ascii_digit()))
        .filter(|token| !token.contains("users") && !token.contains("downloads") && !token.contains("codexplusclaw"))
        .filter(|token| !matches!(*token, "the" | "and" | "for" | "with" | "that" | "this" | "then" | "from" | "into" | "have" | "will" | "just" | "about" | "after" | "before" | "using" | "need" | "next" | "let" | "lets" | "our" | "you" | "your" | "are" | "was" | "were" | "has" | "had" | "workspace" | "artifacts" | "runs" | "attempt" | "path" | "file" | "swebench" | "study" | "kevinlin" | "friendly" | "pragmatic" | "gpt"))
        .map(ToString::to_string)
        .collect()
}

fn make_ngrams(tokens: &[String], n: usize) -> Vec<String> {
    if tokens.len() < n {
        return Vec::new();
    }
    tokens
        .windows(n)
        .map(|window| window.join(" "))
        .collect()
}

fn top_n_terms(map: BTreeMap<String, usize>, limit: usize) -> Vec<(String, usize)> {
    let mut terms = map.into_iter().collect::<Vec<_>>();
    terms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    terms.truncate(limit);
    terms
}

fn render_ranked_terms(terms: &[(String, usize)]) -> String {
    if terms.is_empty() {
        return "-".to_string();
    }
    terms
        .iter()
        .map(|(term, count)| format!("{term}({count})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn median_i64(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn display_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn write_datasets(
    campaign_dir: &Path,
    manifest: &CampaignManifest,
    bundles: &[RunReportBundle],
) -> Result<()> {
    let datasets_dir = campaign_dir.join("datasets");
    fs::create_dir_all(&datasets_dir)?;

    let mut campaign_runs = vec![
        "campaign_id,experiment_id,instance_id,paired_instance_key,cohort_id,model,provider,personality_mode,prompt_style,task_class,status,grading_status,total_tokens,visible_output_total_tokens_est,tool_count,command_count,anomaly_count".to_string()
    ];
    let mut claim_rows = vec![
        "instance_id,cohort_id,claim_id,label".to_string()
    ];
    let mut task_class_summary = vec![
        "task_class,run_count,total_tokens,visible_output_total_tokens_est,tool_count,command_count".to_string()
    ];
    let mut task_rollup = BTreeMap::<String, (usize, i64, i64, usize, usize)>::new();
    let mut model_pair_deltas = vec![
        "paired_instance_key,cohort_id,model,personality_mode,total_tokens,visible_output_total_tokens_est,tool_count,command_count".to_string()
    ];
    let mut message_lexical_summary = vec![
        "cohort_id,term_type,term,count".to_string()
    ];
    let mut cohort_lexical_summary = vec![
        "cohort_id,top_word_mass,top_bigram_mass,top_trigram_mass,social_tone_messages,verification_messages,tool_bridge_messages,planning_messages".to_string()
    ];
    let mut message_discourse_summary = vec![
        "cohort_id,category,count".to_string()
    ];
    let mut message_style = vec![
        "instance_id,cohort_id,message_id,turn_id,seq,hedging_score_bps,confidence_score_bps,collaboration_tone_score_bps,directive_score_bps,reflective_score_bps,formality_score_bps,empathy_alignment_score_bps,content_word_count,function_word_count,type_token_ratio_bps,lexical_diversity_score_bps".to_string()
    ];
    let mut model_phrase_deltas = vec![
        "pair_kind,grouping_key,left_cohort,right_cohort,term_type,term,left_count,right_count,delta".to_string()
    ];
    let mut personality_phrase_deltas = vec![
        "pair_kind,grouping_key,left_cohort,right_cohort,term_type,term,left_count,right_count,delta".to_string()
    ];
    let mut tool_inventory = vec![
        "cohort_id,kind,name,call_count,success_count,failure_count,total_output_size,median_duration_ms".to_string()
    ];
    let mut tool_route_summary = vec![
        "cohort_id,tool_route,count".to_string()
    ];
    let mut tool_by_turn = vec![
        "instance_id,cohort_id,turn_id,kind,name,tool_route,phase,seq,call_id,duration_ms,success,preceded_by_commentary_tokens,output_size".to_string()
    ];

    for bundle in bundles {
        campaign_runs.push(csv_row(&[
            &manifest.campaign_id,
            &manifest.experiment_id,
            &bundle.selected.instance_id,
            &bundle.selected.paired_instance_key,
            &bundle.selected.cohort_id,
            &bundle.selected.model,
            &bundle.selected.provider,
            bundle.selected.personality_mode.as_deref().unwrap_or(""),
            bundle.selected.prompt_style.as_deref().unwrap_or(""),
            &bundle.summary.task_class,
            &bundle.summary.status,
            &bundle.summary.grading_status,
            &bundle.summary.total_tokens.unwrap_or_default().to_string(),
            &bundle.summary.visible_output_total_tokens_est.to_string(),
            &bundle.summary.tool_count.to_string(),
            &bundle.summary.command_count.to_string(),
            &bundle.summary.anomaly_count.to_string(),
        ]));
        model_pair_deltas.push(csv_row(&[
            &bundle.selected.paired_instance_key,
            &bundle.selected.cohort_id,
            &bundle.selected.model,
            bundle.selected.personality_mode.as_deref().unwrap_or(""),
            &bundle.summary.total_tokens.unwrap_or_default().to_string(),
            &bundle.summary.visible_output_total_tokens_est.to_string(),
            &bundle.summary.tool_count.to_string(),
            &bundle.summary.command_count.to_string(),
        ]));
        for claim in &bundle.claim_evidence {
            claim_rows.push(csv_row(&[
                &bundle.selected.instance_id,
                &bundle.selected.cohort_id,
                &claim.claim_id,
                &claim.label,
            ]));
        }
        let entry = task_rollup
            .entry(bundle.summary.task_class.clone())
            .or_insert((0, 0, 0, 0, 0));
        entry.0 += 1;
        entry.1 += bundle.summary.total_tokens.unwrap_or_default();
        entry.2 += bundle.summary.visible_output_total_tokens_est;
        entry.3 += bundle.summary.tool_count;
        entry.4 += bundle.summary.command_count;
    }
    for (task_class, (count, total_tokens, visible_tokens, tool_count, command_count)) in task_rollup {
        task_class_summary.push(csv_row(&[
            &task_class,
            &count.to_string(),
            &total_tokens.to_string(),
            &visible_tokens.to_string(),
            &tool_count.to_string(),
            &command_count.to_string(),
        ]));
    }

    fs::write(datasets_dir.join("campaign_runs.csv"), campaign_runs.join("\n"))?;
    fs::write(datasets_dir.join("claim_evidence.csv"), claim_rows.join("\n"))?;
    fs::write(datasets_dir.join("model_pair_deltas.csv"), model_pair_deltas.join("\n"))?;
    fs::write(datasets_dir.join("task_class_summary.csv"), task_class_summary.join("\n"))?;

    for ((cohort_id, kind, name), stats) in aggregate_tool_inventory(bundles) {
        tool_inventory.push(csv_row(&[
            &cohort_id,
            &kind,
            &name,
            &stats.call_count.to_string(),
            &stats.success_count.to_string(),
            &stats.failure_count.to_string(),
            &stats.total_output_size.to_string(),
            &median_i64(&stats.durations_ms).to_string(),
        ]));
    }
    for ((cohort_id, route), count) in aggregate_tool_routes(bundles) {
        tool_route_summary.push(csv_row(&[&cohort_id, &route, &count.to_string()]));
    }
    for profile in aggregate_linguistic_profiles(bundles) {
        let top_word_mass = profile.top_words.iter().map(|(_, count)| *count).sum::<usize>();
        let top_bigram_mass = profile.top_bigrams.iter().map(|(_, count)| *count).sum::<usize>();
        let top_trigram_mass = profile.top_trigrams.iter().map(|(_, count)| *count).sum::<usize>();
        cohort_lexical_summary.push(csv_row(&[
            &profile.cohort_id,
            &top_word_mass.to_string(),
            &top_bigram_mass.to_string(),
            &top_trigram_mass.to_string(),
            &profile.social_tone_messages.to_string(),
            &profile.verification_messages.to_string(),
            &profile.tool_bridge_messages.to_string(),
            &profile.planning_messages.to_string(),
        ]));
        for (term, count) in profile.top_words {
            message_lexical_summary.push(csv_row(&[&profile.cohort_id, "word", &term, &count.to_string()]));
        }
        for (term, count) in profile.top_bigrams {
            message_lexical_summary.push(csv_row(&[&profile.cohort_id, "bigram", &term, &count.to_string()]));
        }
        for (term, count) in profile.top_trigrams {
            message_lexical_summary.push(csv_row(&[&profile.cohort_id, "trigram", &term, &count.to_string()]));
        }
        for (category, count) in profile.discourse_counts {
            message_discourse_summary.push(csv_row(&[&profile.cohort_id, &category, &count.to_string()]));
        }
    }
    for bundle in bundles {
        let message_path = bundle.selected.run_dir.join("attempt-01").join("message-metrics.jsonl");
        for value in read_jsonl_values(&message_path).unwrap_or_default() {
            message_style.push(csv_row(&[
                &bundle.selected.instance_id,
                &bundle.selected.cohort_id,
                value.get("messageId").and_then(Value::as_str).unwrap_or(""),
                value.get("turnId").and_then(Value::as_str).unwrap_or(""),
                &value.get("seq").and_then(Value::as_u64).unwrap_or_default().to_string(),
                &value.get("hedgingScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("confidenceScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("collaborationToneScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("directiveScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("reflectiveScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("formalityScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("empathyAlignmentScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("contentWordCount").and_then(Value::as_u64).unwrap_or_default().to_string(),
                &value.get("functionWordCount").and_then(Value::as_u64).unwrap_or_default().to_string(),
                &value.get("typeTokenRatioBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("lexicalDiversityScoreBps").and_then(Value::as_i64).unwrap_or_default().to_string(),
            ]));
        }
        let path = bundle.selected.run_dir.join("attempt-01").join("tool-events.jsonl");
        for value in read_jsonl_values(&path).unwrap_or_default() {
            tool_by_turn.push(csv_row(&[
                &bundle.selected.instance_id,
                &bundle.selected.cohort_id,
                value.get("turnId").and_then(Value::as_str).unwrap_or(""),
                value.get("kind").and_then(Value::as_str).unwrap_or(""),
                value.get("name").and_then(Value::as_str).unwrap_or(""),
                value.get("toolRoute").and_then(Value::as_str).unwrap_or(""),
                value.get("phase").and_then(Value::as_str).unwrap_or(""),
                &value.get("seq").and_then(Value::as_u64).unwrap_or_default().to_string(),
                value.get("callId").and_then(Value::as_str).unwrap_or(""),
                &value.get("durationMs").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("success").and_then(Value::as_bool).unwrap_or(false).to_string(),
                &value.get("precededByCommentaryTokens").and_then(Value::as_i64).unwrap_or_default().to_string(),
                &value.get("outputSize").and_then(Value::as_u64).unwrap_or_default().to_string(),
            ]));
        }
    }

    for row in compute_phrase_deltas(bundles, "model") {
        model_phrase_deltas.push(csv_row(&[
            &row.pair_kind,
            &row.grouping_key,
            &row.left_cohort,
            &row.right_cohort,
            &row.term_type,
            &row.term,
            &row.left_count.to_string(),
            &row.right_count.to_string(),
            &row.delta.to_string(),
        ]));
    }
    for row in compute_phrase_deltas(bundles, "personality") {
        personality_phrase_deltas.push(csv_row(&[
            &row.pair_kind,
            &row.grouping_key,
            &row.left_cohort,
            &row.right_cohort,
            &row.term_type,
            &row.term,
            &row.left_count.to_string(),
            &row.right_count.to_string(),
            &row.delta.to_string(),
        ]));
    }

    fs::write(datasets_dir.join("message_lexical_summary.csv"), message_lexical_summary.join("\n"))?;
    fs::write(datasets_dir.join("cohort_lexical_summary.csv"), cohort_lexical_summary.join("\n"))?;
    fs::write(datasets_dir.join("message_discourse_summary.csv"), message_discourse_summary.join("\n"))?;
    fs::write(datasets_dir.join("message_style.csv"), message_style.join("\n"))?;
    fs::write(datasets_dir.join("model_phrase_deltas.csv"), model_phrase_deltas.join("\n"))?;
    fs::write(datasets_dir.join("personality_phrase_deltas.csv"), personality_phrase_deltas.join("\n"))?;
    fs::write(datasets_dir.join("tool_inventory.csv"), tool_inventory.join("\n"))?;
    fs::write(datasets_dir.join("tool_route_summary.csv"), tool_route_summary.join("\n"))?;
    fs::write(datasets_dir.join("tool_by_turn.csv"), tool_by_turn.join("\n"))?;

    write_pass_through_dataset(
        &datasets_dir.join("turn_metrics.csv"),
        "turn-metrics.jsonl",
        bundles,
    )?;
    write_pass_through_dataset(
        &datasets_dir.join("message_metrics.csv"),
        "message-metrics.jsonl",
        bundles,
    )?;
    write_pass_through_dataset(
        &datasets_dir.join("tool_usage.csv"),
        "tool-events.jsonl",
        bundles,
    )?;
    write_pass_through_dataset(
        &datasets_dir.join("verbosity_tool_coupling.csv"),
        "verbosity-tool-coupling.jsonl",
        bundles,
    )?;
    Ok(())
}

fn write_pass_through_dataset(
    output_path: &Path,
    file_name: &str,
    bundles: &[RunReportBundle],
) -> Result<()> {
    let mut rows = vec!["instance_id,cohort_id,json".to_string()];
    for bundle in bundles {
        let path = bundle.selected.run_dir.join("attempt-01").join(file_name);
        for value in read_jsonl_values(&path).unwrap_or_default() {
            rows.push(csv_row(&[
                &bundle.selected.instance_id,
                &bundle.selected.cohort_id,
                &serde_json::to_string(&value)?,
            ]));
        }
    }
    fs::write(output_path, rows.join("\n"))?;
    Ok(())
}

fn csv_row(cells: &[&str]) -> String {
    cells
        .iter()
        .map(|cell| {
            let escaped = cell.replace('"', "\"\"");
            format!("\"{}\"", escaped)
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn render_attempt_log(
    attempt_dir: &Path,
    record: &DatasetRecord,
    summary: &RunSummary,
    turn_metrics: &[Value],
    command_events: &[Value],
    tool_events: &[Value],
    skill_events: &[Value],
    lifecycle_events: &[Value],
    anomaly_events: &[Value],
) -> Result<PathBuf> {
    let mut chronology = Vec::<(usize, String)>::new();

    for value in lifecycle_events {
        chronology.push((value.get("seq").and_then(Value::as_u64).unwrap_or_default() as usize, format_lifecycle_event(value)));
    }
    for value in turn_metrics {
        chronology.push((value.get("startSeq").and_then(Value::as_u64).unwrap_or_default() as usize, format!("TURN {}", format_turn_metric(value))));
    }
    for value in skill_events {
        chronology.push((value.get("seq").and_then(Value::as_u64).unwrap_or_default() as usize, format!("SKILL {}", format_skill_event(value))));
    }
    for value in tool_events {
        chronology.push((value.get("seq").and_then(Value::as_u64).unwrap_or_default() as usize, format!("TOOL {}", format_tool_event(value))));
    }
    for value in command_events {
        chronology.push((value.get("seq").and_then(Value::as_u64).unwrap_or_default() as usize, format!("CMD {}", format_command_event(value))));
    }
    for value in anomaly_events {
        chronology.push((value.get("seq").and_then(Value::as_u64).unwrap_or_default() as usize, format!("ANOMALY {}", format_anomaly_event(value))));
    }
    chronology.sort_by_key(|(seq, _)| *seq);

    let mut lines = Vec::new();
    lines.push("Attempt Log".to_string());
    lines.push("===========".to_string());
    lines.push(format!("Instance: {}", record.instance_id));
    lines.push(format!("Repo: {}", record.repo));
    lines.push(format!("Status: {}", summary.status));
    lines.push(format!("Task class: {}", summary.task_class));
    lines.push(format!(
        "Token totals: input={} output={} cache_read={} total={}",
        summary.total_input_tokens.unwrap_or_default(),
        summary.total_output_tokens.unwrap_or_default(),
        summary.total_cache_read_tokens.unwrap_or_default(),
        summary.total_tokens.unwrap_or_default()
    ));
    lines.push(String::new());
    lines.push("Chronology".to_string());
    lines.push("---------".to_string());
    if chronology.is_empty() {
        lines.push("<no events>".to_string());
    } else {
        for (seq, row) in chronology {
            lines.push(format!("#{seq:05} {row}"));
        }
    }

    let path = attempt_dir.join("attempt-log.txt");
    fs::write(&path, lines.join("\n"))?;
    Ok(path)
}

fn severity_rank(status: &str) -> u8 {
    match status {
        "aborted" => 0,
        "incomplete" => 1,
        "completed" => 2,
        _ => 3,
    }
}

fn render_count_map(map: &BTreeMap<String, usize>) -> String {
    if map.is_empty() {
        return "-".to_string();
    }
    map.iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_count_map_filtered(map: &BTreeMap<String, usize>, prefix: &str) -> String {
    let filtered = map
        .iter()
        .filter(|(key, _)| key.starts_with(prefix))
        .map(|(key, value)| (key.clone(), *value))
        .collect::<BTreeMap<_, _>>();
    render_count_map(&filtered)
}

fn load_official_grading_overview(campaign_dir: &Path) -> OfficialGradingOverview {
    let grader_path = campaign_dir.join("reports").join("grader.json");
    let mut overview = OfficialGradingOverview::default();
    overview.status = read_json::<Value>(&grader_path)
        .ok()
        .and_then(|value| value.get("status").and_then(Value::as_str).map(ToOwned::to_owned))
        .unwrap_or_else(|| "missing".to_string());

    let official_dir = campaign_dir.join("reports").join("official");
    let Ok(entries) = fs::read_dir(&official_dir) else {
        return overview;
    };
    let mut report_files = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    report_files.sort();
    if let Some(path) = report_files.pop() {
        if let Ok(value) = read_json::<Value>(&path) {
            overview.resolved_instances = read_usize_field(&value, "resolved_instances");
            overview.unresolved_instances = read_usize_field(&value, "unresolved_instances");
            overview.error_instances = read_usize_field(&value, "error_instances");
            overview.completed_instances = read_usize_field(&value, "completed_instances");
            overview.summary_path = Some(path);
        }
    }
    overview
}

fn read_usize_field(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or_default()
}

fn format_turn_metric(value: &Value) -> String {
    format!(
        "turn={} status={} total_delta={} input_delta={} output_delta={} cache_delta={} cmds={} mcp={} patch={} skills={} first={} last={}",
        value.get("turnId").and_then(Value::as_str).unwrap_or("-"),
        value.get("status").and_then(Value::as_str).unwrap_or("-"),
        value.get("totalTokensDelta").and_then(Value::as_i64).unwrap_or_default(),
        value.get("inputTokensDelta").and_then(Value::as_i64).unwrap_or_default(),
        value.get("outputTokensDelta").and_then(Value::as_i64).unwrap_or_default(),
        value.get("cacheReadTokensDelta").and_then(Value::as_i64).unwrap_or_default(),
        value.get("commandCount").and_then(Value::as_u64).unwrap_or_default(),
        value.get("mcpToolCount").and_then(Value::as_u64).unwrap_or_default(),
        value.get("patchApplyCount").and_then(Value::as_u64).unwrap_or_default(),
        value.get("skillEventCount").and_then(Value::as_u64).unwrap_or_default(),
        value.get("firstCommand").and_then(Value::as_str).unwrap_or("-"),
        value.get("lastCommand").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_token_snapshot(value: &Value) -> String {
    format!(
        "seq={} total={} input={} output={} cache_read={} last_total={} context_window={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("totalTokens").and_then(Value::as_i64).unwrap_or_default(),
        value.get("inputTokens").and_then(Value::as_i64).unwrap_or_default(),
        value.get("outputTokens").and_then(Value::as_i64).unwrap_or_default(),
        value.get("cachedInputTokens").and_then(Value::as_i64).unwrap_or_default(),
        value.get("lastTotalTokens").and_then(Value::as_i64).unwrap_or_default(),
        value.get("modelContextWindow").and_then(Value::as_i64).unwrap_or_default(),
    )
}

fn format_command_event(value: &Value) -> String {
    format!(
        "seq={} phase={} turn={} command={} exit={} duration_ms={} cwd={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("phase").and_then(Value::as_str).unwrap_or("-"),
        value.get("turnId").and_then(Value::as_str).unwrap_or("-"),
        value.get("command").and_then(Value::as_str).unwrap_or("-"),
        value.get("exitCode").and_then(Value::as_i64).map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
        value.get("durationMs").and_then(Value::as_i64).unwrap_or_default(),
        value.get("cwd").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_tool_event(value: &Value) -> String {
    format!(
        "seq={} phase={} kind={} name={} route={} turn={} call={} server={} tool={} success={} duration_ms={} preceded_tokens={} output_size={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("phase").and_then(Value::as_str).unwrap_or("-"),
        value.get("kind").and_then(Value::as_str).unwrap_or("-"),
        value.get("name").and_then(Value::as_str).unwrap_or("-"),
        value.get("toolRoute").and_then(Value::as_str).unwrap_or("-"),
        value.get("turnId").and_then(Value::as_str).unwrap_or("-"),
        value.get("callId").and_then(Value::as_str).unwrap_or("-"),
        value.get("server").and_then(Value::as_str).unwrap_or("-"),
        value.get("tool").and_then(Value::as_str).unwrap_or("-"),
        value.get("success").and_then(Value::as_bool).map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
        value.get("durationMs").and_then(Value::as_i64).unwrap_or_default(),
        value.get("precededByCommentaryTokens").and_then(Value::as_i64).unwrap_or_default(),
        value.get("outputSize").and_then(Value::as_u64).unwrap_or_default(),
    )
}

fn format_skill_event(value: &Value) -> String {
    format!(
        "seq={} kind={} skill={} scope={} enabled={} command={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("kind").and_then(Value::as_str).unwrap_or("-"),
        value.get("skillName").and_then(Value::as_str).unwrap_or("-"),
        value.get("scope").and_then(Value::as_str).unwrap_or("-"),
        value.get("enabled").and_then(Value::as_bool).map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
        value.get("command").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_message_metric(value: &Value) -> String {
    format!(
        "seq={} phase={} chars={} tokens_est={} categories={} next_step={} tool_intent={} verification={} result={} social={} text={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("phase").and_then(Value::as_str).unwrap_or("-"),
        value.get("textChars").and_then(Value::as_u64).unwrap_or_default(),
        value.get("textTokensEst").and_then(Value::as_i64).unwrap_or_default(),
        value
            .get("categories")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("|"))
            .unwrap_or_else(|| "-".to_string()),
        value.get("containsNextStep").and_then(Value::as_bool).unwrap_or(false),
        value.get("containsToolIntent").and_then(Value::as_bool).unwrap_or(false),
        value
            .get("containsVerificationLanguage")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        value.get("containsResultClaim").and_then(Value::as_bool).unwrap_or(false),
        value
            .get("containsEmpathyOrAlignmentLanguage")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        value.get("message").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_coupling_row(value: &Value) -> String {
    format!(
        "seq={} kind={} name={} visible_chars_since_last_tool={} visible_tokens_since_last_tool={} visible_messages_since_last_tool={} burst_label={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("kind").and_then(Value::as_str).unwrap_or("-"),
        value.get("name").and_then(Value::as_str).unwrap_or("-"),
        value
            .get("visibleCharsSinceLastTool")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        value
            .get("visibleTokensSinceLastTool")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        value
            .get("visibleMessagesSinceLastTool")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        value.get("burstLabel").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_lifecycle_event(value: &Value) -> String {
    format!(
        "seq={} kind={} turn={} model={} provider={} reason={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("kind").and_then(Value::as_str).unwrap_or("-"),
        value.get("turnId").and_then(Value::as_str).unwrap_or("-"),
        value.get("model").and_then(Value::as_str).unwrap_or("-"),
        value.get("provider").and_then(Value::as_str).unwrap_or("-"),
        value.get("reason").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_probe_event(value: &Value) -> String {
    format!(
        "seq={} subsystem={} code={} class={} summary={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("subsystem").and_then(Value::as_str).unwrap_or("-"),
        value.get("evidence_code").and_then(Value::as_str).unwrap_or("-"),
        value.get("classification").and_then(Value::as_str).unwrap_or("-"),
        value.get("summary").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_anomaly_event(value: &Value) -> String {
    format!(
        "seq={} severity={} code={} message={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("severity").and_then(Value::as_str).unwrap_or("-"),
        value.get("code").and_then(Value::as_str).unwrap_or("-"),
        value.get("message").and_then(Value::as_str).unwrap_or("-"),
    )
}

fn format_grade_event(value: &Value) -> String {
    format!(
        "instance={} grading_status={} official_summary={} official_instance_report={}",
        value.get("instanceId").and_then(Value::as_str).unwrap_or("-"),
        value.get("gradingStatus").and_then(Value::as_str).unwrap_or("-"),
        value
            .get("officialSummaryPath")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        value
            .get("officialInstanceReportPath")
            .and_then(Value::as_str)
            .unwrap_or("-"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_bench_core::{EvidenceClassification, SelectedInstance};
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time works")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("codex-bench-report-{label}-{nanos}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn write_jsonl(path: &Path, rows: &[Value]) {
        let contents = rows
            .iter()
            .map(|row| serde_json::to_string(row).expect("json row"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, contents).expect("write jsonl");
    }

    fn sample_bundle(
        root: &Path,
        cohort_id: &str,
        model: &str,
        personality_mode: &str,
        message: &str,
        categories: &[&str],
        tool_route: &str,
        tool_name: &str,
    ) -> RunReportBundle {
        let run_dir = root.join(cohort_id).join("demo__repo-1");
        let attempt_dir = run_dir.join("attempt-01");
        fs::create_dir_all(&attempt_dir).expect("create attempt dir");

        write_jsonl(
            &attempt_dir.join("message-metrics.jsonl"),
            &[json!({
                "messageId": format!("{cohort_id}-m1"),
                "turnId": "turn-1",
                "seq": 1,
                "message": message,
                "categories": categories,
                "hedgingScoreBps": 1200,
                "confidenceScoreBps": 3400,
                "collaborationToneScoreBps": 1800,
                "directiveScoreBps": 900,
                "reflectiveScoreBps": 1500,
                "formalityScoreBps": 1000,
                "empathyAlignmentScoreBps": 800,
                "contentWordCount": 8,
                "functionWordCount": 4,
                "typeTokenRatioBps": 7500,
                "lexicalDiversityScoreBps": 7500
            })],
        );
        write_jsonl(
            &attempt_dir.join("tool-events.jsonl"),
            &[json!({
                "phase": "begin",
                "kind": "shell",
                "name": tool_name,
                "toolRoute": tool_route,
                "turnId": "turn-1",
                "seq": 1,
                "callId": format!("{cohort_id}-call-1"),
                "durationMs": 42,
                "success": true,
                "precededByCommentaryTokens": 24,
                "outputSize": 16
            })],
        );

        RunReportBundle {
            selected: SelectedInstance {
                instance_id: "demo__repo-1".to_string(),
                repo: "demo/repo".to_string(),
                task_class: "verification-heavy".to_string(),
                run_dir: run_dir.clone(),
                paired_instance_key: "demo__repo-1".to_string(),
                cohort_id: cohort_id.to_string(),
                model: model.to_string(),
                provider: "openai".to_string(),
                personality_mode: Some(personality_mode.to_string()),
                prompt_style: Some("research_exploratory".to_string()),
            },
            record: DatasetRecord {
                instance_id: "demo__repo-1".to_string(),
                repo: "demo/repo".to_string(),
                base_commit: "abc123".to_string(),
                patch: None,
                test_patch: None,
                problem_statement: "Investigate the failing verification path".to_string(),
                hints_text: None,
                version: None,
                environment_setup_commit: None,
                fail_to_pass: Vec::new(),
                pass_to_pass: Vec::new(),
                raw: Value::Null,
            },
            summary: RunSummary {
                observability_contract_version: Some(REPORT_OBSERVABILITY_CONTRACT_VERSION.to_string()),
                instance_id: "demo__repo-1".to_string(),
                repo: "demo/repo".to_string(),
                task_class: "verification-heavy".to_string(),
                paired_instance_key: Some("demo__repo-1".to_string()),
                cohort_id: Some(cohort_id.to_string()),
                model: Some(model.to_string()),
                provider: Some("openai".to_string()),
                personality_mode: Some(personality_mode.to_string()),
                prompt_style: Some("research_exploratory".to_string()),
                status: "completed".to_string(),
                grading_status: "grader_not_run".to_string(),
                tool_count: 1,
                command_count: 1,
                total_tokens: Some(1000),
                visible_output_total_tokens_est: 200,
                field_classifications: BTreeMap::from([
                    (
                        "visible_output_total_tokens_est".to_string(),
                        format!("{:?}", EvidenceClassification::Observed).to_lowercase(),
                    ),
                ]),
                ..RunSummary::default()
            },
            probe_summary: ProbeSummary::default(),
            claim_evidence: vec![ClaimEvidence {
                claim_id: "H1".to_string(),
                label: "evidence_consistent".to_string(),
                supporting_evidence: vec!["more visible output".to_string()],
                conflicting_evidence: Vec::new(),
                relevant_runs: vec!["demo__repo-1".to_string()],
                caveats: Vec::new(),
            }],
            artifact_paths: BTreeMap::new(),
        }
    }

    #[test]
    fn aggregate_linguistic_profiles_collects_words_and_discourse() {
        let root = temp_dir("linguistic-profile");
        let bundles = vec![
            sample_bundle(
                &root,
                "gpt-5.4-friendly",
                "gpt-5.4",
                "friendly",
                "Let's verify the fix and explain the result clearly.",
                &["planning", "verification_framing", "social_tone"],
                "exec_command",
                "shell",
            ),
            sample_bundle(
                &root,
                "gpt-5.3-codex-pragmatic",
                "gpt-5.3-codex",
                "pragmatic",
                "Verify the fix and report the result.",
                &["planning", "verification_framing"],
                "apply_patch",
                "apply_patch",
            ),
        ];

        let profiles = aggregate_linguistic_profiles(&bundles);
        assert_eq!(profiles.len(), 2);
        let friendly = profiles
            .iter()
            .find(|profile| profile.cohort_id == "gpt-5.4-friendly")
            .expect("friendly profile");
        assert!(friendly.top_words.iter().any(|(term, _)| term == "verify"));
        assert_eq!(
            friendly.discourse_counts.get("social_tone").copied().unwrap_or_default(),
            1
        );
    }

    #[test]
    fn compute_phrase_deltas_compares_model_and_personality_pairs() {
        let root = temp_dir("phrase-deltas");
        let bundles = vec![
            sample_bundle(
                &root,
                "gpt-5.4-friendly",
                "gpt-5.4",
                "friendly",
                "Let's verify the fix and explain the result clearly.",
                &["planning", "verification_framing", "social_tone"],
                "exec_command",
                "shell",
            ),
            sample_bundle(
                &root,
                "gpt-5.3-codex-friendly",
                "gpt-5.3-codex",
                "friendly",
                "Let's verify the fix.",
                &["planning", "verification_framing", "social_tone"],
                "exec_command",
                "shell",
            ),
            sample_bundle(
                &root,
                "gpt-5.4-pragmatic",
                "gpt-5.4",
                "pragmatic",
                "Verify the fix.",
                &["planning", "verification_framing"],
                "exec_command",
                "shell",
            ),
        ];

        let model_rows = compute_phrase_deltas(&bundles, "model");
        assert!(model_rows.iter().any(|row| {
            row.grouping_key == "demo__repo-1::friendly"
                && row.left_cohort != row.right_cohort
                && row.term == "explain"
        }));

        let personality_rows = compute_phrase_deltas(&bundles, "personality");
        assert!(personality_rows.iter().any(|row| {
            row.grouping_key == "demo__repo-1::gpt-5.4"
                && row.left_cohort != row.right_cohort
        }));
    }

    #[test]
    fn render_task_class_analysis_includes_benchmark_hints() {
        let root = temp_dir("task-class-analysis");
        let bundles = vec![sample_bundle(
            &root,
            "gpt-5.4-friendly",
            "gpt-5.4",
            "friendly",
            "Let's verify the fix and explain the result clearly.",
            &["planning", "verification_framing", "social_tone", "tool_bridge_before"],
            "exec_command",
            "shell",
        )];

        let profile = BenchmarkResearchProfile {
            benchmark_name: "SWE-bench Verified".to_string(),
            benchmark_adapter: "swebench".to_string(),
            summary: "Repo patching benchmark".to_string(),
            benchmark_notes: vec!["local only".to_string()],
            task_class_profiles: vec![codex_bench_core::BenchmarkTaskClassProfile {
                task_class: "verification-heavy".to_string(),
                expected_verification_strength: "strong".to_string(),
                expected_context_pressure: "medium".to_string(),
                expected_tool_mix: vec!["shell".to_string(), "apply_patch".to_string()],
                expected_bootstrap_risk: "medium".to_string(),
                expected_language_need: "high".to_string(),
                language_profile_hint: Some("needs explicit verification narration".to_string()),
                tool_profile_hint: Some("shell-heavy".to_string()),
                interaction_style_hint: Some("explain before verify".to_string()),
                default_analysis_overrides: BTreeMap::new(),
            }],
        };

        let rendered = render_task_class_analysis(&bundles, Some(&profile));
        assert!(rendered.contains("verification-heavy"));
        assert!(rendered.contains("needs explicit verification narration"));
        assert!(rendered.contains("shell-heavy"));
        assert!(rendered.contains("explain before verify"));
    }

    #[test]
    fn write_datasets_outputs_extended_research_csvs() {
        let campaign_dir = temp_dir("write-datasets");
        let bundles = vec![
            sample_bundle(
                &campaign_dir,
                "gpt-5.4-friendly",
                "gpt-5.4",
                "friendly",
                "Let's verify the fix and explain the result clearly.",
                &["planning", "verification_framing", "social_tone", "tool_bridge_before"],
                "exec_command",
                "shell",
            ),
            sample_bundle(
                &campaign_dir,
                "gpt-5.3-codex-friendly",
                "gpt-5.3-codex",
                "friendly",
                "Let's verify the fix.",
                &["planning", "verification_framing", "social_tone"],
                "apply_patch",
                "apply_patch",
            ),
        ];
        let manifest = CampaignManifest {
            schema_version: "v1".to_string(),
            campaign_id: "campaign-1".to_string(),
            campaign_status: "report_generated".to_string(),
            experiment_id: "experiment-1".to_string(),
            experiment_name: "model-personality".to_string(),
            created_at: "2026-03-12T00:00:00Z".to_string(),
            campaign_root: campaign_dir.clone(),
            repo_cache_root: campaign_dir.join("repo-cache"),
            benchmark_name: "SWE-bench Verified".to_string(),
            benchmark_adapter: "swebench".to_string(),
            preset_name: "swebench-v1".to_string(),
            preset_path: campaign_dir.join("preset.json"),
            stage_name: Some("architecture-validation".to_string()),
            probe_profile: "default".to_string(),
            report_profile: "default".to_string(),
            model: "gpt-5.4".to_string(),
            provider: "openai".to_string(),
            personality_mode: Some("friendly".to_string()),
            prompt_style: Some("research_exploratory".to_string()),
            comparison_axes: vec!["model".to_string(), "personality".to_string()],
            cohorts: Vec::new(),
            seed: "seed".to_string(),
            sample_size: 1,
            study_mode: "codex_behavior".to_string(),
            required_task_classes: Vec::new(),
            preferred_task_classes: Vec::new(),
            future_benchmarks: Vec::new(),
            grounding_documents: Vec::new(),
            reference_documents: Vec::new(),
            model_catalog_snapshot_path: None,
            hypothesis_catalog_path: None,
            experiment_lock_path: None,
            benchmark_research_profile_path: None,
            last_report_path: None,
            last_report_generated_at: None,
            selected_instances: bundles.iter().map(|bundle| bundle.selected.clone()).collect(),
        };

        write_datasets(&campaign_dir, &manifest, &bundles).expect("write datasets");

        let datasets_dir = campaign_dir.join("datasets");
        for required in [
            "campaign_runs.csv",
            "message_lexical_summary.csv",
            "message_discourse_summary.csv",
            "message_style.csv",
            "tool_inventory.csv",
            "tool_route_summary.csv",
            "tool_by_turn.csv",
            "model_phrase_deltas.csv",
            "personality_phrase_deltas.csv",
            "cohort_lexical_summary.csv",
        ] {
            assert!(
                datasets_dir.join(required).exists(),
                "expected dataset {required} to exist"
            );
        }
    }
}
