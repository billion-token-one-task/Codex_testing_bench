use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use codex_bench_core::{
    CampaignManifest, ClaimCatalogEntry, ClaimEvidence, DatasetRecord, ProbeSummary, RunSummary,
    SelectedInstance, StudyArchitectureSubsystem, artifact_map_for_attempt, read_json,
};

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
        let summary: RunSummary = read_json(&summary_path)?;
        let probe_summary: ProbeSummary = read_json(&attempt_dir.join("probe-summary.json"))?;
        let claim_evidence: Vec<ClaimEvidence> =
            read_json(&attempt_dir.join("claim-evidence.json")).unwrap_or_default();
        let record: DatasetRecord = read_json(&selected.run_dir.join("record.json"))?;
        let artifact_paths = artifact_map_for_attempt(&attempt_dir);
        bundles.push(RunReportBundle {
            selected: selected.clone(),
            record,
            summary,
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
        &bundles,
    );
    let report_path = campaign_dir.join("reports").join("report.txt");
    fs::create_dir_all(report_path.parent().expect("report path has parent"))?;
    fs::write(&report_path, report)?;
    Ok(report_path)
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
    let mut lines = Vec::new();
    lines.push("Run Summary".to_string());
    lines.push("===========".to_string());
    lines.push(format!("Instance: {}", record.instance_id));
    lines.push(format!("Repo: {}", record.repo));
    lines.push(format!("Task class: {}", summary.task_class));
    lines.push(format!("Status: {}", summary.status));
    lines.push(format!("Grading status: {}", summary.grading_status));
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
    lines.extend(read_jsonl_lines(&attempt_dir.join("lifecycle-events.jsonl"))?);
    lines.push(String::new());
    lines.push("Compaction / Reconstruction Timeline".to_string());
    lines.push("===================================".to_string());
    lines.push(format!("compaction_count={}", probe_summary.compaction_count));
    lines.push(format!("compaction_rediscovery_count={}", probe_summary.compaction_rediscovery_count));
    lines.push(format!("peak_context_utilization_bps={:?}", probe_summary.peak_context_utilization_bps));
    lines.push(String::new());
    lines.push("Tool Orchestration Summary".to_string());
    lines.push("==========================".to_string());
    lines.extend(read_jsonl_lines(&attempt_dir.join("tool-events.jsonl"))?);
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
    Ok(path)
}

fn render_campaign_report_text(
    campaign_dir: &Path,
    manifest: &CampaignManifest,
    architecture_map: &[StudyArchitectureSubsystem],
    grounding_claims: &[ClaimCatalogEntry],
    codex_claims: &[ClaimCatalogEntry],
    bundles: &[RunReportBundle],
) -> String {
    let mut lines = Vec::new();
    lines.push("Study Header".to_string());
    lines.push("============".to_string());
    lines.push(format!("Campaign: {}", manifest.campaign_id));
    lines.push(format!("Created: {}", manifest.created_at));
    lines.push(format!("Model: {} via {}", manifest.model, manifest.provider));
    lines.push(format!("Study mode: {}", manifest.study_mode));
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
    let mut total_anomalies = 0usize;
    let mut artifact_missing = BTreeMap::<String, usize>::new();
    let mut aggregate_probe_codes = BTreeMap::<String, usize>::new();
    let mut aggregate_subsystems = BTreeMap::<String, usize>::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut task_classes = BTreeMap::<String, usize>::new();

    for bundle in bundles {
        total_input += bundle.summary.total_input_tokens.unwrap_or_default();
        total_output += bundle.summary.total_output_tokens.unwrap_or_default();
        total_cache += bundle.summary.total_cache_read_tokens.unwrap_or_default();
        total_commands += bundle.summary.command_count;
        total_tools += bundle.summary.tool_count;
        total_anomalies += bundle.summary.anomaly_count;
        *statuses.entry(bundle.summary.status.clone()).or_default() += 1;
        *task_classes.entry(bundle.summary.task_class.clone()).or_default() += 1;
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
    }

    lines.push("Telemetry And Artifact Coverage".to_string());
    lines.push("===============================".to_string());
    lines.push(format!("Run status counts: {}", render_count_map(&statuses)));
    lines.push(format!("Task class counts: {}", render_count_map(&task_classes)));
    lines.push(format!("Token totals: input={} output={} cache_read={}", total_input, total_output, total_cache));
    lines.push(format!("Command totals: {} | Tool totals: {} | Anomalies: {}", total_commands, total_tools, total_anomalies));
    if artifact_missing.is_empty() {
        lines.push("Artifact coverage: all expected artifacts present in the latest attempts.".to_string());
    } else {
        lines.push(format!("Artifact coverage gaps: {}", render_count_map(&artifact_missing)));
    }
    lines.push(String::new());

    lines.push("Observed Codex Harness Behavior".to_string());
    lines.push("===============================".to_string());
    lines.push(format!("Config/session freezing evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "config.")));
    lines.push(format!("Instruction assembly evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "instruction.")));
    lines.push(format!("Context and compaction evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "context.")));
    lines.push(format!("Tool mediation evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "tools.")));
    lines.push(format!("Persistence/reconstruction evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "persistence.")));
    lines.push(format!("Reliability/contention evidence: {}", render_count_map_filtered(&aggregate_probe_codes, "harness.")));
    lines.push(String::new());

    lines.push("Task-Behavior Evidence Across Live Tasks".to_string());
    lines.push("=======================================".to_string());
    for bundle in bundles {
        lines.push(format!(
            "{} | status={} | class={} | tokens={} | patch={} | compactions={} | repeated_git={} | repeated_verify={} | config_drift={} | friction={}",
            bundle.selected.instance_id,
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
            "{} | {} | {} | tokens={} | probes={} | anomalies={} | evidence={}",
            bundle.selected.instance_id,
            bundle.summary.status,
            bundle.summary.task_class,
            bundle.summary.total_tokens.unwrap_or_default(),
            bundle.summary.raw_probe_count,
            bundle.summary.anomaly_count,
            bundle.selected.run_dir.join("attempt-01").join("run-evidence.txt").display()
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

fn read_jsonl_lines(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(vec!["<missing>".to_string()]);
    }
    let content = fs::read_to_string(path)?;
    Ok(content.lines().map(ToOwned::to_owned).collect())
}

