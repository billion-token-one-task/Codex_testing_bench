use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, anyhow};
use codex_protocol::protocol::{
    Event, EventMsg, ExecCommandBeginEvent, ExecCommandEndEvent, StudyProbeEvent, TokenUsageInfo,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::study::{read_json, write_json_pretty, write_jsonl};
use crate::types::{
    CampaignManifest, ClaimCatalogEntry, ClaimEvidence, DatasetRecord, ProbeEventRow, ProbeSummary,
    RunSummary, SelectedInstance, StudyArchitectureSubsystem,
};

const TOKEN_BUDGET_DOC: &str = "/Users/kevinlin/Downloads/Token预算实验_完整分析报告_v3.docx";
const SCHEDULER_DOC: &str =
    "/Users/kevinlin/Downloads/TokenMartCC/docs/papers/2026-03-09-单任务十亿级token调度架构初论.md";
const DEEPWIKI_DOC: &str = "https://deepwiki.com/openai/codex";
const OPENAI_HARNESS_DOC: &str = "https://openai.com/index/unlocking-the-codex-harness/";

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

pub async fn render_campaign_report(campaign_dir: &Path) -> Result<PathBuf> {
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

pub async fn render_single_run_replay(run_dir: &Path) -> Result<PathBuf> {
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

pub fn derive_run_outputs(
    attempt_dir: &Path,
    run_id: &str,
    task_class: &str,
    record: &DatasetRecord,
    decoded_events: &[Event],
    raw_probe_events: &[StudyProbeEvent],
    raw_diagnostics: &[Value],
    patch_text: &[u8],
) -> Result<RunSummary> {
    let mut token_rows = Vec::<Value>::new();
    let mut command_rows = Vec::<Value>::new();
    let mut tool_rows = Vec::<Value>::new();
    let mut patch_rows = Vec::<Value>::new();
    let mut lifecycle_rows = Vec::<Value>::new();
    let mut anomaly_rows = Vec::<Value>::new();
    let mut derived_probes = Vec::<ProbeEventRow>::new();

    let mut event_type_counts = BTreeMap::<String, usize>::new();
    let mut probe_code_counts = BTreeMap::<String, usize>::new();
    let mut probe_subsystem_counts = BTreeMap::<String, usize>::new();
    let mut diagnostic_type_counts = BTreeMap::<String, usize>::new();

    let mut probe_summary = ProbeSummary::default();
    let mut last_token_info = None::<TokenUsageInfo>;
    let mut last_verification_token_total = None::<i64>;
    let mut last_patch_seq = None::<usize>;
    let mut last_write_seq = None::<usize>;
    let mut command_seq = 0usize;

    let mut seen_read_commands_since_write = BTreeSet::<String>::new();
    let mut seen_verification_commands_since_write = BTreeSet::<String>::new();
    let mut seen_git_commands_since_write = BTreeSet::<String>::new();

    for (seq, event) in decoded_events.iter().enumerate() {
        let event_type = event_type_name(&event.msg).to_string();
        *event_type_counts.entry(event_type.clone()).or_default() += 1;

        match &event.msg {
            EventMsg::SessionConfigured(ev) => {
                lifecycle_rows.push(json!({
                    "seq": seq,
                    "kind": "session_configured",
                    "sessionId": ev.session_id.to_string(),
                    "model": ev.model,
                    "provider": ev.model_provider_id,
                    "cwd": ev.cwd,
                    "approvalPolicy": format!("{:?}", ev.approval_policy),
                    "sandboxPolicy": format!("{:?}", ev.sandbox_policy),
                    "hasInitialMessages": ev.initial_messages.as_ref().map(|messages| !messages.is_empty()).unwrap_or(false),
                    "historyEntryCount": ev.history_entry_count,
                }));
            }
            EventMsg::TurnStarted(ev) => {
                lifecycle_rows.push(json!({
                    "seq": seq,
                    "kind": "turn_started",
                    "turnId": ev.turn_id,
                    "modelContextWindow": ev.model_context_window,
                }));
            }
            EventMsg::TurnComplete(ev) => {
                lifecycle_rows.push(json!({
                    "seq": seq,
                    "kind": "turn_complete",
                    "turnId": ev.turn_id,
                    "lastAgentMessage": ev.last_agent_message,
                }));
            }
            EventMsg::TurnAborted(ev) => {
                lifecycle_rows.push(json!({
                    "seq": seq,
                    "kind": "turn_aborted",
                    "turnId": ev.turn_id,
                    "reason": format!("{:?}", ev.reason),
                }));
                anomaly_rows.push(json!({
                    "seq": seq,
                    "severity": "warning",
                    "code": "turn_aborted",
                    "message": format!("Turn aborted: {:?}", ev.reason),
                    "sourceRefs": ["raw-agent-events.jsonl"],
                }));
            }
            EventMsg::TokenCount(ev) => {
                if let Some(info) = &ev.info {
                    last_token_info = Some(info.clone());
                    token_rows.push(json!({
                        "seq": seq,
                        "classification": "exact",
                        "inputTokens": info.total_token_usage.input_tokens,
                        "cachedInputTokens": info.total_token_usage.cached_input_tokens,
                        "outputTokens": info.total_token_usage.output_tokens,
                        "reasoningOutputTokens": info.total_token_usage.reasoning_output_tokens,
                        "totalTokens": info.total_token_usage.total_tokens,
                        "lastInputTokens": info.last_token_usage.input_tokens,
                        "lastCachedInputTokens": info.last_token_usage.cached_input_tokens,
                        "lastOutputTokens": info.last_token_usage.output_tokens,
                        "lastReasoningOutputTokens": info.last_token_usage.reasoning_output_tokens,
                        "lastTotalTokens": info.last_token_usage.total_tokens,
                        "modelContextWindow": info.model_context_window,
                    }));
                }
            }
            EventMsg::ExecCommandBegin(ev) => {
                command_seq += 1;
                handle_command_begin(
                    seq,
                    run_id,
                    record,
                    task_class,
                    ev,
                    last_token_total(&last_token_info),
                    &mut command_rows,
                    &mut derived_probes,
                    &mut probe_summary,
                    &mut seen_read_commands_since_write,
                    &mut seen_verification_commands_since_write,
                    &mut seen_git_commands_since_write,
                    &mut last_write_seq,
                );
            }
            EventMsg::ExecCommandEnd(ev) => {
                handle_command_end(
                    seq,
                    ev,
                    &mut command_rows,
                    &mut derived_probes,
                    &mut probe_summary,
                    last_write_seq,
                    last_patch_seq,
                );
            }
            EventMsg::McpToolCallBegin(ev) => {
                tool_rows.push(json!({
                    "seq": seq,
                    "phase": "begin",
                    "classification": "exact",
                    "kind": "mcp",
                    "callId": ev.call_id,
                    "server": ev.invocation.server,
                    "tool": ev.invocation.tool,
                    "arguments": ev.invocation.arguments,
                }));
            }
            EventMsg::McpToolCallEnd(ev) => {
                tool_rows.push(json!({
                    "seq": seq,
                    "phase": "end",
                    "classification": "exact",
                    "kind": "mcp",
                    "callId": ev.call_id,
                    "server": ev.invocation.server,
                    "tool": ev.invocation.tool,
                    "durationMs": duration_to_ms_i64(ev.duration),
                    "success": ev.result.is_ok(),
                }));
                probe_summary.tool_mediation_tax_count += 1;
            }
            EventMsg::PatchApplyBegin(ev) => {
                if probe_summary.first_patch_tokens.is_none() {
                    probe_summary.first_patch_tokens = last_token_total(&last_token_info);
                }
                patch_rows.push(json!({
                    "seq": seq,
                    "phase": "begin",
                    "classification": "exact",
                    "callId": ev.call_id,
                    "autoApproved": ev.auto_approved,
                }));
            }
            EventMsg::PatchApplyEnd(ev) => {
                last_write_seq = Some(seq);
                last_patch_seq = Some(seq);
                patch_rows.push(json!({
                    "seq": seq,
                    "phase": "end",
                    "classification": "exact",
                    "callId": ev.call_id,
                    "status": format!("{:?}", ev.status),
                    "stdoutChars": ev.stdout.chars().count(),
                    "stderrChars": ev.stderr.chars().count(),
                }));
            }
            EventMsg::StudyProbe(probe) => {
                *probe_code_counts.entry(probe.code.clone()).or_default() += 1;
                *probe_subsystem_counts
                    .entry(format!("{:?}", probe.subsystem))
                    .or_default() += 1;
                handle_raw_probe(
                    seq,
                    run_id,
                    record,
                    task_class,
                    probe,
                    &mut derived_probes,
                    &mut probe_summary,
                    &mut anomaly_rows,
                );
            }
            EventMsg::Warning(ev) => {
                anomaly_rows.push(json!({
                    "seq": seq,
                    "severity": "warning",
                    "code": "warning_event",
                    "message": ev.message,
                    "sourceRefs": ["raw-agent-events.jsonl"],
                }));
            }
            EventMsg::Error(ev) => {
                anomaly_rows.push(json!({
                    "seq": seq,
                    "severity": "error",
                    "code": "error_event",
                    "message": ev.message,
                    "sourceRefs": ["raw-agent-events.jsonl"],
                }));
                probe_summary.containment_breach_count += 1;
            }
            EventMsg::StreamError(ev) => {
                anomaly_rows.push(json!({
                    "seq": seq,
                    "severity": "error",
                    "code": "stream_error",
                    "message": ev.message,
                    "sourceRefs": ["raw-agent-events.jsonl"],
                }));
                probe_summary.containment_breach_count += 1;
            }
            _ => {}
        }
    }

    for diagnostic in raw_diagnostics {
        let kind = diagnostic
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *diagnostic_type_counts.entry(kind).or_default() += 1;
    }

    if let Some(info) = &last_token_info {
        probe_summary.final_patch_tokens = Some(info.total_token_usage.total_tokens);
        probe_summary.cache_read_ratio_num = info.total_token_usage.cached_input_tokens;
        probe_summary.cache_read_ratio_den = info.total_token_usage.input_tokens.max(1);
        probe_summary.context_window = info.model_context_window;
        probe_summary.peak_context_utilization_bps = info.model_context_window.map(|window| {
            if window <= 0 {
                0
            } else {
                (info.total_token_usage.total_tokens * 10_000) / window
            }
        });
    }
    if probe_summary.useful_step_proxy_den == 0 {
        probe_summary.useful_step_proxy_den = command_seq.max(1);
    }
    if probe_summary.useful_token_proxy_den == 0 {
        probe_summary.useful_token_proxy_den = last_token_total(&last_token_info).unwrap_or_default();
    }
    if probe_summary.friction_token_proxy_den == 0 {
        probe_summary.friction_token_proxy_den = last_token_total(&last_token_info).unwrap_or_default();
    }
    if probe_summary.retained_edit_ratio_den == 0 {
        probe_summary.retained_edit_ratio_den = patch_rows
            .iter()
            .filter(|row| row.get("phase").and_then(Value::as_str) == Some("end"))
            .count();
    }
    if probe_summary.reverted_work_ratio_den == 0 {
        probe_summary.reverted_work_ratio_den = probe_summary.retained_edit_ratio_den;
    }

    if raw_probe_events.is_empty() {
        anomaly_rows.push(json!({
            "seq": decoded_events.len(),
            "severity": "warning",
            "code": "missing_raw_study_probes",
            "message": "No codex raw study probes were observed for this run.",
            "sourceRefs": ["codex-probe-events.jsonl"],
        }));
    }
    if !decoded_events
        .iter()
        .any(|event| matches!(event.msg, EventMsg::TurnComplete(_) | EventMsg::TurnAborted(_)))
    {
        anomaly_rows.push(json!({
            "seq": decoded_events.len(),
            "severity": "error",
            "code": "missing_terminal_turn_event",
            "message": "Run did not emit a terminal turn-complete or turn-aborted event.",
            "sourceRefs": ["raw-agent-events.jsonl"],
        }));
    }

    write_jsonl(&attempt_dir.join("lifecycle-events.jsonl"), &lifecycle_rows)?;
    write_jsonl(&attempt_dir.join("token-snapshots.jsonl"), &token_rows)?;
    write_jsonl(&attempt_dir.join("command-events.jsonl"), &command_rows)?;
    write_jsonl(&attempt_dir.join("tool-events.jsonl"), &tool_rows)?;
    write_jsonl(&attempt_dir.join("patch-events.jsonl"), &patch_rows)?;
    write_jsonl(&attempt_dir.join("anomalies.jsonl"), &anomaly_rows)?;
    write_jsonl(&attempt_dir.join("probe-events.jsonl"), &derived_probes)?;
    write_json_pretty(&attempt_dir.join("probe-summary.json"), &probe_summary)?;

    let claim_evidence = build_claim_evidence(record, task_class, &probe_summary);
    write_json_pretty(&attempt_dir.join("claim-evidence.json"), &claim_evidence)?;

    let patch_sha256 = if patch_text.is_empty() {
        None
    } else {
        let mut hasher = Sha256::new();
        hasher.update(patch_text);
        Some(format!("{:x}", hasher.finalize()))
    };

    let summary = RunSummary {
        instance_id: record.instance_id.clone(),
        repo: record.repo.clone(),
        task_class: task_class.to_string(),
        status: if decoded_events
            .iter()
            .any(|event| matches!(event.msg, EventMsg::TurnComplete(_)))
        {
            "completed".to_string()
        } else if decoded_events
            .iter()
            .any(|event| matches!(event.msg, EventMsg::TurnAborted(_)))
        {
            "aborted".to_string()
        } else {
            "incomplete".to_string()
        },
        grading_status: "pending".to_string(),
        raw_event_count: decoded_events.len(),
        raw_probe_count: raw_probe_events.len(),
        raw_diagnostic_count: raw_diagnostics.len(),
        token_snapshot_count: token_rows.len(),
        command_count: command_rows.len(),
        tool_count: tool_rows.len(),
        patch_event_count: patch_rows.len(),
        patch_file_count: patch_file_count(patch_text),
        patch_sha256,
        total_input_tokens: last_token_info
            .as_ref()
            .map(|info| info.total_token_usage.input_tokens),
        total_output_tokens: last_token_info
            .as_ref()
            .map(|info| info.total_token_usage.output_tokens),
        total_cache_read_tokens: last_token_info
            .as_ref()
            .map(|info| info.total_token_usage.cached_input_tokens),
        total_tokens: last_token_info
            .as_ref()
            .map(|info| info.total_token_usage.total_tokens),
        model_context_window: last_token_info.as_ref().and_then(|info| info.model_context_window),
        anomaly_count: anomaly_rows.len(),
        event_type_counts,
        probe_code_counts,
        probe_subsystem_counts,
        diagnostic_type_counts,
        artifact_inventory: artifact_inventory_for_attempt(attempt_dir),
    };
    write_json_pretty(&attempt_dir.join("run-summary.json"), &summary)?;
    write_json_pretty(
        &attempt_dir.join("replay.json"),
        &json!({
            "recordPath": attempt_dir.parent().expect("attempt dir has parent").join("record.json"),
            "artifactPaths": artifact_map_for_attempt(attempt_dir),
        }),
    )?;
    Ok(summary)
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
    lines.push(format!(
        "config_freeze_drift_count={}",
        probe_summary.config_freeze_drift_count
    ));
    lines.push(format!(
        "instruction_shift_count={}",
        probe_summary.instruction_shift_count
    ));
    lines.push(format!(
        "harness_friction_count={}",
        probe_summary.harness_friction_count
    ));
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
    lines.push(format!(
        "compaction_rediscovery_count={}",
        probe_summary.compaction_rediscovery_count
    ));
    lines.push(format!(
        "peak_context_utilization_bps={:?}",
        probe_summary.peak_context_utilization_bps
    ));
    lines.push(String::new());
    lines.push("Tool Orchestration Summary".to_string());
    lines.push("==========================".to_string());
    lines.extend(read_jsonl_lines(&attempt_dir.join("tool-events.jsonl"))?);
    lines.push(String::new());
    lines.push("Redundancy Incidents".to_string());
    lines.push("====================".to_string());
    lines.push(format!(
        "repeated_read_count={}",
        probe_summary.repeated_read_count
    ));
    lines.push(format!(
        "repeated_verification_count={}",
        probe_summary.repeated_verification_count
    ));
    lines.push(format!(
        "repeated_git_inspection_count={}",
        probe_summary.repeated_git_inspection_count
    ));
    lines.push(format!(
        "post_submit_activity_count={}",
        probe_summary.post_submit_activity_count
    ));
    lines.push(String::new());
    lines.push("Verification Chain".to_string());
    lines.push("==================".to_string());
    lines.push(format!(
        "first_meaningful_edit_tokens={:?}",
        probe_summary.first_meaningful_edit_tokens
    ));
    lines.push(format!(
        "first_verification_tokens={:?}",
        probe_summary.first_verification_tokens
    ));
    lines.push(format!("first_patch_tokens={:?}", probe_summary.first_patch_tokens));
    lines.push(format!("final_patch_tokens={:?}", probe_summary.final_patch_tokens));
    lines.push(format!(
        "useful_step_proxy={}/{}",
        probe_summary.useful_step_proxy_num, probe_summary.useful_step_proxy_den
    ));
    lines.push(format!(
        "useful_token_proxy={}/{}",
        probe_summary.useful_token_proxy_num, probe_summary.useful_token_proxy_den
    ));
    lines.push(String::new());
    lines.push("Failure Or Success Narrative".to_string());
    lines.push("============================".to_string());
    lines.push(format!(
        "anomaly_count={} raw_event_count={} raw_probe_count={} raw_diagnostic_count={}",
        summary.anomaly_count,
        summary.raw_event_count,
        summary.raw_probe_count,
        summary.raw_diagnostic_count
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
            lines.push(format!(
                "  conflict: {}",
                claim.conflicting_evidence.join("; ")
            ));
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
    lines.push(format!("Grounding docs:"));
    for doc in &manifest.grounding_documents {
        lines.push(format!("- {doc}"));
    }
    lines.push(format!("Reference docs:"));
    for doc in &manifest.reference_documents {
        lines.push(format!("- {doc}"));
    }
    lines.push(String::new());

    lines.push("Codex Architecture Under Observation".to_string());
    lines.push("===================================".to_string());
    for subsystem in architecture_map {
        lines.push(format!("{}: {}", subsystem.id, subsystem.purpose));
        lines.push(format!("  files: {}", subsystem.files.join(", ")));
        lines.push(format!(
            "  reference_docs: {}",
            subsystem.reference_docs.join(", ")
        ));
        lines.push(format!(
            "  visible_events: {}",
            subsystem.visible_events.join(", ")
        ));
        lines.push(format!(
            "  hidden_state: {}",
            subsystem.hidden_state.join(", ")
        ));
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
    lines.push(format!(
        "Run status counts: {}",
        render_count_map(&statuses)
    ));
    lines.push(format!(
        "Task class counts: {}",
        render_count_map(&task_classes)
    ));
    lines.push(format!(
        "Token totals: input={} output={} cache_read={}",
        total_input, total_output, total_cache
    ));
    lines.push(format!(
        "Command totals: {} | Tool totals: {} | Anomalies: {}",
        total_commands, total_tools, total_anomalies
    ));
    if artifact_missing.is_empty() {
        lines.push("Artifact coverage: all expected artifacts present in the latest attempts.".to_string());
    } else {
        lines.push(format!(
            "Artifact coverage gaps: {}",
            render_count_map(&artifact_missing)
        ));
    }
    lines.push(String::new());

    lines.push("Observed Codex Harness Behavior".to_string());
    lines.push("===============================".to_string());
    lines.push(format!(
        "Config/session freezing evidence: {}",
        render_count_map_filtered(&aggregate_probe_codes, "session_")
    ));
    lines.push(format!(
        "Instruction assembly evidence: {}",
        render_count_map_filtered(&aggregate_subsystems, "InstructionChannel")
    ));
    lines.push(format!(
        "Context and compaction evidence: {}",
        render_count_map_filtered(&aggregate_subsystems, "ContextCompaction")
    ));
    lines.push(format!(
        "Tool mediation evidence: {}",
        render_count_map_filtered(&aggregate_subsystems, "ToolMediation")
    ));
    lines.push(format!(
        "Persistence/reconstruction evidence: {}",
        render_count_map_filtered(&aggregate_subsystems, "PersistenceReconstruction")
    ));
    lines.push(format!(
        "Reliability/contention evidence: {}",
        render_count_map_filtered(&aggregate_subsystems, "HarnessFriction")
    ));
    lines.push(String::new());

    lines.push("Task-Behavior Evidence Across SWE-bench Runs".to_string());
    lines.push("============================================".to_string());
    for bundle in bundles {
        lines.push(format!(
            "{} | status={} | class={} | tokens={} | patch={} | compactions={} | repeated_git={} | repeated_verify={} | config_drift={} | friction={}",
            bundle.selected.instance_id,
            bundle.summary.status,
            bundle.summary.task_class,
            bundle.summary.total_tokens.unwrap_or_default(),
            bundle
                .summary
                .patch_sha256
                .clone()
                .unwrap_or_else(|| "-".to_string()),
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
        lines.push(format!("{claim_id}"));
        lines.push(format!("  source: {}", descriptor.source));
        lines.push(format!("  text: {}", descriptor.text));
        lines.push(format!(
            "  operationalization: {}",
            descriptor.operationalization
        ));
        if claim_rows.is_empty() {
            lines.push("  evidence: none captured yet".to_string());
        } else {
            for (instance_id, claim) in claim_rows {
                lines.push(format!("  run: {instance_id} -> {}", claim.label));
                if !claim.supporting_evidence.is_empty() {
                    lines.push(format!(
                        "    support: {}",
                        claim.supporting_evidence.join("; ")
                    ));
                }
                if !claim.conflicting_evidence.is_empty() {
                    lines.push(format!(
                        "    conflict: {}",
                        claim.conflicting_evidence.join("; ")
                    ));
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
        aggregate_probe_codes.get("compaction_completed").copied().unwrap_or_default(),
        aggregate_subsystems.get("InstructionChannel").copied().unwrap_or_default()
    ));
    lines.push(format!(
        "Potentially unlike pure flat-history assumptions when: config_freeze_drift_count_total={} and persistence_probe_total={}",
        bundles
            .iter()
            .map(|bundle| bundle.probe_summary.config_freeze_drift_count)
            .sum::<usize>(),
        aggregate_subsystems
            .get("PersistenceReconstruction")
            .copied()
            .unwrap_or_default()
    ));
    lines.push(format!(
        "Novel Codex-specific signal: harness_friction_count_total={} tool_mediation_tax_total={}",
        bundles
            .iter()
            .map(|bundle| bundle.probe_summary.harness_friction_count)
            .sum::<usize>(),
        bundles
            .iter()
            .map(|bundle| bundle.probe_summary.tool_mediation_tax_count)
            .sum::<usize>()
    ));
    lines.push(String::new());

    lines.push("Threats To Validity".to_string());
    lines.push("===================".to_string());
    lines.push("macOS-only bias: run environments and toolchain behavior are intentionally Mac-hosted in this study path.".to_string());
    lines.push("SWE-bench-only bias: these observations are grounded in benchmark tasks rather than arbitrary human workflows.".to_string());
    lines.push("Hidden reasoning observability limit: internal chain-of-thought remains only partially visible even with raw events and probes.".to_string());
    lines.push("Harness noise: auth, listener, state, or environment friction may contaminate purely cognitive interpretations.".to_string());
    if artifact_missing.is_empty() {
        lines.push("Telemetry gaps: no artifact-coverage gaps detected in the latest attempts.".to_string());
    } else {
        lines.push(format!(
            "Telemetry gaps: {}",
            render_count_map(&artifact_missing)
        ));
    }
    lines.push(String::new());

    lines.push("Run Index".to_string());
    lines.push("=========".to_string());
    for bundle in bundles {
        lines.push(format!(
            "{} | {} | class={} | tokens={} | input={} output={} cache={} | probes={} | anomalies={} | {}",
            bundle.selected.instance_id,
            bundle.summary.status,
            bundle.summary.task_class,
            bundle.summary.total_tokens.unwrap_or_default(),
            bundle.summary.total_input_tokens.unwrap_or_default(),
            bundle.summary.total_output_tokens.unwrap_or_default(),
            bundle.summary.total_cache_read_tokens.unwrap_or_default(),
            bundle.summary.raw_probe_count,
            bundle.summary.anomaly_count,
            bundle
                .artifact_paths
                .get("runEvidence")
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string())
        ));
    }
    lines.push(String::new());

    lines.push("Artifact Appendix".to_string());
    lines.push("=================".to_string());
    lines.push(format!(
        "Campaign manifest: {}",
        campaign_dir.join("campaign-manifest.json").display()
    ));
    lines.push(format!(
        "Architecture map: {}",
        campaign_dir.join("codex-architecture-map.json").display()
    ));
    lines.push(format!(
        "Grounding claims: {}",
        campaign_dir.join("grounding-claims.json").display()
    ));
    lines.push(format!(
        "Codex claims: {}",
        campaign_dir.join("codex-unique-claims.json").display()
    ));
    for bundle in bundles {
        lines.push(format!("{}:", bundle.selected.instance_id));
        for (name, path_ref) in &bundle.artifact_paths {
            lines.push(format!("  {name}: {}", path_ref.display()));
        }
    }

    lines.join("\n")
}

fn handle_raw_probe(
    seq: usize,
    run_id: &str,
    record: &DatasetRecord,
    task_class: &str,
    probe: &StudyProbeEvent,
    derived_probes: &mut Vec<ProbeEventRow>,
    probe_summary: &mut ProbeSummary,
    anomalies: &mut Vec<Value>,
) {
    let subsystem = format!("{:?}", probe.subsystem);
    let classification = format!("{:?}", probe.classification).to_lowercase();
    match classification.as_str() {
        "exact" => probe_summary.exact_probe_count += 1,
        "inferred" => probe_summary.inferred_probe_count += 1,
        "estimated" => probe_summary.estimated_probe_count += 1,
        _ => {}
    }
    if probe.code == "compaction_started" || probe.code == "compaction_completed" {
        probe_summary.compaction_count += usize::from(probe.code == "compaction_completed");
    }
    if probe.code.contains("instruction") || subsystem == "InstructionChannel" {
        probe_summary.instruction_shift_count += 1;
    }
    if probe.code.contains("config") || subsystem == "ConfigFreeze" {
        probe_summary.config_freeze_drift_count += 1;
    }
    if probe.code.contains("listener")
        || probe.code.contains("friction")
        || subsystem == "HarnessFriction"
    {
        probe_summary.harness_friction_count += 1;
    }
    if probe.code == "session_configured"
        && let Some(payload) = &probe.payload
    {
        let requested_model = probe
            .study
            .as_ref()
            .map(|study| study.run_id.clone())
            .unwrap_or_default();
        let effective_model = payload
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        derived_probes.push(ProbeEventRow {
            run_id: run_id.to_string(),
            instance_id: record.instance_id.clone(),
            repo: record.repo.clone(),
            attempt: 1,
            task_class: Some(task_class.to_string()),
            seq: Some(seq),
            timestamp: None,
            subsystem: subsystem.clone(),
            evidence_code: "config.requested_vs_effective".to_string(),
            classification: "inferred".to_string(),
            summary: format!("effective model `{effective_model}` frozen for run `{requested_model}`"),
            source_refs: vec!["codex-probe-events.jsonl".to_string()],
            payload: Some(payload.clone()),
        });
    }
    if probe.code == "compaction_completed"
        && let Some(payload) = &probe.payload
    {
        let before = payload
            .get("historyItemsBefore")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let replacement = payload
            .get("replacementHistoryItems")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        if replacement + 1 < before {
            probe_summary.compaction_rediscovery_count += 1;
            derived_probes.push(ProbeEventRow {
                run_id: run_id.to_string(),
                instance_id: record.instance_id.clone(),
                repo: record.repo.clone(),
                attempt: 1,
                task_class: Some(task_class.to_string()),
                seq: Some(seq),
                timestamp: None,
                subsystem: subsystem.clone(),
                evidence_code: "context.rediscovery".to_string(),
                classification: "inferred".to_string(),
                summary: format!(
                    "compaction replaced {before} history items with {replacement}, suggesting a non-trivial reconstruction boundary"
                ),
                source_refs: vec!["codex-probe-events.jsonl".to_string()],
                payload: Some(payload.clone()),
            });
        }
    }
    if subsystem == "HarnessFriction" {
        anomalies.push(json!({
            "seq": seq,
            "severity": "warning",
            "code": "harness_friction_probe",
            "message": probe.code,
            "sourceRefs": ["codex-probe-events.jsonl"],
        }));
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_command_begin(
    seq: usize,
    run_id: &str,
    record: &DatasetRecord,
    task_class: &str,
    ev: &ExecCommandBeginEvent,
    token_total: Option<i64>,
    command_rows: &mut Vec<Value>,
    derived_probes: &mut Vec<ProbeEventRow>,
    probe_summary: &mut ProbeSummary,
    seen_read_commands_since_write: &mut BTreeSet<String>,
    seen_verification_commands_since_write: &mut BTreeSet<String>,
    seen_git_commands_since_write: &mut BTreeSet<String>,
    last_write_seq: &mut Option<usize>,
) {
    let command_text = join_command(&ev.command);
    command_rows.push(json!({
        "seq": seq,
        "phase": "begin",
        "classification": "exact",
        "callId": ev.call_id,
        "processId": ev.process_id,
        "turnId": ev.turn_id,
        "command": command_text,
        "argv": ev.command,
        "cwd": ev.cwd,
        "source": format!("{:?}", ev.source),
    }));

    if probe_summary.first_meaningful_edit_tokens.is_none()
        && is_meaningful_edit_command(&command_text)
    {
        probe_summary.first_meaningful_edit_tokens = token_total;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "TurnLifecycle",
            "activation.first_meaningful_edit",
            "inferred",
            format!(
                "first likely meaningful edit command observed: `{command_text}` at {:?} total tokens",
                token_total
            ),
            vec!["command-events.jsonl".to_string()],
        ));
    }
    if probe_summary.first_verification_tokens.is_none() && is_verification_command(&command_text) {
        probe_summary.first_verification_tokens = token_total;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "TurnLifecycle",
            "activation.first_verification",
            "inferred",
            format!(
                "first verification command observed: `{command_text}` at {:?} total tokens",
                token_total
            ),
            vec!["command-events.jsonl".to_string()],
        ));
    }
    if is_read_only_command(&command_text) {
        if !seen_read_commands_since_write.insert(command_text.clone()) {
            probe_summary.repeated_read_count += 1;
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                seq,
                "InstructionChannel",
                "redundancy.repeated_read",
                "inferred",
                format!(
                    "repeated read-only command without intervening write: `{command_text}`"
                ),
                vec!["command-events.jsonl".to_string()],
            ));
        }
    } else {
        seen_read_commands_since_write.clear();
    }
    if is_verification_command(&command_text) {
        if !seen_verification_commands_since_write.insert(command_text.clone()) {
            probe_summary.repeated_verification_count += 1;
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                seq,
                "TurnLifecycle",
                "verification.retry_loop",
                "inferred",
                format!(
                    "verification command repeated without intervening write: `{command_text}`"
                ),
                vec!["command-events.jsonl".to_string()],
            ));
        }
        probe_summary.chain_reaction_cycle_count += 1;
    } else if is_meaningful_edit_command(&command_text) {
        seen_verification_commands_since_write.clear();
        *last_write_seq = Some(seq);
    }
    if is_git_inspection_command(&command_text) {
        if !seen_git_commands_since_write.insert(command_text.clone()) {
            probe_summary.repeated_git_inspection_count += 1;
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                seq,
                "HarnessFriction",
                "redundancy.git_loop",
                "inferred",
                format!("repeated git inspection command: `{command_text}`"),
                vec!["command-events.jsonl".to_string()],
            ));
        }
    } else if is_meaningful_edit_command(&command_text) {
        seen_git_commands_since_write.clear();
    }
}

fn handle_command_end(
    seq: usize,
    ev: &ExecCommandEndEvent,
    command_rows: &mut Vec<Value>,
    derived_probes: &mut Vec<ProbeEventRow>,
    probe_summary: &mut ProbeSummary,
    last_write_seq: Option<usize>,
    last_patch_seq: Option<usize>,
) {
    let command_text = join_command(&ev.command);
    command_rows.push(json!({
        "seq": seq,
        "phase": "end",
        "classification": "exact",
        "callId": ev.call_id,
        "turnId": ev.turn_id,
        "command": command_text,
        "cwd": ev.cwd,
        "exitCode": ev.exit_code,
        "durationMs": duration_to_ms_i64(ev.duration),
        "stdoutBytes": ev.stdout.len(),
        "stderrBytes": ev.stderr.len(),
        "aggregatedOutputBytes": ev.aggregated_output.len(),
    }));
    if let Some(patch_seq) = last_patch_seq
        && seq > patch_seq
    {
        probe_summary.post_submit_activity_count += 1;
        derived_probes.push(make_probe(
            "unknown",
            &DatasetRecord {
                instance_id: String::new(),
                repo: String::new(),
                base_commit: String::new(),
                patch: None,
                test_patch: None,
                problem_statement: String::new(),
                hints_text: None,
                version: None,
                environment_setup_commit: None,
                fail_to_pass: Vec::new(),
                pass_to_pass: Vec::new(),
                raw: Value::Null,
            },
            "",
            seq,
            "TurnLifecycle",
            "redundancy.post_submit",
            "inferred",
            format!("command observed after patch application: `{command_text}`"),
            vec!["command-events.jsonl".to_string()],
        ));
    }
    if let Some(write_seq) = last_write_seq
        && is_verification_command(&command_text)
        && seq > write_seq
    {
        probe_summary.useful_step_proxy_num += 1;
        probe_summary.useful_token_proxy_num += 1.max(ev.duration.as_millis() as i64);
    }
    if !is_verification_command(&command_text) && !is_meaningful_edit_command(&command_text) {
        probe_summary.friction_token_proxy_num += 1.max(ev.duration.as_millis() as i64);
    }
}

fn build_claim_evidence(
    record: &DatasetRecord,
    task_class: &str,
    probe_summary: &ProbeSummary,
) -> Vec<ClaimEvidence> {
    let run_ref = format!("{}[{task_class}]", record.instance_id);
    vec![
        ClaimEvidence {
            claim_id: "grounding.activation_threshold".to_string(),
            label: if probe_summary.first_meaningful_edit_tokens.is_some() {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: probe_summary
                .first_meaningful_edit_tokens
                .map(|tokens| format!("first meaningful edit observed at total tokens={tokens}"))
                .into_iter()
                .chain(
                    probe_summary
                        .first_verification_tokens
                        .map(|tokens| format!("first verification observed at total tokens={tokens}")),
                )
                .collect(),
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Single-run evidence does not establish a full sigmoid curve.".to_string()],
        },
        ClaimEvidence {
            claim_id: "grounding.flat_history_tax".to_string(),
            label: if probe_summary.repeated_read_count > 0
                || probe_summary.compaction_count > 0
                || probe_summary.cache_read_ratio_num > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!("repeated_read_count={}", probe_summary.repeated_read_count),
                format!("compaction_count={}", probe_summary.compaction_count),
                format!(
                    "cache_read_ratio={}/{}",
                    probe_summary.cache_read_ratio_num, probe_summary.cache_read_ratio_den
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Repeated-read evidence is inferred from shell-level telemetry.".to_string()],
        },
        ClaimEvidence {
            claim_id: "grounding.verification_pressure".to_string(),
            label: if probe_summary.first_verification_tokens.is_some()
                && probe_summary.useful_step_proxy_num > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!(
                    "first_verification_tokens={:?}",
                    probe_summary.first_verification_tokens
                ),
                format!(
                    "useful_step_proxy={}/{}",
                    probe_summary.useful_step_proxy_num, probe_summary.useful_step_proxy_den
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Closure is inferred from command chronology rather than semantic proof graphs.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.compaction_rebuild".to_string(),
            label: if probe_summary.compaction_count > 0 {
                "evidence_consistent"
            } else {
                "not_observable_with_current_probes"
            }
            .to_string(),
            supporting_evidence: vec![
                format!("compaction_count={}", probe_summary.compaction_count),
                format!(
                    "compaction_rediscovery_count={}",
                    probe_summary.compaction_rediscovery_count
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Needs compaction-heavy tasks for strong evidence.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.config_freeze".to_string(),
            label: if probe_summary.config_freeze_drift_count > 0 {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![format!(
                "config_freeze_drift_count={}",
                probe_summary.config_freeze_drift_count
            )],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Some apparent drift may reflect intentional provider normalization.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.tool_mediation".to_string(),
            label: if probe_summary.tool_mediation_tax_count > 0 {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![format!(
                "tool_mediation_tax_count={}",
                probe_summary.tool_mediation_tax_count
            )],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["A given run may not exercise MCP and patch pathways equally.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.harness_overhead_tax".to_string(),
            label: if probe_summary.harness_friction_count > 0
                || probe_summary.friction_token_proxy_num > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!("harness_friction_count={}", probe_summary.harness_friction_count),
                format!(
                    "friction_token_proxy={}/{}",
                    probe_summary.friction_token_proxy_num, probe_summary.friction_token_proxy_den
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref],
            caveats: vec!["Friction-token estimates are inferred from event sequencing and command duration.".to_string()],
        },
    ]
}

fn artifact_map_for_attempt(attempt_dir: &Path) -> BTreeMap<String, PathBuf> {
    BTreeMap::from([
        ("prompt".to_string(), attempt_dir.join("prompt.txt")),
        (
            "environmentPlan".to_string(),
            attempt_dir.join("environment-plan.json"),
        ),
        (
            "rawAgentEvents".to_string(),
            attempt_dir.join("raw-agent-events.jsonl"),
        ),
        (
            "rawDiagnostics".to_string(),
            attempt_dir.join("raw-diagnostics.jsonl"),
        ),
        (
            "codexProbeEvents".to_string(),
            attempt_dir.join("codex-probe-events.jsonl"),
        ),
        (
            "lifecycleEvents".to_string(),
            attempt_dir.join("lifecycle-events.jsonl"),
        ),
        (
            "tokenSnapshots".to_string(),
            attempt_dir.join("token-snapshots.jsonl"),
        ),
        (
            "commandEvents".to_string(),
            attempt_dir.join("command-events.jsonl"),
        ),
        ("toolEvents".to_string(), attempt_dir.join("tool-events.jsonl")),
        ("patchEvents".to_string(), attempt_dir.join("patch-events.jsonl")),
        ("probeEvents".to_string(), attempt_dir.join("probe-events.jsonl")),
        ("probeSummary".to_string(), attempt_dir.join("probe-summary.json")),
        ("claimEvidence".to_string(), attempt_dir.join("claim-evidence.json")),
        ("patch".to_string(), attempt_dir.join("patch.diff")),
        ("runSummary".to_string(), attempt_dir.join("run-summary.json")),
        ("runEvidence".to_string(), attempt_dir.join("run-evidence.txt")),
        ("replay".to_string(), attempt_dir.join("replay.json")),
        ("anomalies".to_string(), attempt_dir.join("anomalies.jsonl")),
    ])
}

fn artifact_inventory_for_attempt(attempt_dir: &Path) -> BTreeMap<String, bool> {
    artifact_map_for_attempt(attempt_dir)
        .into_iter()
        .map(|(name, path)| (name, path.exists()))
        .collect()
}

fn patch_file_count(patch_text: &[u8]) -> usize {
    patch_text
        .split(|byte| *byte == b'\n')
        .filter(|line| line.starts_with(b"diff --git "))
        .count()
}

fn last_token_total(info: &Option<TokenUsageInfo>) -> Option<i64> {
    info.as_ref().map(|info| info.total_token_usage.total_tokens)
}

fn duration_to_ms_i64(duration: Duration) -> i64 {
    duration.as_millis().min(i64::MAX as u128) as i64
}

fn join_command(parts: &[String]) -> String {
    parts.join(" ")
}

fn is_meaningful_edit_command(command: &str) -> bool {
    let lowered = command.to_ascii_lowercase();
    lowered.contains("apply_patch")
        || lowered.contains("python -c")
        || lowered.contains("python3 -c")
        || lowered.contains("perl -0pi")
        || lowered.contains("sed -i")
        || lowered.contains("tee ")
        || lowered.contains(" > ")
}

fn is_verification_command(command: &str) -> bool {
    let lowered = command.to_ascii_lowercase();
    [
        "pytest",
        "py.test",
        "cargo test",
        "cargo nextest",
        "pnpm test",
        "npm test",
        "yarn test",
        "go test",
        "tox",
        "nosetests",
        "unittest",
        "manage.py test",
        "ruff check",
        "mypy",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn is_git_inspection_command(command: &str) -> bool {
    let lowered = command.to_ascii_lowercase();
    lowered.starts_with("git status")
        || lowered.starts_with("git diff")
        || lowered.starts_with("git log")
        || lowered.starts_with("git show")
    || lowered.starts_with("git rev-parse")
}

fn is_read_only_command(command: &str) -> bool {
    let lowered = command.to_ascii_lowercase();
    lowered.starts_with("rg ")
        || lowered.starts_with("grep ")
        || lowered.starts_with("fd ")
        || lowered.starts_with("find ")
        || lowered.starts_with("ls")
        || lowered.starts_with("cat ")
        || lowered.starts_with("sed -n")
        || lowered.starts_with("head ")
        || lowered.starts_with("tail ")
        || is_git_inspection_command(command)
}

fn make_probe(
    run_id: &str,
    record: &DatasetRecord,
    task_class: &str,
    seq: usize,
    subsystem: &str,
    evidence_code: &str,
    classification: &str,
    summary: String,
    source_refs: Vec<String>,
) -> ProbeEventRow {
    ProbeEventRow {
        run_id: run_id.to_string(),
        instance_id: record.instance_id.clone(),
        repo: record.repo.clone(),
        attempt: 1,
        task_class: Some(task_class.to_string()),
        seq: Some(seq),
        timestamp: None,
        subsystem: subsystem.to_string(),
        evidence_code: evidence_code.to_string(),
        classification: classification.to_string(),
        summary,
        source_refs,
        payload: None,
    }
}

fn event_type_name(msg: &EventMsg) -> &'static str {
    match msg {
        EventMsg::SessionConfigured(_) => "session_configured",
        EventMsg::TurnStarted(_) => "turn_started",
        EventMsg::TurnComplete(_) => "turn_complete",
        EventMsg::TurnAborted(_) => "turn_aborted",
        EventMsg::TokenCount(_) => "token_count",
        EventMsg::ExecCommandBegin(_) => "exec_command_begin",
        EventMsg::ExecCommandEnd(_) => "exec_command_end",
        EventMsg::McpToolCallBegin(_) => "mcp_tool_call_begin",
        EventMsg::McpToolCallEnd(_) => "mcp_tool_call_end",
        EventMsg::PatchApplyBegin(_) => "patch_apply_begin",
        EventMsg::PatchApplyEnd(_) => "patch_apply_end",
        EventMsg::StudyProbe(_) => "study_probe",
        EventMsg::Warning(_) => "warning",
        EventMsg::Error(_) => "error",
        EventMsg::StreamError(_) => "stream_error",
        _ => "other",
    }
}

fn render_count_map(map: &BTreeMap<String, usize>) -> String {
    if map.is_empty() {
        return "none".to_string();
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
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        "none".to_string()
    } else {
        filtered.join(", ")
    }
}

fn read_jsonl_lines(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(vec!["missing".to_string()]);
    }
    let text = fs::read_to_string(path)?;
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        Ok(vec!["empty".to_string()])
    } else {
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn command_heuristics_detect_expected_patterns() {
        assert!(is_verification_command("pytest tests/test_app.py"));
        assert!(is_git_inspection_command("git diff --stat"));
        assert!(is_read_only_command("rg -n token_count src"));
        assert!(is_meaningful_edit_command("perl -0pi -e 's/a/b/' file.py"));
    }

    #[test]
    fn run_evidence_report_renders_known_sections() {
        let dir = tempdir().expect("tempdir");
        let attempt = dir.path().join("attempt-01");
        fs::create_dir_all(&attempt).expect("create attempt dir");
        write_json_pretty(&attempt.join("probe-summary.json"), &ProbeSummary::default())
            .expect("write probe summary");
        write_json_pretty(
            &attempt.join("claim-evidence.json"),
            &Vec::<ClaimEvidence>::new(),
        )
        .expect("write claims");
        fs::write(&attempt.join("lifecycle-events.jsonl"), "{}\n").expect("write lifecycle");
        fs::write(&attempt.join("tool-events.jsonl"), "{}\n").expect("write tool events");
        let record = DatasetRecord {
            instance_id: "demo__repo-1".to_string(),
            repo: "demo/repo".to_string(),
            base_commit: "abc".to_string(),
            patch: None,
            test_patch: None,
            problem_statement: "problem".to_string(),
            hints_text: None,
            version: None,
            environment_setup_commit: None,
            fail_to_pass: Vec::new(),
            pass_to_pass: Vec::new(),
            raw: Value::Null,
        };
        let summary = RunSummary {
            instance_id: record.instance_id.clone(),
            repo: record.repo.clone(),
            task_class: "patch-heavy".to_string(),
            status: "completed".to_string(),
            grading_status: "pending".to_string(),
            patch_sha256: Some("deadbeef".to_string()),
            ..RunSummary::default()
        };
        let path = render_run_evidence(&attempt, &record, &summary).expect("render evidence");
        let text = fs::read_to_string(path).expect("read evidence");
        assert!(text.contains("Run Summary"));
        assert!(text.contains("Verification Chain"));
        assert!(text.contains("Artifact Paths"));
    }
}
