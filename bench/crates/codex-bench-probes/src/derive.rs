use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use codex_bench_core::{
    ClaimEvidence, DatasetRecord, ProbeEventRow, ProbeSummary, RunManifest, RunSummary,
    artifact_inventory_for_attempt, patch_file_count, read_json, write_json_pretty, write_jsonl,
};
use codex_protocol::protocol::{
    Event, EventMsg, ExecCommandBeginEvent, ExecCommandEndEvent, StudyProbeEvent, TokenUsageInfo,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Default)]
struct TurnAccumulator {
    turn_id: String,
    status: String,
    start_seq: usize,
    end_seq: Option<usize>,
    start_input_tokens: i64,
    start_output_tokens: i64,
    start_cache_read_tokens: i64,
    start_total_tokens: i64,
    end_input_tokens: i64,
    end_output_tokens: i64,
    end_cache_read_tokens: i64,
    end_total_tokens: i64,
    model_context_window: Option<i64>,
    token_snapshot_count: usize,
    command_count: usize,
    shell_command_count: usize,
    mcp_tool_count: usize,
    patch_apply_count: usize,
    skill_event_count: usize,
    first_command: Option<String>,
    last_command: Option<String>,
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
    let mut turn_rows = Vec::<Value>::new();
    let mut message_rows = Vec::<Value>::new();
    let mut command_rows = Vec::<Value>::new();
    let mut tool_rows = Vec::<Value>::new();
    let mut skill_rows = Vec::<Value>::new();
    let mut patch_rows = Vec::<Value>::new();
    let mut lifecycle_rows = Vec::<Value>::new();
    let mut anomaly_rows = Vec::<Value>::new();
    let mut coupling_rows = Vec::<Value>::new();
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
    let mut current_turn_id = None::<String>;
    let mut turn_accumulators = BTreeMap::<String, TurnAccumulator>::new();
    let mut tool_kind_counts = BTreeMap::<String, usize>::new();
    let mut tool_name_counts = BTreeMap::<String, usize>::new();
    let mut skill_name_counts = BTreeMap::<String, usize>::new();
    let mut message_category_counts = BTreeMap::<String, usize>::new();

