use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use codex_bench_core::{
    ClaimEvidence, DatasetRecord, ProbeEventRow, ProbeSummary, RunSummary,
    artifact_inventory_for_attempt, patch_file_count, write_json_pretty, write_jsonl,
};
use codex_protocol::protocol::{
    Event, EventMsg, ExecCommandBeginEvent, ExecCommandEndEvent, StudyProbeEvent, TokenUsageInfo,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

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
                    run_id,
                    record,
                    task_class,
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
        if kind == "lagged" {
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                decoded_events.len(),
                "AppServerTranslation",
                "events.backpressure",
                "exact",
                "the app-server client reported lagged event delivery".to_string(),
                vec!["raw-diagnostics.jsonl".to_string()],
            ));
        }
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            decoded_events.len(),
            "ContextCompaction",
            "context.cache_read_ratio",
            "exact",
            format!(
                "cache-read ratio observed as {}/{} input tokens",
                probe_summary.cache_read_ratio_num, probe_summary.cache_read_ratio_den
            ),
            vec!["token-snapshots.jsonl".to_string()],
        ));
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
            "artifactPaths": codex_bench_core::artifact_map_for_attempt(attempt_dir),
        }),
    )?;
    Ok(summary)
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "ContextCompaction",
            "context.compaction",
            "exact",
            format!("raw compaction probe observed: {}", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
    }
    if probe.code.contains("instruction") || subsystem == "InstructionChannel" {
        probe_summary.instruction_shift_count += 1;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "InstructionChannel",
            "instruction.channel_mix",
            "inferred",
            format!("instruction-layer event observed: {}", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
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
        let effective_model = payload
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let requested_run = probe
            .study
            .as_ref()
            .map(|study| study.run_id.clone())
            .unwrap_or_default();
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
            summary: format!(
                "effective model `{effective_model}` frozen for study run `{requested_run}`"
            ),
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
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                seq,
                "ContextCompaction",
                "context.rediscovery",
                "inferred",
                format!(
                    "compaction replaced {before} history items with {replacement}, suggesting a non-trivial reconstruction boundary"
                ),
                vec!["codex-probe-events.jsonl".to_string()],
            ));
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "HarnessFriction",
            "harness.friction",
            classification.as_str(),
            format!("harness friction event observed: {}", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
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

    if probe_summary.first_meaningful_edit_tokens.is_none() && is_meaningful_edit_command(&command_text) {
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "TurnLifecycle",
            "fission.ignition",
            "inferred",
            format!("run ignition appears to happen via `{command_text}`"),
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
                format!("repeated read-only command without intervening write: `{command_text}`"),
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
                format!("verification command repeated without intervening write: `{command_text}`"),
                vec!["command-events.jsonl".to_string()],
            ));
        }
        probe_summary.chain_reaction_cycle_count += 1;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "TurnLifecycle",
            "chain_reaction.cycle",
            "inferred",
            format!("productive reaction cycle advanced through verification command `{command_text}`"),
            vec!["command-events.jsonl".to_string()],
        ));
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
    run_id: &str,
    record: &DatasetRecord,
    task_class: &str,
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
    if let Some(patch_seq) = last_patch_seq && seq > patch_seq {
        probe_summary.post_submit_activity_count += 1;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "TurnLifecycle",
            "verification.edit_closure",
            "inferred",
            format!("verification closed after earlier write via `{command_text}`"),
            vec!["command-events.jsonl".to_string()],
        ));
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
                format!("cache_read_ratio={}/{}", probe_summary.cache_read_ratio_num, probe_summary.cache_read_ratio_den),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Repeated-read evidence is inferred from shell-level telemetry.".to_string()],
        },
        ClaimEvidence {
            claim_id: "grounding.verification_pressure".to_string(),
            label: if probe_summary.first_verification_tokens.is_some() && probe_summary.useful_step_proxy_num > 0 {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!("first_verification_tokens={:?}", probe_summary.first_verification_tokens),
                format!("useful_step_proxy={}/{}", probe_summary.useful_step_proxy_num, probe_summary.useful_step_proxy_den),
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
                format!("compaction_rediscovery_count={}", probe_summary.compaction_rediscovery_count),
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
            supporting_evidence: vec![format!("config_freeze_drift_count={}", probe_summary.config_freeze_drift_count)],
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
            supporting_evidence: vec![format!("tool_mediation_tax_count={}", probe_summary.tool_mediation_tax_count)],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["A given run may not exercise MCP and patch pathways equally.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.harness_overhead_tax".to_string(),
            label: if probe_summary.harness_friction_count > 0 || probe_summary.friction_token_proxy_num > 0 {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!("harness_friction_count={}", probe_summary.harness_friction_count),
                format!("friction_token_proxy={}/{}", probe_summary.friction_token_proxy_num, probe_summary.friction_token_proxy_den),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref],
            caveats: vec!["Friction-token estimates are inferred from event sequencing and command duration.".to_string()],
        },
    ]
}

fn last_token_total(info: &Option<TokenUsageInfo>) -> Option<i64> {
    info.as_ref().map(|info| info.total_token_usage.total_tokens)
}

fn duration_to_ms_i64(duration: Duration) -> i64 {
    duration.as_millis().try_into().unwrap_or(i64::MAX)
}

fn join_command(parts: &[String]) -> String {
    parts.join(" ")
}

fn is_meaningful_edit_command(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    ["sed -i", "perl -0pi", "python -c", "python3 -c", "apply_patch", "git apply", "cat >"]
        .iter()
        .any(|needle| command.contains(needle))
        || command.starts_with("ed ")
        || command.starts_with("perl ")
}

fn is_verification_command(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    [
        "pytest",
        "tox",
        "nox",
        "cargo test",
        "pnpm test",
        "npm test",
        "yarn test",
        "vitest",
        "jest",
        "ruff check",
        "mypy",
        "python -m pytest",
    ]
    .iter()
    .any(|needle| command.contains(needle))
}

fn is_git_inspection_command(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    command.starts_with("git status")
        || command.starts_with("git diff")
        || command.starts_with("git log")
        || command.starts_with("git show")
}

fn is_read_only_command(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    ["rg ", "grep ", "find ", "ls ", "cat ", "sed -n", "git grep", "git show", "git diff"]
        .iter()
        .any(|needle| command.starts_with(needle))
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
