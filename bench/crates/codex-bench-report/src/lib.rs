use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use codex_app_server_protocol::JSONRPCNotification;
use codex_bench_codex::decode_legacy_notification;
use codex_bench_core::{
    CampaignManifest, ClaimCatalogEntry, ClaimEvidence, DatasetRecord, ProbeSummary, RunSummary,
    SelectedInstance, StudyArchitectureSubsystem, artifact_inventory_for_attempt,
    artifact_map_for_attempt, read_json, read_jsonl_values, write_json_pretty,
};
use codex_bench_probes::derive_run_outputs;
use codex_protocol::protocol::{Event, StudyProbeEvent};
use serde_json::Value;

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
        &grounding_claims,
        &codex_claims,
        &load_official_grading_overview(campaign_dir),
        &bundles,
    );
    let report_path = campaign_dir.join("reports").join("report.txt");
    fs::create_dir_all(report_path.parent().expect("report path has parent"))?;
    fs::write(&report_path, report)?;
    write_supporting_reports(campaign_dir, &manifest, &bundles)?;
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

    if !turn_metrics_missing && !skill_events_missing && !message_metrics_missing && !coupling_missing {
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
    grounding_claims: &[ClaimCatalogEntry],
    codex_claims: &[ClaimCatalogEntry],
    grading_overview: &OfficialGradingOverview,
    bundles: &[RunReportBundle],
) -> String {
    let mut lines = Vec::new();
    lines.push("Study Header".to_string());
    lines.push("============".to_string());
    lines.push(format!("Campaign: {}", manifest.campaign_id));
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
        "# personality 分析\n\n该文件聚合 cohort 的 personality、可见输出、桥接语言和工具使用耦合证据。\n",
    )?;
    fs::write(
        reports_dir.join("task-class-analysis.md"),
        "# 任务类型分析\n\n请结合 `report.txt` 和 `datasets/task_class_summary.csv` 查看不同 task class 的差异。\n",
    )?;
    fs::write(
        reports_dir.join("methods-appendix.md"),
        "# 方法附录\n\n本研究以用户可见输出为“说更多”的主观测面，并结合 Codex 原始 probe、tool/patch/verification 时序构造耦合证据。\n",
    )?;
    Ok(())
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
        "seq={} phase={} kind={} name={} server={} tool={} success={} duration_ms={}",
        value.get("seq").and_then(Value::as_u64).unwrap_or_default(),
        value.get("phase").and_then(Value::as_str).unwrap_or("-"),
        value.get("kind").and_then(Value::as_str).unwrap_or("-"),
        value.get("name").and_then(Value::as_str).unwrap_or("-"),
        value.get("server").and_then(Value::as_str).unwrap_or("-"),
        value.get("tool").and_then(Value::as_str).unwrap_or("-"),
        value.get("success").and_then(Value::as_bool).map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
        value.get("durationMs").and_then(Value::as_i64).unwrap_or_default(),
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