    let mut seen_read_commands_since_write = BTreeSet::<String>::new();
    let mut seen_verification_commands_since_write = BTreeSet::<String>::new();
    let mut seen_git_commands_since_write = BTreeSet::<String>::new();
    let mut visible_output_total_chars = 0usize;
    let mut visible_output_total_tokens_est = 0i64;
    let mut visible_output_sentence_count = 0usize;
    let mut visible_output_paragraph_count = 0usize;
    let mut visible_output_bullet_count = 0usize;
    let mut visible_output_codeblock_count = 0usize;
    let mut actionable_commentary_tokens = 0i64;
    let mut tool_grounded_commentary_tokens = 0i64;
    let mut verification_grounded_commentary_tokens = 0i64;
    let mut restatement_tokens = 0i64;
    let mut redundant_commentary_tokens = 0i64;
    let mut speculation_tokens = 0i64;
    let mut social_tone_tokens = 0i64;
    let mut commentary_chars_since_last_tool = 0usize;
    let mut commentary_tokens_since_last_tool = 0i64;
    let mut commentary_messages_since_last_tool = 0usize;
    let mut first_tool_seen = false;
    let mut coupling_index = 0usize;

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
                current_turn_id = Some(ev.turn_id.clone());
                let mut acc = TurnAccumulator {
                    turn_id: ev.turn_id.clone(),
                    status: "running".to_string(),
                    start_seq: seq,
                    model_context_window: ev.model_context_window,
                    ..Default::default()
                };
                if let Some(info) = &last_token_info {
                    acc.start_input_tokens = info.total_token_usage.input_tokens;
                    acc.start_output_tokens = info.total_token_usage.output_tokens;
                    acc.start_cache_read_tokens = info.total_token_usage.cached_input_tokens;
                    acc.start_total_tokens = info.total_token_usage.total_tokens;
                    acc.end_input_tokens = acc.start_input_tokens;
                    acc.end_output_tokens = acc.start_output_tokens;
                    acc.end_cache_read_tokens = acc.start_cache_read_tokens;
                    acc.end_total_tokens = acc.start_total_tokens;
                }
                turn_accumulators.insert(ev.turn_id.clone(), acc);
                lifecycle_rows.push(json!({
                    "seq": seq,
                    "kind": "turn_started",
                    "turnId": ev.turn_id,
                    "modelContextWindow": ev.model_context_window,
                }));
            }
            EventMsg::TurnComplete(ev) => {
                if let Some(acc) = turn_accumulators.get_mut(&ev.turn_id) {
                    acc.status = "completed".to_string();
                    acc.end_seq = Some(seq);
                    if let Some(info) = &last_token_info {
                        update_turn_token_state(acc, info);
                    }
                    turn_rows.push(turn_row(acc));
                }
                if current_turn_id.as_deref() == Some(ev.turn_id.as_str()) {
                    current_turn_id = None;
                }
                lifecycle_rows.push(json!({
                    "seq": seq,
                    "kind": "turn_complete",
                    "turnId": ev.turn_id,
                    "lastAgentMessage": ev.last_agent_message,
                }));
            }
            EventMsg::TurnAborted(ev) => {
                if let Some(turn_id) = ev.turn_id.as_ref() {
                    if let Some(acc) = turn_accumulators.get_mut(turn_id) {
                        acc.status = "aborted".to_string();
                        acc.end_seq = Some(seq);
                        if let Some(info) = &last_token_info {
                            update_turn_token_state(acc, info);
                        }
                        turn_rows.push(turn_row(acc));
                    }
                }
                if current_turn_id.as_deref() == ev.turn_id.as_deref() {
                    current_turn_id = None;
                }
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
                    if let Some(turn_id) = current_turn_id.as_ref()
                        && let Some(acc) = turn_accumulators.get_mut(turn_id)
                    {
                        acc.token_snapshot_count += 1;
                        update_turn_token_state(acc, info);
                    }
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
                *tool_kind_counts.entry("shell".to_string()).or_default() += 1;
                if !first_tool_seen {
                    first_tool_seen = true;
                    probe_summary.tokens_before_first_tool = last_token_total(&last_token_info);
                    probe_summary.visible_text_before_first_tool_chars =
                        Some(commentary_chars_since_last_tool);
                }
                let burst_label = classify_tool_burst(
                    commentary_messages_since_last_tool,
                    commentary_tokens_since_last_tool,
                );
                if burst_label == "silent_tool_burst" {
                    probe_summary.silent_tool_burst_count += 1;
                }
                if burst_label == "micro_narrated_tool_burst" {
                    probe_summary.micro_narrated_tool_burst_count += 1;
                }
                probe_summary.tool_burst_count += 1;
                coupling_rows.push(json!({
                    "seq": seq,
                    "index": coupling_index,
                    "turnId": ev.turn_id,
                    "kind": "shell",
                    "name": "shell",
                    "command": join_command(&ev.command),
                    "visibleCharsSinceLastTool": commentary_chars_since_last_tool,
                    "visibleTokensSinceLastTool": commentary_tokens_since_last_tool,
                    "visibleMessagesSinceLastTool": commentary_messages_since_last_tool,
                    "burstLabel": burst_label,
                    "classification": "inferred",
                }));
                coupling_index += 1;
                commentary_chars_since_last_tool = 0;
                commentary_tokens_since_last_tool = 0;
                commentary_messages_since_last_tool = 0;
                tool_rows.push(json!({
                    "seq": seq,
                    "phase": "begin",
                    "classification": "exact",
                    "kind": "shell",
                    "name": "shell",
                    "turnId": ev.turn_id,
                    "callId": ev.call_id,
                    "command": join_command(&ev.command),
                    "cwd": ev.cwd,
                }));
                if let Some(acc) = turn_accumulators.get_mut(&ev.turn_id) {
                    acc.command_count += 1;
                    acc.shell_command_count += 1;
                    let command_text = join_command(&ev.command);
                    if acc.first_command.is_none() {
                        acc.first_command = Some(command_text.clone());
                    }
                    acc.last_command = Some(command_text.clone());
                    let detected_skills = detect_skills_from_command(&command_text);
                    for skill_name in detected_skills {
                        acc.skill_event_count += 1;
                        *skill_name_counts.entry(skill_name.clone()).or_default() += 1;
                        skill_rows.push(json!({
                            "seq": seq,
                            "turnId": ev.turn_id,
                            "classification": "inferred",
                            "kind": "command_skill_access",
                            "skillName": skill_name,
                            "command": command_text,
                            "cwd": ev.cwd,
                        }));
                    }
                }
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
                tool_rows.push(json!({
                    "seq": seq,
                    "phase": "end",
                    "classification": "exact",
                    "kind": "shell",
                    "name": "shell",
                    "turnId": ev.turn_id,
                    "callId": ev.call_id,
                    "command": join_command(&ev.command),
                    "cwd": ev.cwd,
                    "exitCode": ev.exit_code,
                    "durationMs": duration_to_ms_i64(ev.duration),
                    "success": ev.exit_code == 0,
                }));
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
            EventMsg::AgentMessage(ev) => {
                let phase = ev
                    .phase
                    .as_ref()
                    .map(|phase| format!("{phase:?}").to_lowercase())
                    .unwrap_or_else(|| "unknown".to_string());
                let categories = classify_agent_message(&ev.message, &phase);
                let text_chars = ev.message.chars().count();
                let text_tokens_est = estimate_text_tokens(&ev.message);
                let sentence_count = count_sentences(&ev.message);
                let paragraph_count = count_paragraphs(&ev.message);
                let bullet_count = count_bullets(&ev.message);
                let codeblock_count = ev.message.matches("```").count() / 2;
                let contains_question = ev.message.contains('?') || ev.message.contains('？');
                let contains_uncertainty = contains_any(&ev.message, &["maybe", "might", "probably", "可能", "也许", "perhaps", "unclear"]);
                let contains_next_step = contains_any(&ev.message, &["next", "接下来", "now i", "i'll", "will ", "计划", "then "]);
                let contains_tool_intent = contains_any(&ev.message, &["run", "inspect", "check", "test", "edit", "patch", "search", "打开", "查看", "测试", "修改"]);
                let contains_verification_language = contains_any(&ev.message, &["verify", "verified", "test", "tests", "确认", "验证", "通过"]);
                let contains_result_claim = contains_any(&ev.message, &["fixed", "resolved", "done", "完成", "修复", "解决"]);
                let contains_empathy_or_alignment_language = contains_any(&ev.message, &["we ", "let's", "together", "我会", "我们", "一起", "friendly"]);
                for category in &categories {
                    *message_category_counts.entry(category.clone()).or_default() += 1;
                }
                if categories.iter().any(|category| matches!(category.as_str(), "planning" | "decision_explanation" | "tool_bridge_before" | "tool_bridge_after" | "verification_framing" | "result_framing" | "observation")) {
                    actionable_commentary_tokens += text_tokens_est;
                }
                if categories.iter().any(|category| matches!(category.as_str(), "tool_bridge_before" | "tool_bridge_after")) {
                    tool_grounded_commentary_tokens += text_tokens_est;
                }
                if categories.iter().any(|category| matches!(category.as_str(), "verification_framing" | "result_framing")) || contains_verification_language {
                    verification_grounded_commentary_tokens += text_tokens_est;
                }
                if categories.iter().any(|category| category == "task_restatement") {
                    restatement_tokens += text_tokens_est;
                }
                if categories.iter().any(|category| category == "redundant_recap") {
                    redundant_commentary_tokens += text_tokens_est;
                }
                if contains_uncertainty {
                    speculation_tokens += text_tokens_est;
                }
                if categories.iter().any(|category| category == "social_tone") || contains_empathy_or_alignment_language {
                    social_tone_tokens += text_tokens_est;
                }
                visible_output_total_chars += text_chars;
                visible_output_total_tokens_est += text_tokens_est;
                visible_output_sentence_count += sentence_count;
                visible_output_paragraph_count += paragraph_count;
                visible_output_bullet_count += bullet_count;
                visible_output_codeblock_count += codeblock_count;
                commentary_chars_since_last_tool += text_chars;
                commentary_tokens_since_last_tool += text_tokens_est;
                commentary_messages_since_last_tool += 1;
                message_rows.push(json!({
                    "seq": seq,
                    "classification": "inferred",
                    "turnId": current_turn_id,
                    "phase": phase,
                    "messageId": format!("{run_id}-message-{seq}"),
                    "textChars": text_chars,
                    "textTokensEst": text_tokens_est,
                    "sentenceCount": sentence_count,
                    "paragraphCount": paragraph_count,
                    "bulletCount": bullet_count,
                    "codeblockCount": codeblock_count,
                    "containsQuestion": contains_question,
                    "containsUncertainty": contains_uncertainty,
                    "containsNextStep": contains_next_step,
                    "containsToolIntent": contains_tool_intent,
                    "containsVerificationLanguage": contains_verification_language,
                    "containsResultClaim": contains_result_claim,
                    "containsEmpathyOrAlignmentLanguage": contains_empathy_or_alignment_language,
                    "categories": categories,
                    "message": ev.message,
                }));
            }
            EventMsg::McpToolCallBegin(ev) => {
                *tool_kind_counts.entry("mcp".to_string()).or_default() += 1;
                *tool_name_counts
                    .entry(format!("{}::{}", ev.invocation.server, ev.invocation.tool))
                    .or_default() += 1;
                if !first_tool_seen {
                    first_tool_seen = true;
                    probe_summary.tokens_before_first_tool = last_token_total(&last_token_info);
                    probe_summary.visible_text_before_first_tool_chars =
                        Some(commentary_chars_since_last_tool);
                }
                let burst_label = classify_tool_burst(
                    commentary_messages_since_last_tool,
                    commentary_tokens_since_last_tool,
                );
                if burst_label == "silent_tool_burst" {
                    probe_summary.silent_tool_burst_count += 1;
                }
                if burst_label == "micro_narrated_tool_burst" {
                    probe_summary.micro_narrated_tool_burst_count += 1;
                }
                probe_summary.tool_burst_count += 1;
                coupling_rows.push(json!({
                    "seq": seq,
                    "index": coupling_index,
                    "turnId": current_turn_id,
                    "kind": "mcp",
                    "name": format!("{}::{}", ev.invocation.server, ev.invocation.tool),
                    "visibleCharsSinceLastTool": commentary_chars_since_last_tool,
                    "visibleTokensSinceLastTool": commentary_tokens_since_last_tool,
                    "visibleMessagesSinceLastTool": commentary_messages_since_last_tool,
                    "burstLabel": burst_label,
                    "classification": "inferred",
                }));
                coupling_index += 1;
                commentary_chars_since_last_tool = 0;
                commentary_tokens_since_last_tool = 0;
                commentary_messages_since_last_tool = 0;
                if let Some(turn_id) = current_turn_id.as_ref()
                    && let Some(acc) = turn_accumulators.get_mut(turn_id)
                {
                    acc.mcp_tool_count += 1;
                }
                if probe_summary.first_controlled_change_tokens.is_none() {
                    probe_summary.first_controlled_change_tokens =
                        last_token_total(&last_token_info);
                    probe_summary.ignition_tool_mediated_count += 1;
                    derived_probes.push(make_probe(
                        run_id,
                        record,
                        task_class,
                        seq,
                        "ToolMediation",
                        "fission.ignition.tool_mediated",
                        "inferred",
                        format!(
                            "first controlled tool-mediated action appears to begin through MCP tool `{}::{}`",
                            ev.invocation.server, ev.invocation.tool
                        ),
                        vec!["tool-events.jsonl".to_string()],
                    ));
                }
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
                *tool_kind_counts.entry("apply_patch".to_string()).or_default() += 1;
                *tool_name_counts.entry("apply_patch".to_string()).or_default() += 1;
                if !first_tool_seen {
                    first_tool_seen = true;
                    probe_summary.tokens_before_first_tool = last_token_total(&last_token_info);
                    probe_summary.visible_text_before_first_tool_chars =
                        Some(commentary_chars_since_last_tool);
                }
                let burst_label = classify_tool_burst(
                    commentary_messages_since_last_tool,
                    commentary_tokens_since_last_tool,
                );
                if burst_label == "silent_tool_burst" {
                    probe_summary.silent_tool_burst_count += 1;
                }
                if burst_label == "micro_narrated_tool_burst" {
                    probe_summary.micro_narrated_tool_burst_count += 1;
                }
                probe_summary.tool_burst_count += 1;
                coupling_rows.push(json!({
                    "seq": seq,
                    "index": coupling_index,
                    "turnId": current_turn_id,
                    "kind": "apply_patch",
                    "name": "apply_patch",
                    "visibleCharsSinceLastTool": commentary_chars_since_last_tool,
                    "visibleTokensSinceLastTool": commentary_tokens_since_last_tool,
                    "visibleMessagesSinceLastTool": commentary_messages_since_last_tool,
                    "burstLabel": burst_label,
                    "classification": "inferred",
                }));
                coupling_index += 1;
                commentary_chars_since_last_tool = 0;
                commentary_tokens_since_last_tool = 0;
                commentary_messages_since_last_tool = 0;
                tool_rows.push(json!({
                    "seq": seq,
                    "phase": "begin",
                    "classification": "exact",
                    "kind": "apply_patch",
                    "name": "apply_patch",
                    "callId": ev.call_id,
                    "autoApproved": ev.auto_approved,
                }));
                if let Some(turn_id) = current_turn_id.as_ref()
                    && let Some(acc) = turn_accumulators.get_mut(turn_id)
                {
                    acc.patch_apply_count += 1;
                }
                if probe_summary.first_controlled_change_tokens.is_none() {
                    probe_summary.first_controlled_change_tokens =
                        last_token_total(&last_token_info);
                    probe_summary.ignition_patch_apply_count += 1;
                    derived_probes.push(make_probe(
                        run_id,
                        record,
                        task_class,
                        seq,
                        "ToolMediation",
                        "fission.ignition.patch_apply",
                        "exact",
                        "first controlled change appears to happen via patch application".to_string(),
                        vec!["patch-events.jsonl".to_string()],
                    ));
                }
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
                tool_rows.push(json!({
                    "seq": seq,
                    "phase": "end",
                    "classification": "exact",
                    "kind": "apply_patch",
                    "name": "apply_patch",
                    "callId": ev.call_id,
                    "status": format!("{:?}", ev.status),
                    "stdoutChars": ev.stdout.chars().count(),
                    "stderrChars": ev.stderr.chars().count(),
                    "success": format!("{:?}", ev.status).eq_ignore_ascii_case("success"),
                }));
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
            EventMsg::ListSkillsResponse(ev) => {
                for entry in &ev.skills {
                    for skill in &entry.skills {
                        *skill_name_counts.entry(skill.name.clone()).or_default() += 1;
                        skill_rows.push(json!({
                            "seq": seq,
                            "classification": "exact",
                            "kind": "listed",
                            "skillName": skill.name,
                            "path": skill.path,
                            "scope": format!("{:?}", skill.scope),
                            "enabled": skill.enabled,
                            "cwd": entry.cwd,
                        }));
                    }
                }
            }
            EventMsg::ListRemoteSkillsResponse(ev) => {
                for skill in &ev.skills {
                    *skill_name_counts.entry(skill.name.clone()).or_default() += 1;
                    skill_rows.push(json!({
                        "seq": seq,
                        "classification": "exact",
                        "kind": "remote_listed",
                        "skillName": skill.name,
                        "remoteSkillId": skill.id,
                        "description": skill.description,
                    }));
                }
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
        *diagnostic_type_counts.entry(kind.clone()).or_default() += 1;
        if kind == "lagged" {
            probe_summary.event_discontinuity_count += 1;
            probe_summary.containment_heat_leak_count += 1;
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
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                decoded_events.len(),
                "HarnessFriction",
                "containment.heat_leak",
                "exact",
                "event delivery lag suggests orchestration heat leakage rather than direct task progress".to_string(),
                vec!["raw-diagnostics.jsonl".to_string()],
            ));
        }
    }

    for acc in turn_accumulators.values() {
        if acc.end_seq.is_none() {
            let mut running = acc.clone();
            running.status = "running".to_string();
            turn_rows.push(turn_row(&running));
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
    if probe_summary.useful_token_proxy_den > 0 {
        probe_summary.useful_token_proxy_bps = Some(
            (probe_summary.useful_token_proxy_num * 10_000) / probe_summary.useful_token_proxy_den,
        );
    }
    if probe_summary.friction_token_proxy_den > 0 {
        probe_summary.friction_token_proxy_bps = Some(
            (probe_summary.friction_token_proxy_num * 10_000)
                / probe_summary.friction_token_proxy_den,
        );
    }
    if let (Some(useful_bps), Some(friction_bps)) = (
        probe_summary.useful_token_proxy_bps,
        probe_summary.friction_token_proxy_bps,
    ) {
        probe_summary.harness_overhead_proxy_bps = Some(friction_bps.saturating_sub(useful_bps));
    }
    probe_summary.visible_output_total_chars = visible_output_total_chars;
    probe_summary.visible_output_total_tokens_est = visible_output_total_tokens_est;
    probe_summary.visible_output_message_count = message_rows.len();
    if visible_output_total_tokens_est > 0 {
        probe_summary.actionable_commentary_ratio_bps =
            Some((actionable_commentary_tokens * 10_000) / visible_output_total_tokens_est);
        probe_summary.tool_grounded_commentary_ratio_bps =
            Some((tool_grounded_commentary_tokens * 10_000) / visible_output_total_tokens_est);
        probe_summary.verification_grounded_commentary_ratio_bps = Some(
            (verification_grounded_commentary_tokens * 10_000) / visible_output_total_tokens_est,
        );
        probe_summary.restatement_ratio_bps =
            Some((restatement_tokens * 10_000) / visible_output_total_tokens_est);
        probe_summary.redundant_commentary_ratio_bps =
            Some((redundant_commentary_tokens * 10_000) / visible_output_total_tokens_est);
        probe_summary.speculation_ratio_bps =
            Some((speculation_tokens * 10_000) / visible_output_total_tokens_est);
        probe_summary.social_tone_ratio_bps =
            Some((social_tone_tokens * 10_000) / visible_output_total_tokens_est);
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
    write_jsonl(&attempt_dir.join("turn-metrics.jsonl"), &turn_rows)?;
    write_jsonl(&attempt_dir.join("message-metrics.jsonl"), &message_rows)?;
    write_jsonl(&attempt_dir.join("command-events.jsonl"), &command_rows)?;
    write_jsonl(&attempt_dir.join("tool-events.jsonl"), &tool_rows)?;
    write_jsonl(&attempt_dir.join("skill-events.jsonl"), &skill_rows)?;
    write_jsonl(&attempt_dir.join("patch-events.jsonl"), &patch_rows)?;
    write_jsonl(&attempt_dir.join("anomalies.jsonl"), &anomaly_rows)?;
    write_jsonl(
        &attempt_dir.join("verbosity-tool-coupling.jsonl"),
        &coupling_rows,
    )?;
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

    let run_manifest_meta = attempt_dir
        .parent()
        .map(|run_dir| run_dir.join("manifest.json"))
        .filter(|path| path.exists())
        .and_then(|path| read_json::<RunManifest>(&path).ok());

    let summary = RunSummary {
        instance_id: record.instance_id.clone(),
        repo: record.repo.clone(),
        task_class: task_class.to_string(),
        paired_instance_key: run_manifest_meta
            .as_ref()
            .map(|manifest| manifest.paired_instance_key.clone()),
        cohort_id: run_manifest_meta
            .as_ref()
            .map(|manifest| manifest.cohort_id.clone()),
        model: run_manifest_meta
            .as_ref()
            .map(|manifest| manifest.model.clone()),
        provider: run_manifest_meta
            .as_ref()
            .map(|manifest| manifest.provider.clone()),
        personality_mode: run_manifest_meta
            .as_ref()
            .and_then(|manifest| manifest.personality_mode.clone()),
        prompt_style: run_manifest_meta
            .as_ref()
            .and_then(|manifest| manifest.prompt_style.clone()),
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
        turn_count: turn_rows.len(),
        token_snapshot_count: token_rows.len(),
        command_count: command_rows.len(),
        tool_count: tool_rows.len(),
        skill_event_count: skill_rows.len(),
        message_metric_count: message_rows.len(),
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
        visible_output_total_chars,
        visible_output_total_tokens_est,
        visible_output_sentence_count,
        visible_output_paragraph_count,
        visible_output_bullet_count,
        visible_output_codeblock_count,
        visible_output_per_turn_tokens_est: div_i64(visible_output_total_tokens_est, turn_rows.len()),
        visible_output_per_tool_call_tokens_est: div_i64(visible_output_total_tokens_est, tool_rows.iter().filter(|row| row.get("phase").and_then(Value::as_str) == Some("begin")).count()),
        visible_output_per_patch_event_tokens_est: div_i64(visible_output_total_tokens_est, patch_rows.iter().filter(|row| row.get("phase").and_then(Value::as_str) == Some("end")).count()),
        visible_output_per_verification_event_tokens_est: div_i64(visible_output_total_tokens_est, probe_summary.verification_closure_count),
        event_type_counts,
        probe_code_counts,
        probe_subsystem_counts,
        diagnostic_type_counts,
        tool_kind_counts,
        tool_name_counts,
        skill_name_counts,
        message_category_counts,
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
        probe_summary.control_rod_compaction_count += 1;
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "ContextCompaction",
            "control_rod.compaction_regulation",
            "inferred",
            format!("compaction acted as a regulation layer via `{}`", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
    }
    if probe.code.contains("instruction") || subsystem == "InstructionChannel" {
        probe_summary.instruction_shift_count += 1;
        probe_summary.instruction_stratification_count += 1;
        probe_summary.externalized_coordination_count += 1;
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "InstructionChannel",
            "instruction.stratification",
            "inferred",
            format!("Codex exposed layered instruction behavior via `{}`", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
    }
    if probe.code.contains("config") || subsystem == "ConfigFreeze" {
        probe_summary.config_freeze_drift_count += 1;
        probe_summary.control_rod_config_freeze_count += 1;
    }
    if probe.code.contains("listener")
        || probe.code.contains("friction")
        || subsystem == "HarnessFriction"
    {
        probe_summary.harness_friction_count += 1;
        probe_summary.containment_heat_leak_count += 1;
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "ConfigFreeze",
            "control_rod.config_freeze",
            "inferred",
            format!(
                "session config froze model/runtime choices at `{}::{}`",
                payload
                    .get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                effective_model
            ),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
        if payload
            .get("persistExtendedHistory")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            probe_summary.externalized_coordination_count += 1;
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                seq,
                "PersistenceReconstruction",
                "persistence.externalized_state",
                "inferred",
                "Codex session was configured to persist extended history, suggesting externalized continuity beyond the immediate turn".to_string(),
                vec!["codex-probe-events.jsonl".to_string()],
            ));
        }
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
            probe_summary.persistence_staleness_risk_count += 1;
            derived_probes.push(make_probe(
                run_id,
                record,
                task_class,
                seq,
                "PersistenceReconstruction",
                "persistence.half_life",
                "estimated",
                format!(
                    "compaction compressed history from {before} items to {replacement}, raising a state half-life / rediscovery risk"
                ),
                vec!["codex-probe-events.jsonl".to_string()],
            ));
        }
    }
    if subsystem == "PersistenceReconstruction" {
        probe_summary.control_rod_persistence_count += 1;
        probe_summary.persistence_continuity_count += 1;
        probe_summary.externalized_coordination_count += 1;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "PersistenceReconstruction",
            "persistence.resume_path",
            classification.as_str(),
            format!("persistence or reconstruction event observed: {}", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "PersistenceReconstruction",
            "control_rod.persistence",
            "inferred",
            format!("persistence layer acted as a regulation/continuity mechanism via `{}`", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
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
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "HarnessFriction",
            "containment.heat_leak",
            classification.as_str(),
            format!("Codex harness leaked effort into orchestration via `{}`", probe.code),
            vec!["codex-probe-events.jsonl".to_string()],
        ));
    }
    if subsystem == "AppServerTranslation" {
        probe_summary.event_discontinuity_count += 1;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "AppServerTranslation",
            "events.discontinuity",
            classification.as_str(),
            format!("observer-visible event architecture discontinuity: `{}`", probe.code),
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
    let search_like = is_search_command(&command_text);
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

    if probe_summary.first_controlled_change_tokens.is_none()
        && search_like
        && !is_meaningful_edit_command(&command_text)
    {
        probe_summary.ignition_shell_search_count += 1;
        derived_probes.push(make_probe(
            run_id,
            record,
            task_class,
            seq,
            "TurnLifecycle",
            "fission.search_ignition",
            "inferred",
            format!("run appears to approach ignition through exploratory search command `{command_text}`"),
            vec!["command-events.jsonl".to_string()],
        ));
    }
    if probe_summary.first_meaningful_edit_tokens.is_none() && is_meaningful_edit_command(&command_text) {
        probe_summary.first_meaningful_edit_tokens = token_total;
        if probe_summary.first_controlled_change_tokens.is_none() {
            probe_summary.first_controlled_change_tokens = token_total;
        }
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
            "fission.first_controlled_change",
            "inferred",
            format!("first controlled code-change path appears to begin via `{command_text}`"),
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
        probe_summary.verification_closure_count += 1;
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
        if is_git_inspection_command(&command_text) || is_read_only_command(&command_text) {
            probe_summary.containment_heat_leak_count += 1;
        }
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
            label: if probe_summary.first_verification_tokens.is_some()
                && probe_summary.useful_step_proxy_num > 0
                && probe_summary.verification_closure_count > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!("first_verification_tokens={:?}", probe_summary.first_verification_tokens),
                format!("useful_step_proxy={}/{}", probe_summary.useful_step_proxy_num, probe_summary.useful_step_proxy_den),
                format!("verification_closure_count={}", probe_summary.verification_closure_count),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Closure is inferred from command chronology rather than semantic proof graphs.".to_string()],
        },
        ClaimEvidence {
            claim_id: "grounding.externalized_coordination".to_string(),
            label: if probe_summary.externalized_coordination_count > 0
                || probe_summary.persistence_continuity_count > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!(
                    "externalized_coordination_count={}",
                    probe_summary.externalized_coordination_count
                ),
                format!(
                    "persistence_continuity_count={}",
                    probe_summary.persistence_continuity_count
                ),
                format!(
                    "instruction_stratification_count={}",
                    probe_summary.instruction_stratification_count
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec![
                "This is evidence of Codex relying on layered/persistent mechanisms, not a direct test of a multi-agent scheduler.".to_string(),
            ],
        },
        ClaimEvidence {
            claim_id: "grounding.control_regulation".to_string(),
            label: if probe_summary.control_rod_compaction_count > 0
                || probe_summary.control_rod_config_freeze_count > 0
                || probe_summary.control_rod_persistence_count > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!(
                    "control_rod_compaction_count={}",
                    probe_summary.control_rod_compaction_count
                ),
                format!(
                    "control_rod_config_freeze_count={}",
                    probe_summary.control_rod_config_freeze_count
                ),
                format!(
                    "control_rod_persistence_count={}",
                    probe_summary.control_rod_persistence_count
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["A regulation mechanism can improve stability while also introducing some compression loss or overhead.".to_string()],
        },
        ClaimEvidence {
            claim_id: "grounding.state_verbalization".to_string(),
            label: if probe_summary.actionable_commentary_ratio_bps.unwrap_or_default() > 0
                || probe_summary.tool_grounded_commentary_ratio_bps.unwrap_or_default() > 0
                || probe_summary.verification_grounded_commentary_ratio_bps.unwrap_or_default() > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!(
                    "actionable_commentary_ratio_bps={:?}",
                    probe_summary.actionable_commentary_ratio_bps
                ),
                format!(
                    "tool_grounded_commentary_ratio_bps={:?}",
                    probe_summary.tool_grounded_commentary_ratio_bps
                ),
                format!(
                    "verification_grounded_commentary_ratio_bps={:?}",
                    probe_summary.verification_grounded_commentary_ratio_bps
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Visible commentary is only a proxy for deeper state externalization.".to_string()],
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
            claim_id: "codex.instruction_stratification".to_string(),
            label: if probe_summary.instruction_stratification_count > 0 {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![format!(
                "instruction_stratification_count={}",
                probe_summary.instruction_stratification_count
            )],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Instruction layering is inferred from probe events rather than from direct model-visible prompt dumps.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.persistence_continuity".to_string(),
            label: if probe_summary.persistence_continuity_count > 0 {
                "evidence_consistent"
            } else {
                "not_observable_with_current_probes"
            }
            .to_string(),
            supporting_evidence: vec![
                format!(
                    "persistence_continuity_count={}",
                    probe_summary.persistence_continuity_count
                ),
                format!(
                    "persistence_staleness_risk_count={}",
                    probe_summary.persistence_staleness_risk_count
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["Strong evidence usually requires resume, compaction, or reconstruction heavy runs.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.control_rods".to_string(),
            label: if probe_summary.control_rod_compaction_count > 0
                || probe_summary.control_rod_config_freeze_count > 0
                || probe_summary.control_rod_persistence_count > 0
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!(
                    "control_rod_compaction_count={}",
                    probe_summary.control_rod_compaction_count
                ),
                format!(
                    "control_rod_config_freeze_count={}",
                    probe_summary.control_rod_config_freeze_count
                ),
                format!(
                    "control_rod_persistence_count={}",
                    probe_summary.control_rod_persistence_count
                ),
                format!(
                    "containment_heat_leak_count={}",
                    probe_summary.containment_heat_leak_count
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["This claim interprets regulation behavior at the harness level; it is not meant as a statement about the base model alone.".to_string()],
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
            claim_id: "codex.event_architecture".to_string(),
            label: if probe_summary.event_discontinuity_count > 0 {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![format!(
                "event_discontinuity_count={}",
                probe_summary.event_discontinuity_count
            )],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref.clone()],
            caveats: vec!["A quiet run may not exercise listener lag or translation discontinuities.".to_string()],
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
                format!("containment_heat_leak_count={}", probe_summary.containment_heat_leak_count),
                format!("harness_overhead_proxy_bps={:?}", probe_summary.harness_overhead_proxy_bps),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![run_ref],
            caveats: vec!["Friction-token estimates are inferred from event sequencing and command duration.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.personality_policy_shape".to_string(),
            label: if probe_summary.visible_output_total_tokens_est > 0
                && (probe_summary.tool_grounded_commentary_ratio_bps.unwrap_or_default() > 0
                    || probe_summary.actionable_commentary_ratio_bps.unwrap_or_default() > 0)
            {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!("visible_output_total_tokens_est={}", probe_summary.visible_output_total_tokens_est),
                format!(
                    "tool_grounded_commentary_ratio_bps={:?}",
                    probe_summary.tool_grounded_commentary_ratio_bps
                ),
                format!("tool_burst_count={}", probe_summary.tool_burst_count),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![format!("{}[{task_class}]", record.instance_id)],
            caveats: vec!["Strong support requires paired cohort comparisons across the same tasks.".to_string()],
        },
        ClaimEvidence {
            claim_id: "codex.state_verbalization".to_string(),
            label: if probe_summary.actionable_commentary_ratio_bps.unwrap_or_default() > 0 {
                "evidence_consistent"
            } else {
                "evidence_inconclusive"
            }
            .to_string(),
            supporting_evidence: vec![
                format!(
                    "actionable_commentary_ratio_bps={:?}",
                    probe_summary.actionable_commentary_ratio_bps
                ),
                format!(
                    "restatement_ratio_bps={:?}",
                    probe_summary.restatement_ratio_bps
                ),
                format!(
                    "redundant_commentary_ratio_bps={:?}",
                    probe_summary.redundant_commentary_ratio_bps
                ),
            ],
            conflicting_evidence: Vec::new(),
            relevant_runs: vec![format!("{}[{task_class}]", record.instance_id)],
            caveats: vec!["A single run cannot by itself establish model-to-model differences.".to_string()],
        },
    ]
}

fn last_token_total(info: &Option<TokenUsageInfo>) -> Option<i64> {
    info.as_ref().map(|info| info.total_token_usage.total_tokens)
}

fn update_turn_token_state(acc: &mut TurnAccumulator, info: &TokenUsageInfo) {
    acc.end_input_tokens = info.total_token_usage.input_tokens;
    acc.end_output_tokens = info.total_token_usage.output_tokens;
    acc.end_cache_read_tokens = info.total_token_usage.cached_input_tokens;
    acc.end_total_tokens = info.total_token_usage.total_tokens;
    acc.model_context_window = info.model_context_window;
}

fn turn_row(acc: &TurnAccumulator) -> Value {
    json!({
        "turnId": acc.turn_id,
        "classification": "exact",
        "status": acc.status,
        "startSeq": acc.start_seq,
        "endSeq": acc.end_seq,
        "inputTokensStart": acc.start_input_tokens,
        "inputTokensEnd": acc.end_input_tokens,
        "inputTokensDelta": acc.end_input_tokens - acc.start_input_tokens,
        "outputTokensStart": acc.start_output_tokens,
        "outputTokensEnd": acc.end_output_tokens,
        "outputTokensDelta": acc.end_output_tokens - acc.start_output_tokens,
        "cacheReadTokensStart": acc.start_cache_read_tokens,
        "cacheReadTokensEnd": acc.end_cache_read_tokens,
        "cacheReadTokensDelta": acc.end_cache_read_tokens - acc.start_cache_read_tokens,
        "totalTokensStart": acc.start_total_tokens,
        "totalTokensEnd": acc.end_total_tokens,
        "totalTokensDelta": acc.end_total_tokens - acc.start_total_tokens,
        "modelContextWindow": acc.model_context_window,
        "tokenSnapshotCount": acc.token_snapshot_count,
        "commandCount": acc.command_count,
        "shellCommandCount": acc.shell_command_count,
        "mcpToolCount": acc.mcp_tool_count,
        "patchApplyCount": acc.patch_apply_count,
        "skillEventCount": acc.skill_event_count,
        "firstCommand": acc.first_command,
        "lastCommand": acc.last_command,
    })
}

fn duration_to_ms_i64(duration: Duration) -> i64 {
    duration.as_millis().try_into().unwrap_or(i64::MAX)
}

fn join_command(parts: &[String]) -> String {
    parts.join(" ")
}

fn detect_skills_from_command(command: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    for token in command.split_whitespace() {
        if token.ends_with("SKILL.md") {
            let normalized = token.trim_matches(|ch| ch == '"' || ch == '\'' || ch == '`');
            let path = Path::new(normalized);
            if let Some(parent) = path.parent()
                && let Some(name) = parent.file_name().and_then(|name| name.to_str())
            {
                names.insert(name.to_string());
            }
        }
    }
    names.into_iter().collect()
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

fn is_search_command(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    ["rg ", "grep ", "find ", "git grep", "ls ", "sed -n", "cat "]
        .iter()
        .any(|needle| command.starts_with(needle))
}

fn estimate_text_tokens(text: &str) -> i64 {
    let chars = text.chars().count() as i64;
    (chars / 4).max(1)
}

fn count_sentences(text: &str) -> usize {
    text.matches(['.', '!', '?', '。', '！', '？'])
        .count()
        .max(usize::from(!text.trim().is_empty()))
}

fn count_paragraphs(text: &str) -> usize {
    text.split("\n\n")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .count()
        .max(usize::from(!text.trim().is_empty()))
}

fn count_bullets(text: &str) -> usize {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("1. ")
                || trimmed.starts_with("2. ")
                || trimmed.starts_with("3. ")
        })
        .count()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lowered = text.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| lowered.contains(&needle.to_ascii_lowercase()))
}

fn classify_agent_message(message: &str, phase: &str) -> Vec<String> {
    let lowered = message.to_ascii_lowercase();
    let mut categories = Vec::new();
    if phase == "commentary" {
        categories.push("orientation".to_string());
    }
    if contains_any(&lowered, &["problem", "task", "issue", "need to", "要求", "问题", "任务"]) {
        categories.push("task_restatement".to_string());
    }
    if contains_any(&lowered, &["plan", "next", "first", "then", "接下来", "先", "然后"]) {
        categories.push("planning".to_string());
    }
    if contains_any(&lowered, &["found", "noticed", "see", "looks like", "发现", "看到", "看起来"]) {
        categories.push("observation".to_string());
    }
    if contains_any(&lowered, &["because", "so that", "i'm going to", "decide", "因为", "所以", "决定"]) {
        categories.push("decision_explanation".to_string());
    }
    if contains_any(&lowered, &["i'll run", "i'm going to run", "run ", "check ", "inspect ", "我去", "我会先", "先跑"]) {
        categories.push("tool_bridge_before".to_string());
    }
    if contains_any(&lowered, &["output shows", "that means", "the result", "结果", "说明", "表明"]) {
        categories.push("tool_bridge_after".to_string());
    }
    if contains_any(&lowered, &["verify", "test", "validated", "验证", "测试", "确认"]) {
        categories.push("verification_framing".to_string());
    }
    if phase == "finalanswer" || contains_any(&lowered, &["fixed", "resolved", "done", "修复", "解决", "完成"]) {
        categories.push("result_framing".to_string());
    }
    if contains_any(&lowered, &["we ", "let's", "happy to", "together", "我们", "一起", "我来"]) {
        categories.push("social_tone".to_string());
    }
    if contains_any(&lowered, &["again", "as mentioned", "recap", "再说一遍", "总结一下", "重申"]) {
        categories.push("redundant_recap".to_string());
    }
    if categories.is_empty() {
        categories.push("observation".to_string());
    }
    categories
}

fn classify_tool_burst(message_count: usize, token_count: i64) -> &'static str {
    if message_count == 0 || token_count == 0 {
        "silent_tool_burst"
    } else if token_count <= 80 {
        "micro_narrated_tool_burst"
    } else {
        "talk_then_act"
    }
}

fn div_i64(value: i64, denominator: usize) -> Option<i64> {
    if denominator == 0 {
        None
    } else {
        Some(value / denominator as i64)
    }
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
        EventMsg::AgentMessage(_) => "agent_message",
        EventMsg::ExecCommandBegin(_) => "exec_command_begin",
        EventMsg::ExecCommandEnd(_) => "exec_command_end",
        EventMsg::McpToolCallBegin(_) => "mcp_tool_call_begin",
        EventMsg::McpToolCallEnd(_) => "mcp_tool_call_end",
        EventMsg::PatchApplyBegin(_) => "patch_apply_begin",
        EventMsg::PatchApplyEnd(_) => "patch_apply_end",
        EventMsg::ListSkillsResponse(_) => "list_skills_response",
        EventMsg::ListRemoteSkillsResponse(_) => "list_remote_skills_response",
        EventMsg::StudyProbe(_) => "study_probe",
        EventMsg::Warning(_) => "warning",
        EventMsg::Error(_) => "error",
        EventMsg::StreamError(_) => "stream_error",
        _ => "other",
    }
}
