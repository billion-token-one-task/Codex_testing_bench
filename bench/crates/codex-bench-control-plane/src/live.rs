use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::index::{RunIndexEntry, read_jsonl_file};
use crate::processes::UiEvent;

pub type LiveSnapshotMap = Arc<RwLock<BTreeMap<String, LiveRunSnapshot>>>;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LiveRunProgress {
    pub current_phase: String,
    pub turn_count: usize,
    pub message_count: usize,
    pub command_count: usize,
    pub tool_count: usize,
    pub patch_event_count: usize,
    pub verification_event_count: usize,
    pub raw_event_count: usize,
    pub artifact_row_count: usize,
    pub stalled: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LiveRunTelemetry {
    pub total_tokens: Option<i64>,
    pub visible_output_total_tokens_est: i64,
    pub tokens_per_minute: f64,
    pub messages_per_minute: f64,
    pub commands_per_minute: f64,
    pub tool_bursts_per_minute: f64,
    pub visible_tokens_per_tool_call: f64,
    pub visible_tokens_per_message: f64,
    pub tool_calls_per_message: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LiveRunMechanismSnapshot {
    pub personality_requested: Option<String>,
    pub personality_effective: Option<String>,
    pub personality_fallback_count: usize,
    pub personality_model_messages_preserved: Option<bool>,
    pub instruction_layers_active: Vec<String>,
    pub compaction_count: usize,
    pub harness_friction_count: usize,
    pub skill_inferred_count: usize,
    pub active_skill_names: Vec<String>,
    pub last_message_category: Option<String>,
    pub top_tool_route: Option<String>,
    pub latest_mechanism_event: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LiveRunSnapshot {
    pub campaign_id: String,
    pub run_id: String,
    pub instance_id: String,
    pub repo: String,
    pub cohort_id: String,
    pub model: String,
    pub provider: String,
    pub personality_mode: Option<String>,
    pub task_class: String,
    pub run_status: String,
    pub grading_status: String,
    pub started_at: Option<String>,
    pub last_event_at: Option<String>,
    pub elapsed_ms: Option<i64>,
    pub activity_heat: String,
    pub current_focus: Option<String>,
    pub warnings: Vec<String>,
    pub progress: LiveRunProgress,
    pub telemetry: LiveRunTelemetry,
    pub latest_message_preview: Option<String>,
    pub latest_tool: Option<String>,
    pub latest_patch: Option<String>,
    pub latest_command: Option<String>,
    pub mechanism: LiveRunMechanismSnapshot,
}

#[derive(Debug, Clone, Default)]
pub struct FileCursorState {
    line_count: usize,
}

pub fn build_live_run_snapshot(run: &RunIndexEntry) -> Option<LiveRunSnapshot> {
    if run.status != "running" {
        return None;
    }

    let attempt = run.latest_attempt.as_ref()?;
    let attempt_dir = Path::new(&attempt.directory);
    let message_rows = maybe_jsonl(&attempt_dir.join("message-metrics.jsonl"));
    let tool_rows = maybe_jsonl(&attempt_dir.join("tool-events.jsonl"));
    let command_rows = maybe_jsonl(&attempt_dir.join("command-events.jsonl"));
    let patch_rows = maybe_jsonl(&attempt_dir.join("patch-events.jsonl"));
    let token_rows = maybe_jsonl(&attempt_dir.join("token-snapshots.jsonl"));
    let probe_rows = maybe_jsonl(&attempt_dir.join("codex-probe-events.jsonl"));
    let skill_rows = maybe_jsonl(&attempt_dir.join("skill-events.jsonl"));
    let raw_rows = maybe_jsonl(&attempt_dir.join("raw-agent-events.jsonl"));

    let message_count = if !message_rows.is_empty() {
        message_rows.len()
    } else {
        count_raw_types(&raw_rows, &["agent_message"])
    };
    let command_count = if !command_rows.is_empty() {
        count_begins(&command_rows)
    } else {
        count_raw_types(&raw_rows, &["exec_command_begin"])
    };
    let tool_count = if !tool_rows.is_empty() {
        count_tool_begin_rows(&tool_rows)
    } else {
        count_raw_types(
            &raw_rows,
            &["exec_command_begin", "mcp_tool_call_begin", "patch_apply_begin", "view_image_tool_call"],
        )
    };
    let patch_event_count = if !patch_rows.is_empty() {
        patch_rows.len()
    } else {
        count_raw_types(&raw_rows, &["patch_apply_begin", "patch_apply_end", "turn_diff"])
    };
    let verification_event_count = run.verification_closure_count;
    let latest_message_preview = latest_message_preview(&message_rows, &raw_rows);
    let latest_tool = latest_tool_summary(&tool_rows, &raw_rows);
    let latest_patch = latest_patch_summary(&patch_rows, &raw_rows);
    let latest_command = latest_command_summary(&command_rows, &raw_rows);
    let current_phase = infer_current_phase(&latest_patch, &latest_tool, &latest_message_preview, &patch_rows, &tool_rows);

    let started_at = attempt_started_at(attempt_dir);
    let last_event_at = latest_artifact_timestamp(attempt_dir);
    let elapsed_ms = started_at
        .as_deref()
        .and_then(parse_rfc3339)
        .map(|started| (Utc::now() - started).num_milliseconds())
        .filter(|value| *value >= 0);
    let minutes = elapsed_ms
        .map(|ms| (ms as f64 / 60_000.0).max(1.0 / 60.0))
        .unwrap_or(1.0);

    let total_tokens = latest_i64(&token_rows, &["totalTokens", "total_tokens"]).or(run.total_tokens);
    let visible_output_total_tokens_est = sum_i64(&message_rows, &["textTokensEst", "text_tokens_est"])
        .unwrap_or(run.visible_output_total_tokens_est);
    let tool_burst_count = count_micro_bursts(&attempt_dir.join("verbosity-tool-coupling.jsonl"))
        .unwrap_or_else(|| tool_count);
    let raw_event_count = raw_rows.len();
    let artifact_row_count = message_rows.len()
        + tool_rows.len()
        + command_rows.len()
        + patch_rows.len()
        + token_rows.len()
        + probe_rows.len()
        + skill_rows.len();
    let stalled = elapsed_ms
        .zip(last_event_at.as_deref().and_then(parse_rfc3339))
        .map(|(_, last)| (Utc::now() - last).num_seconds() >= 90)
        .unwrap_or(false);

    let mechanism = build_mechanism_snapshot(run, &probe_rows, &skill_rows, &message_rows, &tool_rows);
    let activity_heat = infer_activity_heat(message_count, command_count, tool_count, patch_event_count, minutes, stalled);
    let current_focus = infer_current_focus(
        &latest_patch,
        &latest_tool,
        &latest_command,
        &latest_message_preview,
        &mechanism,
    );
    let warnings = build_warnings(run, &mechanism, total_tokens, stalled, message_count, tool_count, command_count);

    Some(LiveRunSnapshot {
        campaign_id: run.campaign_id.clone(),
        run_id: run.run_id.clone(),
        instance_id: run.instance_id.clone(),
        repo: run.repo.clone(),
        cohort_id: run.cohort_id.clone(),
        model: run.model.clone(),
        provider: run.provider.clone(),
        personality_mode: run.personality_mode.clone(),
        task_class: run.task_class.clone(),
        run_status: run.status.clone(),
        grading_status: run.grading_status.clone(),
        started_at,
        last_event_at,
        elapsed_ms,
        activity_heat,
        current_focus,
        warnings,
        progress: LiveRunProgress {
            current_phase,
            turn_count: token_rows.len().max(1),
            message_count,
            command_count,
            tool_count,
            patch_event_count,
            verification_event_count,
            raw_event_count,
            artifact_row_count,
            stalled,
        },
        telemetry: LiveRunTelemetry {
            total_tokens,
            visible_output_total_tokens_est,
            tokens_per_minute: total_tokens.map(|value| value as f64 / minutes).unwrap_or(0.0),
            messages_per_minute: message_count as f64 / minutes,
            commands_per_minute: command_count as f64 / minutes,
            tool_bursts_per_minute: tool_burst_count as f64 / minutes,
            visible_tokens_per_tool_call: if tool_count == 0 {
                0.0
            } else {
                visible_output_total_tokens_est as f64 / tool_count as f64
            },
            visible_tokens_per_message: if message_count == 0 {
                0.0
            } else {
                visible_output_total_tokens_est as f64 / message_count as f64
            },
            tool_calls_per_message: if message_count == 0 {
                tool_count as f64
            } else {
                tool_count as f64 / message_count as f64
            },
        },
        latest_message_preview,
        latest_tool,
        latest_patch,
        latest_command,
        mechanism,
    })
}

pub fn append_jsonl_rows_since(
    path: &Path,
    cursor: &mut FileCursorState,
) -> anyhow::Result<Vec<Value>> {
    if !path.exists() {
        cursor.line_count = 0;
        return Ok(Vec::new());
    }
    let rows = read_jsonl_file(path)?;
    if rows.len() < cursor.line_count {
        cursor.line_count = 0;
    }
    let appended = rows.into_iter().skip(cursor.line_count).collect::<Vec<_>>();
    cursor.line_count += appended.len();
    Ok(appended)
}

pub fn live_events_from_artifact_row(
    run: &RunIndexEntry,
    attempt: u32,
    artifact_key: &str,
    row: &Value,
) -> Vec<UiEvent> {
    let base = base_payload(run, attempt, artifact_key, row.clone());
    match artifact_key {
        "messageMetrics" => vec![
            ui_event("run.message.appended", base.clone()),
            ui_event("run.timeline.appended", base),
        ],
        "toolEvents" => vec![
            ui_event("run.tool.appended", base.clone()),
            ui_event("run.timeline.appended", base),
        ],
        "patchEvents" | "patchChain" => vec![
            ui_event("run.patch.appended", base.clone()),
            ui_event("run.timeline.appended", base),
        ],
        "commandEvents" => vec![
            ui_event("run.command.appended", base.clone()),
            ui_event("run.timeline.appended", base),
        ],
        "personalityEvents" => vec![
            ui_event("run.personality.appended", base.clone()),
            ui_event("run.mechanism.appended", base.clone()),
            ui_event("run.timeline.appended", base),
        ],
        "skillEvents" | "skillMechanism" => vec![
            ui_event("run.skill.appended", base.clone()),
            ui_event("run.mechanism.appended", base.clone()),
            ui_event("run.timeline.appended", base),
        ],
        "tokenSnapshots" => vec![ui_event("run.token.appended", base)],
        "codexProbeEvents" | "anomalies" | "lifecycleEvents" => vec![
            ui_event("run.mechanism.appended", base.clone()),
            ui_event("run.timeline.appended", base),
        ],
        _ => vec![ui_event("artifact.updated", base)],
    }
}

pub fn live_events_from_raw_agent_event(run: &RunIndexEntry, attempt: u32, row: &Value) -> Vec<UiEvent> {
    let msg = row
        .get("params")
        .and_then(|value| value.get("msg"))
        .cloned()
        .unwrap_or(Value::Null);
    let event_type = msg
        .get("type")
        .and_then(Value::as_str)
        .or_else(|| row.get("method").and_then(Value::as_str))
        .unwrap_or("raw_event");
    let payload = serde_json::json!({
        "runId": run.run_id,
        "campaignId": run.campaign_id,
        "cohortId": run.cohort_id,
        "instanceId": run.instance_id,
        "attempt": attempt,
        "artifact": "rawAgentEvents",
        "source": "raw_agent_events",
        "rawEventType": event_type,
        "row": row,
        "messagePreview": raw_message_preview(&msg),
    });
    match event_type {
        "agent_message" | "agent_message_delta" | "agent_message_content_delta" => vec![
            ui_event("run.message.appended", payload.clone()),
            ui_event("run.timeline.appended", payload),
        ],
        "exec_command_begin" | "exec_command_end" => vec![
            ui_event("run.command.appended", payload.clone()),
            ui_event("run.tool.appended", payload.clone()),
            ui_event("run.timeline.appended", payload),
        ],
        "mcp_tool_call_begin" | "mcp_tool_call_end" | "view_image_tool_call" | "dynamic_tool_call_request" | "dynamic_tool_call_response" => vec![
            ui_event("run.tool.appended", payload.clone()),
            ui_event("run.timeline.appended", payload),
        ],
        "patch_apply_begin" | "patch_apply_end" | "turn_diff" => vec![
            ui_event("run.patch.appended", payload.clone()),
            ui_event("run.timeline.appended", payload),
        ],
        "study_probe" | "warning" | "context_compacted" | "thread_rolled_back" | "model_reroute" | "stream_error" => vec![
            ui_event("run.mechanism.appended", payload.clone()),
            ui_event("run.timeline.appended", payload),
        ],
        "token_count" => vec![ui_event("run.token.appended", payload)],
        _ => vec![ui_event("run.timeline.appended", payload)],
    }
}

fn maybe_jsonl(path: &Path) -> Vec<Value> {
    if !path.exists() {
        return Vec::new();
    }
    read_jsonl_file(path).unwrap_or_default()
}

fn latest_message_preview(message_rows: &[Value], raw_rows: &[Value]) -> Option<String> {
    message_rows
        .last()
        .and_then(|row| {
            row.get("textPreview")
                .and_then(Value::as_str)
                .or_else(|| row.get("message").and_then(Value::as_str))
                .map(trim_string)
        })
        .or_else(|| {
            raw_rows.iter().rev().find_map(|row| {
                let msg = row.get("params")?.get("msg")?;
                let event_type = msg.get("type")?.as_str()?;
                if event_type != "agent_message" {
                    return None;
                }
                msg.get("message").and_then(Value::as_str).map(trim_string)
            })
        })
}

fn latest_tool_summary(tool_rows: &[Value], raw_rows: &[Value]) -> Option<String> {
    tool_rows
        .iter()
        .rev()
        .find_map(|row| {
            if row.get("phase").and_then(Value::as_str) == Some("begin") {
                Some(format!(
                    "{}:{}",
                    row.get("kind").and_then(Value::as_str).unwrap_or("tool"),
                    row.get("name").and_then(Value::as_str).unwrap_or("unknown")
                ))
            } else {
                None
            }
        })
        .or_else(|| {
            raw_rows.iter().rev().find_map(|row| {
                let msg = row.get("params")?.get("msg")?;
                match msg.get("type")?.as_str()? {
                    "exec_command_begin" => Some("shell:exec_command".to_string()),
                    "mcp_tool_call_begin" => Some(format!(
                        "mcp:{}",
                        msg.get("tool").and_then(Value::as_str).unwrap_or("tool")
                    )),
                    "patch_apply_begin" => Some("patch:apply_patch".to_string()),
                    "view_image_tool_call" => Some("view_image:view_image".to_string()),
                    _ => None,
                }
            })
        })
}

fn latest_patch_summary(patch_rows: &[Value], raw_rows: &[Value]) -> Option<String> {
    patch_rows
        .iter()
        .rev()
        .find_map(|row| row.get("event").and_then(Value::as_str).map(str::to_string))
        .or_else(|| {
            raw_rows.iter().rev().find_map(|row| {
                let msg = row.get("params")?.get("msg")?;
                match msg.get("type")?.as_str()? {
                    "patch_apply_begin" => Some("patch_apply_begin".to_string()),
                    "patch_apply_end" => Some("patch_apply_end".to_string()),
                    "turn_diff" => Some("turn_diff".to_string()),
                    _ => None,
                }
            })
        })
}

fn latest_command_summary(command_rows: &[Value], raw_rows: &[Value]) -> Option<String> {
    command_rows
        .iter()
        .rev()
        .find_map(|row| {
            row.get("command")
                .and_then(Value::as_str)
                .map(trim_string)
                .or_else(|| row.get("title").and_then(Value::as_str).map(trim_string))
        })
        .or_else(|| {
            raw_rows.iter().rev().find_map(|row| {
                let msg = row.get("params")?.get("msg")?;
                if msg.get("type")?.as_str()? != "exec_command_begin" {
                    return None;
                }
                msg.get("command")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .as_deref()
                    .map(trim_string)
            })
        })
}

fn infer_current_phase(
    latest_patch: &Option<String>,
    latest_tool: &Option<String>,
    latest_message: &Option<String>,
    patch_rows: &[Value],
    tool_rows: &[Value],
) -> String {
    if latest_patch.is_some() || !patch_rows.is_empty() {
        return "patching".to_string();
    }
    if latest_tool.is_some() || !tool_rows.is_empty() {
        return "tooling".to_string();
    }
    if latest_message.is_some() {
        return "commentary".to_string();
    }
    "starting".to_string()
}

fn build_mechanism_snapshot(
    run: &RunIndexEntry,
    probe_rows: &[Value],
    skill_rows: &[Value],
    message_rows: &[Value],
    tool_rows: &[Value],
) -> LiveRunMechanismSnapshot {
    let latest_mechanism_event = probe_rows
        .iter()
        .rev()
        .find_map(|row| row.get("code").and_then(Value::as_str).map(str::to_string));
    let personality_effective = probe_rows.iter().find_map(|row| {
        row.get("payload")
            .and_then(|payload| payload.get("personality"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let personality_model_messages_preserved = probe_rows.iter().find_map(|row| {
        row.get("payload")
            .and_then(|payload| payload.get("modelNativeInstructionsPreserved"))
            .and_then(Value::as_bool)
    });
    let compaction_count = probe_rows
        .iter()
        .filter(|row| {
            row.get("subsystem").and_then(Value::as_str) == Some("context_compaction")
                || row.get("code").and_then(Value::as_str).map(|code| code.contains("compact")).unwrap_or(false)
        })
        .count();
    let skill_inferred_count = skill_rows.len();
    let active_skill_names = top_unique_strings(skill_rows, &["skillName", "skill_name", "name"], 4);
    let last_message_category = message_rows
        .iter()
        .rev()
        .find_map(|row| {
            row.get("primaryCategory")
                .and_then(Value::as_str)
                .or_else(|| row.get("category").and_then(Value::as_str))
                .map(str::to_string)
        })
        .or_else(|| {
            probe_rows
                .iter()
                .rev()
                .find_map(|row| row.get("lastMessageCategory").and_then(Value::as_str).map(str::to_string))
        });
    let top_tool_route = most_common_string(tool_rows, &["toolRoute", "tool_route", "route"])
        .or_else(|| {
            probe_rows
                .iter()
                .rev()
                .find_map(|row| row.get("topToolRoute").and_then(Value::as_str).map(str::to_string))
        });
    let mut layers = BTreeSet::new();
    for row in probe_rows {
        if let Some(payload) = row.get("payload") {
            if payload.get("hasBaseInstructions").and_then(Value::as_bool) == Some(true) {
                layers.insert("base".to_string());
            }
            if payload.get("hasDeveloperInstructions").and_then(Value::as_bool) == Some(true) {
                layers.insert("developer".to_string());
            }
            if payload.get("hasUserInstructions").and_then(Value::as_bool) == Some(true) {
                layers.insert("user".to_string());
            }
            if payload.get("modelNativeInstructionsPreserved").and_then(Value::as_bool) == Some(true) {
                layers.insert("model_native".to_string());
            }
        }
    }

    LiveRunMechanismSnapshot {
        personality_requested: run.personality_mode.clone(),
        personality_effective,
        personality_fallback_count: run.personality_fallback_count,
        personality_model_messages_preserved,
        instruction_layers_active: layers.into_iter().collect(),
        compaction_count,
        harness_friction_count: run.harness_friction_count,
        skill_inferred_count,
        active_skill_names,
        last_message_category,
        top_tool_route,
        latest_mechanism_event,
    }
}

fn infer_activity_heat(
    message_count: usize,
    command_count: usize,
    tool_count: usize,
    patch_event_count: usize,
    minutes: f64,
    stalled: bool,
) -> String {
    if stalled {
        return "stalled".to_string();
    }
    let activity_rate = (message_count + command_count + tool_count + patch_event_count) as f64 / minutes.max(0.1);
    if activity_rate >= 10.0 {
        "hot".to_string()
    } else if activity_rate >= 4.0 {
        "warm".to_string()
    } else if activity_rate >= 1.0 {
        "steady".to_string()
    } else {
        "cold".to_string()
    }
}

fn infer_current_focus(
    latest_patch: &Option<String>,
    latest_tool: &Option<String>,
    latest_command: &Option<String>,
    latest_message_preview: &Option<String>,
    mechanism: &LiveRunMechanismSnapshot,
) -> Option<String> {
    if let Some(patch) = latest_patch {
        return Some(format!("patch:{patch}"));
    }
    if let Some(tool) = latest_tool {
        return Some(format!("tool:{tool}"));
    }
    if let Some(command) = latest_command {
        return Some(format!("cmd:{command}"));
    }
    if let Some(category) = &mechanism.last_message_category {
        return Some(format!("message:{category}"));
    }
    latest_message_preview.clone()
}

fn build_warnings(
    run: &RunIndexEntry,
    mechanism: &LiveRunMechanismSnapshot,
    total_tokens: Option<i64>,
    stalled: bool,
    message_count: usize,
    tool_count: usize,
    command_count: usize,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if run.personality_fallback_count > 0 {
        warnings.push(format!("personality fallback ×{}", run.personality_fallback_count));
    }
    if run.harness_friction_count > 0 {
        warnings.push(format!("harness friction ×{}", run.harness_friction_count));
    }
    if stalled {
        warnings.push("run appears stalled".to_string());
    }
    if tool_count == 0 && command_count == 0 && message_count > 0 {
        warnings.push("commentary without tool activity".to_string());
    }
    if mechanism.compaction_count > 0 {
        warnings.push(format!("compaction observed ×{}", mechanism.compaction_count));
    }
    if total_tokens.unwrap_or_default() >= 100_000 {
        warnings.push("high token pressure".to_string());
    }
    warnings
}

fn top_unique_strings(rows: &[Value], keys: &[&str], limit: usize) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for row in rows.iter().rev() {
        if let Some(value) = keys
            .iter()
            .find_map(|key| row.get(*key).and_then(Value::as_str))
        {
            if seen.insert(value.to_string()) {
                values.push(value.to_string());
            }
        }
        if values.len() >= limit {
            break;
        }
    }
    values
}

fn most_common_string(rows: &[Value], keys: &[&str]) -> Option<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for row in rows {
        if let Some(value) = keys
            .iter()
            .find_map(|key| row.get(*key).and_then(Value::as_str))
        {
            *counts.entry(value.to_string()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)))
        .map(|(value, _)| value)
}

fn count_begins(rows: &[Value]) -> usize {
    rows.iter()
        .filter(|row| row.get("phase").and_then(Value::as_str) == Some("begin"))
        .count()
}

fn count_tool_begin_rows(rows: &[Value]) -> usize {
    rows.iter()
        .filter(|row| row.get("phase").and_then(Value::as_str) == Some("begin"))
        .count()
}

fn count_raw_types(rows: &[Value], types: &[&str]) -> usize {
    rows.iter()
        .filter(|row| {
            let msg = row.get("params").and_then(|value| value.get("msg"));
            let event_type = msg.and_then(|value| value.get("type")).and_then(Value::as_str);
            event_type.map(|value| types.contains(&value)).unwrap_or(false)
        })
        .count()
}

fn latest_i64(rows: &[Value], keys: &[&str]) -> Option<i64> {
    rows.iter().rev().find_map(|row| {
        keys.iter()
            .find_map(|key| row.get(*key).and_then(Value::as_i64))
    })
}

fn sum_i64(rows: &[Value], keys: &[&str]) -> Option<i64> {
    let mut found = false;
    let mut sum = 0_i64;
    for row in rows {
        if let Some(value) = keys.iter().find_map(|key| row.get(*key).and_then(Value::as_i64)) {
            found = true;
            sum += value;
        }
    }
    found.then_some(sum)
}

fn count_micro_bursts(path: &Path) -> Option<usize> {
    let rows = maybe_jsonl(path);
    if rows.is_empty() {
        return None;
    }
    Some(rows.len())
}

fn base_payload(run: &RunIndexEntry, attempt: u32, artifact_key: &str, row: Value) -> Value {
    serde_json::json!({
        "runId": run.run_id,
        "campaignId": run.campaign_id,
        "cohortId": run.cohort_id,
        "instanceId": run.instance_id,
        "attempt": attempt,
        "artifact": artifact_key,
        "source": "normalized",
        "row": row,
    })
}

fn ui_event(event_type: &str, payload: Value) -> UiEvent {
    UiEvent {
        event_type: event_type.to_string(),
        payload,
    }
}

fn raw_message_preview(msg: &Value) -> Option<String> {
    msg.get("message")
        .and_then(Value::as_str)
        .or_else(|| msg.get("delta").and_then(Value::as_str))
        .map(trim_string)
}

fn trim_string(value: &str) -> String {
    const LIMIT: usize = 180;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    let head = value.chars().take(LIMIT).collect::<String>();
    format!("{head}…")
}

fn attempt_started_at(attempt_dir: &Path) -> Option<String> {
    fs::metadata(attempt_dir)
        .ok()
        .and_then(|meta| meta.created().ok().or_else(|| meta.modified().ok()))
        .map(|time| DateTime::<Utc>::from(time).to_rfc3339())
}

fn latest_artifact_timestamp(attempt_dir: &Path) -> Option<String> {
    let mut latest: Option<std::time::SystemTime> = None;
    for entry in fs::read_dir(attempt_dir).ok()? {
        let entry = entry.ok()?;
        let modified = entry.metadata().ok()?.modified().ok()?;
        latest = Some(latest.map(|current| current.max(modified)).unwrap_or(modified));
    }
    latest.map(|time| DateTime::<Utc>::from(time).to_rfc3339())
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn jsonl_append_cursor_only_returns_new_rows() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rows.jsonl");
        fs::write(&path, "{\"a\":1}\n{\"a\":2}\n").unwrap();
        let mut cursor = FileCursorState::default();
        let first = append_jsonl_rows_since(&path, &mut cursor).unwrap();
        assert_eq!(first.len(), 2);
        fs::write(&path, "{\"a\":1}\n{\"a\":2}\n{\"a\":3}\n").unwrap();
        let second = append_jsonl_rows_since(&path, &mut cursor).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].get("a").and_then(Value::as_i64), Some(3));
    }

    #[test]
    fn raw_exec_command_becomes_command_and_tool_event() {
        let run = RunIndexEntry {
            campaign_id: "c".into(),
            run_id: "r".into(),
            manifest_run_id: "m".into(),
            instance_id: "i".into(),
            repo: "repo".into(),
            task_class: "search-heavy".into(),
            cohort_id: "cohort".into(),
            model: "gpt-5.4".into(),
            provider: "openai".into(),
            personality_mode: Some("friendly".into()),
            prompt_style: None,
            status: "running".into(),
            grading_status: "grader_not_run".into(),
            run_dir: "".into(),
            manifest_path: "".into(),
            latest_updated_at: None,
            command_count: 0,
            tool_count: 0,
            patch_file_count: 0,
            message_metric_count: 0,
            visible_output_total_tokens_est: 0,
            total_tokens: None,
            anomaly_count: 0,
            tool_kind_counts: BTreeMap::new(),
            tool_name_counts: BTreeMap::new(),
            tool_route_counts: BTreeMap::new(),
            message_category_counts: BTreeMap::new(),
            ignition_shell_search_count: 0,
            verification_closure_count: 0,
            personality_fallback_count: 0,
            harness_friction_count: 0,
            latest_attempt: None,
        };
        let row = serde_json::json!({
            "params": {
                "msg": {
                    "type": "exec_command_begin",
                    "command": ["/bin/zsh", "-lc", "rg foo"]
                }
            }
        });
        let events = live_events_from_raw_agent_event(&run, 1, &row);
        let names = events.into_iter().map(|event| event.event_type).collect::<Vec<_>>();
        assert!(names.contains(&"run.command.appended".to_string()));
        assert!(names.contains(&"run.tool.appended".to_string()));
        assert!(names.contains(&"run.timeline.appended".to_string()));
    }

    #[test]
    fn live_snapshot_uses_attempt_files_for_progress_and_mechanism() {
        let dir = tempdir().unwrap();
        let attempt_dir = dir.path().join("attempt-01");
        fs::create_dir_all(&attempt_dir).unwrap();
        fs::write(
            attempt_dir.join("message-metrics.jsonl"),
            "{\"message\":\"Investigating the bug now.\",\"textTokensEst\":14,\"textPreview\":\"Investigating the bug now.\",\"primaryCategory\":\"planning\"}\n",
        )
        .unwrap();
        fs::write(
            attempt_dir.join("tool-events.jsonl"),
            "{\"phase\":\"begin\",\"kind\":\"shell\",\"name\":\"shell\",\"toolRoute\":\"exec_command\"}\n",
        )
        .unwrap();
        fs::write(
            attempt_dir.join("command-events.jsonl"),
            "{\"phase\":\"begin\",\"command\":\"rg -n foo\"}\n",
        )
        .unwrap();
        fs::write(
            attempt_dir.join("token-snapshots.jsonl"),
            "{\"totalTokens\":1200}\n",
        )
        .unwrap();
        fs::write(
            attempt_dir.join("codex-probe-events.jsonl"),
            "{\"subsystem\":\"instruction_channel\",\"code\":\"turn_context_built\",\"payload\":{\"personality\":\"friendly\",\"modelNativeInstructionsPreserved\":true,\"hasBaseInstructions\":true,\"hasUserInstructions\":true}}\n",
        )
        .unwrap();
        let run = RunIndexEntry {
            campaign_id: "campaign".into(),
            run_id: "run".into(),
            manifest_run_id: "manifest".into(),
            instance_id: "instance".into(),
            repo: "repo".into(),
            task_class: "verification-heavy".into(),
            cohort_id: "cohort".into(),
            model: "gpt-5.4".into(),
            provider: "openai".into(),
            personality_mode: Some("friendly".into()),
            prompt_style: None,
            status: "running".into(),
            grading_status: "grader_not_run".into(),
            run_dir: dir.path().display().to_string(),
            manifest_path: dir.path().join("manifest.json").display().to_string(),
            latest_updated_at: None,
            command_count: 0,
            tool_count: 0,
            patch_file_count: 0,
            message_metric_count: 0,
            visible_output_total_tokens_est: 0,
            total_tokens: None,
            anomaly_count: 0,
            tool_kind_counts: BTreeMap::new(),
            tool_name_counts: BTreeMap::new(),
            tool_route_counts: BTreeMap::new(),
            message_category_counts: BTreeMap::new(),
            ignition_shell_search_count: 0,
            verification_closure_count: 2,
            personality_fallback_count: 0,
            harness_friction_count: 0,
            latest_attempt: Some(crate::index::AttemptIndex {
                attempt: 1,
                directory: attempt_dir.display().to_string(),
                artifacts: Vec::new(),
            }),
        };

        let snapshot = build_live_run_snapshot(&run).expect("live snapshot");
        assert_eq!(snapshot.progress.message_count, 1);
        assert_eq!(snapshot.progress.command_count, 1);
        assert_eq!(snapshot.progress.tool_count, 1);
        assert_eq!(snapshot.telemetry.total_tokens, Some(1200));
        assert_eq!(snapshot.mechanism.personality_effective.as_deref(), Some("friendly"));
        assert!(snapshot
            .mechanism
            .instruction_layers_active
            .contains(&"base".to_string()));
    }
}
