use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::{get, post}};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast};
use tokio::time::sleep;

use codex_bench_core::{CampaignManifest, read_json};
use crate::index::{
    ArtifactDescriptor, CampaignDetail, CampaignIndexEntry, RunIndexEntry, WorkspaceIndex, read_csv_file, read_jsonl_file,
    read_text_file, scan_campaign_detail, scan_workspace,
};
use crate::live::{
    FileCursorState, LiveRunSnapshot, LiveSnapshotMap, append_jsonl_rows_since,
    build_live_run_snapshot, live_events_from_artifact_row, live_events_from_raw_agent_event,
};
use crate::processes::{ManagedProcessDetail, ProcessRegistry, UiEvent};

#[derive(Clone)]
pub struct AppState {
    pub repo_root: PathBuf,
    pub processes: ProcessRegistry,
    pub events: broadcast::Sender<UiEvent>,
    pub live_runs: LiveSnapshotMap,
    pub workspace_cache: std::sync::Arc<RwLock<Option<WorkspaceIndex>>>,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactFileQuery {
    pub path: String,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactTailQuery {
    pub path: String,
    #[serde(default = "default_tail_lines")]
    pub lines: usize,
}

#[derive(Debug, Deserialize)]
pub struct RunStreamQuery {
    #[serde(default)]
    pub event_types: Option<String>,
}

fn default_tail_lines() -> usize {
    120
}

#[derive(Debug, Deserialize)]
pub struct CampaignActionRequest {
    #[serde(default)]
    pub campaign_dir: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub refresh_repo_cache: Option<bool>,
    #[serde(default)]
    pub release: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PrepareActionRequest {
    pub campaign_root: String,
    #[serde(default)]
    pub preset_path: Option<String>,
    #[serde(default)]
    pub sample_size: Option<usize>,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub personality: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
    #[serde(default)]
    pub experiment_name: Option<String>,
    #[serde(default)]
    pub max_parallel_runs: Option<usize>,
    #[serde(default)]
    pub per_repo_prepare_parallelism: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct StopActionRequest {
    pub process_id: String,
}

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub process_id: String,
    pub kind: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ArtifactTailResponse {
    pub path: String,
    pub lines: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineRow {
    pub lane: String,
    pub kind: String,
    pub timestamp: Option<String>,
    pub title: String,
    pub summary: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RunDetailResponse {
    pub run: RunIndexEntry,
    pub campaign: Option<crate::index::CampaignIndexEntry>,
    pub reports: Vec<ArtifactDescriptor>,
    pub datasets: Vec<ArtifactDescriptor>,
    pub attempt_artifacts: Vec<ArtifactDescriptor>,
    pub run_summary: Option<serde_json::Value>,
    pub probe_summary: Option<serde_json::Value>,
    pub timeline: Vec<TimelineRow>,
    pub tables: BTreeMap<String, Vec<serde_json::Value>>,
    pub previews: BTreeMap<String, String>,
    pub live_snapshot: Option<LiveRunSnapshot>,
}

#[derive(Debug, Serialize)]
pub struct RunOperationalSummary {
    pub run: RunIndexEntry,
    pub live_snapshot: Option<LiveRunSnapshot>,
    pub latest_reports: Vec<ArtifactDescriptor>,
    pub latest_datasets: Vec<ArtifactDescriptor>,
    pub attempt_artifact_count: usize,
    pub artifact_type_counts: BTreeMap<String, usize>,
    pub event_table_counts: BTreeMap<String, usize>,
    pub current_phase: Option<String>,
    pub latest_focus: Option<String>,
    pub latest_message_preview: Option<String>,
    pub latest_tool: Option<String>,
    pub latest_patch: Option<String>,
    pub latest_command: Option<String>,
    pub live_warning_count: usize,
    pub operational_warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CampaignOperationalSummary {
    pub campaign: CampaignIndexEntry,
    pub active_live_runs: Vec<LiveRunSnapshot>,
    pub latest_reports: Vec<ArtifactDescriptor>,
    pub latest_datasets: Vec<ArtifactDescriptor>,
    pub active_process_count: usize,
    pub latest_activity_at: Option<String>,
    pub live_visible_output_total_tokens_est: i64,
    pub live_total_tokens: i64,
    pub live_message_count: usize,
    pub live_command_count: usize,
    pub live_tool_count: usize,
    pub live_patch_event_count: usize,
    pub solver_status_counts: BTreeMap<String, usize>,
    pub grading_status_counts: BTreeMap<String, usize>,
    pub cohort_counts: BTreeMap<String, usize>,
    pub task_class_counts: BTreeMap<String, usize>,
    pub model_counts: BTreeMap<String, usize>,
    pub personality_counts: BTreeMap<String, usize>,
    pub tool_route_counts: BTreeMap<String, usize>,
    pub tool_name_counts: BTreeMap<String, usize>,
    pub active_cohorts: Vec<String>,
    pub active_instances: Vec<String>,
    pub unresolved_infra_failure_count: usize,
    pub active_warning_count: usize,
    pub stalled_live_run_count: usize,
    pub personality_fallback_live_count: usize,
    pub heat_counts: BTreeMap<String, usize>,
    pub focus_samples: Vec<String>,
    pub latest_message_previews: Vec<String>,
    pub operational_warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LiveProcessDossier {
    pub snapshot: ManagedProcessDetail,
    pub kind_group: String,
}

#[derive(Debug, Serialize)]
pub struct LiveOverviewResponse {
    pub workspace: WorkspaceIndex,
    pub active_campaign: Option<CampaignIndexEntry>,
    pub active_campaign_summary: Option<CampaignOperationalSummary>,
    pub active_live_runs: Vec<LiveRunSnapshot>,
    pub current_campaign_live_runs: Vec<LiveRunSnapshot>,
    pub other_live_runs: Vec<LiveRunSnapshot>,
    pub hottest_live_runs: Vec<LiveRunSnapshot>,
    pub stalled_live_runs: Vec<LiveRunSnapshot>,
    pub running_processes: Vec<crate::processes::ManagedProcessSnapshot>,
    pub process_dossiers: Vec<LiveProcessDossier>,
    pub active_process_count: usize,
    pub latest_process_output_at: Option<String>,
    pub latest_global_focus_samples: Vec<String>,
    pub latest_global_message_previews: Vec<String>,
    pub latest_global_warnings: Vec<String>,
    pub operator_notices: Vec<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/workspace/index", get(get_workspace_index))
        .route("/api/live/overview", get(get_live_overview))
        .route("/api/campaigns", get(list_campaigns))
        .route("/api/campaigns/{campaign_id}", get(get_campaign))
        .route("/api/campaigns/{campaign_id}/operational-summary", get(get_campaign_operational_summary))
        .route("/api/campaigns/{campaign_id}/reports", get(get_campaign_reports))
        .route("/api/campaigns/{campaign_id}/datasets", get(get_campaign_datasets))
        .route("/api/runs/{run_id}", get(get_run))
        .route("/api/runs/{run_id}/operational-summary", get(get_run_operational_summary))
        .route("/api/runs/{run_id}/detail", get(get_run_detail))
        .route("/api/runs/{run_id}/stream", get(stream_run_events))
        .route("/api/runs/{run_id}/attempts/{attempt}", get(get_run_attempt))
        .route("/api/processes", get(list_processes))
        .route("/api/processes/{process_id}", get(get_process_detail))
        .route("/api/live/runs", get(list_live_runs))
        .route("/api/live/runs/{run_id}", get(get_live_run))
        .route("/api/events", get(stream_events))
        .route("/api/artifacts/file", get(get_artifact_file))
        .route("/api/artifacts/tail", get(get_artifact_tail))
        .route("/api/actions/prepare", post(post_prepare))
        .route("/api/actions/bootstrap-local", post(post_bootstrap_local))
        .route("/api/actions/warm-cache", post(post_warm_cache))
        .route("/api/actions/run", post(post_run))
        .route("/api/actions/grade", post(post_grade))
        .route("/api/actions/report", post(post_report))
        .route("/api/actions/replay", post(post_replay))
        .route("/api/actions/inspect-codex", post(post_inspect_codex))
        .route("/api/actions/stop", post(post_stop))
        .with_state(state)
}

async fn get_workspace_index(State(state): State<AppState>) -> ApiResult<Json<WorkspaceIndex>> {
    Ok(Json(workspace_index_or_cached(&state).await?))
}

async fn get_live_overview(State(state): State<AppState>) -> ApiResult<Json<LiveOverviewResponse>> {
    let workspace = workspace_index_or_cached(&state).await?;
    let mut active_live_runs = state
        .live_runs
        .read()
        .await
        .values()
        .cloned()
        .collect::<Vec<_>>();
    active_live_runs.sort_by(|left, right| {
        right
            .last_event_at
            .cmp(&left.last_event_at)
            .then_with(|| left.instance_id.cmp(&right.instance_id))
    });
    let active_campaign = choose_active_campaign(&workspace, &active_live_runs);
    let active_campaign_summary = if let Some(campaign) = &active_campaign {
        Some(build_campaign_operational_summary(&state, &workspace, campaign.clone()).await?)
    } else {
        None
    };
    let current_campaign_live_runs = if let Some(campaign) = &active_campaign {
        active_live_runs
            .iter()
            .filter(|run| run.campaign_id == campaign.campaign_id)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let other_live_runs = if let Some(campaign) = &active_campaign {
        active_live_runs
            .iter()
            .filter(|run| run.campaign_id != campaign.campaign_id)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        active_live_runs.clone()
    };
    let mut hottest_live_runs = active_live_runs.clone();
    hottest_live_runs.sort_by(|left, right| {
        let left_score = live_heat_score(left);
        let right_score = live_heat_score(right);
        right_score
            .cmp(&left_score)
            .then_with(|| right.last_event_at.cmp(&left.last_event_at))
    });
    hottest_live_runs.truncate(6);
    let stalled_live_runs = active_live_runs
        .iter()
        .filter(|run| run.progress.stalled || run.activity_heat == "stalled")
        .cloned()
        .collect::<Vec<_>>();
    let running_processes = state
        .processes
        .list()
        .await
        .into_iter()
        .filter(|process| process.status == "running")
        .collect::<Vec<_>>();
    let latest_process_output_at = running_processes
        .iter()
        .filter_map(|process| process.last_output_at.clone())
        .max();
    let mut process_dossiers = Vec::new();
    for process in running_processes.iter().take(4) {
        if let Ok(detail) = state.processes.detail(&process.id).await {
            process_dossiers.push(LiveProcessDossier {
                kind_group: process.kind.clone(),
                snapshot: detail,
            });
        }
    }
    let latest_global_focus_samples = top_unique_strings(
        active_live_runs
            .iter()
            .filter_map(|run| run.current_focus.clone()),
        8,
    );
    let latest_global_message_previews = top_unique_strings(
        active_live_runs
            .iter()
            .filter_map(|run| run.latest_message_preview.clone()),
        10,
    );
    let latest_global_warnings = top_unique_strings(
        active_live_runs
            .iter()
            .flat_map(|run| run.warnings.iter().cloned()),
        10,
    );
    let mut operator_notices = Vec::new();
    if !current_campaign_live_runs.is_empty() && running_processes.is_empty() {
        operator_notices.push(
            "当前 live run 有活跃项，但 control plane 没有对应受管进程；这通常表示这些 run 由外部 launcher 或旧会话启动。".to_string(),
        );
    }
    if current_campaign_live_runs.is_empty() && !active_live_runs.is_empty() {
        operator_notices.push(
            "当前选中的 active campaign 没有 live run，页面正在展示跨 campaign 的历史 live/stalled 现场。".to_string(),
        );
    }
    if hottest_live_runs.is_empty() && !active_live_runs.is_empty() {
        operator_notices.push("有 live run，但热度排行仍为空；请检查 live snapshot 是否缺少最新 token/tool/message 行。".to_string());
    }

    Ok(Json(LiveOverviewResponse {
        workspace,
        active_campaign,
        active_campaign_summary,
        active_live_runs,
        current_campaign_live_runs,
        other_live_runs,
        hottest_live_runs,
        stalled_live_runs,
        active_process_count: running_processes.len(),
        running_processes,
        process_dossiers,
        latest_process_output_at,
        latest_global_focus_samples,
        latest_global_message_previews,
        latest_global_warnings,
        operator_notices,
    }))
}

fn choose_active_campaign(
    workspace: &WorkspaceIndex,
    active_live_runs: &[LiveRunSnapshot],
) -> Option<CampaignIndexEntry> {
    if !active_live_runs.is_empty() {
        let mut freshest_by_campaign = BTreeMap::<String, String>::new();
        for run in active_live_runs {
            let key = run.last_event_at.clone().or_else(|| run.started_at.clone()).unwrap_or_default();
            let entry = freshest_by_campaign.entry(run.campaign_id.clone()).or_default();
            if key > *entry {
                *entry = key;
            }
        }
        if let Some((campaign_id, _)) = freshest_by_campaign
            .into_iter()
            .max_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)))
        {
            if let Some(campaign) = workspace
                .campaigns
                .iter()
                .find(|campaign| campaign.campaign_id == campaign_id)
                .cloned()
            {
                return Some(campaign);
            }
        }
    }

    workspace
        .campaigns
        .iter()
        .filter(|campaign| campaign.status == "running" || campaign.active_run_count > 0)
        .max_by(|left, right| left.created_at.cmp(&right.created_at).then_with(|| left.campaign_id.cmp(&right.campaign_id)))
        .cloned()
        .or_else(|| {
            workspace
                .campaigns
                .iter()
                .max_by(|left, right| left.created_at.cmp(&right.created_at).then_with(|| left.campaign_id.cmp(&right.campaign_id)))
                .cloned()
        })
}

async fn list_campaigns(State(state): State<AppState>) -> ApiResult<Json<Vec<crate::index::CampaignIndexEntry>>> {
    let index = workspace_index_or_cached(&state).await?;
    Ok(Json(index.campaigns))
}

async fn get_campaign(
    State(state): State<AppState>,
    AxumPath(campaign_id): AxumPath<String>,
) -> ApiResult<Json<CampaignDetail>> {
    let campaign_dir = campaign_dir_for_id(&state.repo_root, &campaign_id)?;
    Ok(Json(scan_campaign_detail(&state.repo_root, &campaign_dir)?))
}

async fn get_campaign_operational_summary(
    State(state): State<AppState>,
    AxumPath(campaign_id): AxumPath<String>,
) -> ApiResult<Json<CampaignOperationalSummary>> {
    let index = workspace_index_or_cached(&state).await?;
    let campaign = index
        .campaigns
        .iter()
        .find(|campaign| campaign.campaign_id == campaign_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("campaign not found"))?;
    Ok(Json(build_campaign_operational_summary(&state, &index, campaign).await?))
}

async fn get_campaign_reports(
    State(state): State<AppState>,
    AxumPath(campaign_id): AxumPath<String>,
) -> ApiResult<Json<Vec<ArtifactDescriptor>>> {
    let campaign_dir = campaign_dir_for_id(&state.repo_root, &campaign_id)?;
    Ok(Json(scan_campaign_detail(&state.repo_root, &campaign_dir)?.reports))
}

async fn get_campaign_datasets(
    State(state): State<AppState>,
    AxumPath(campaign_id): AxumPath<String>,
) -> ApiResult<Json<Vec<ArtifactDescriptor>>> {
    let campaign_dir = campaign_dir_for_id(&state.repo_root, &campaign_id)?;
    Ok(Json(scan_campaign_detail(&state.repo_root, &campaign_dir)?.datasets))
}

async fn get_run(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> ApiResult<Json<crate::index::RunIndexEntry>> {
    let index = workspace_index_or_cached(&state).await?;
    let run = index
        .runs
        .into_iter()
        .find(|run| run.run_id == run_id)
        .ok_or_else(|| anyhow::anyhow!("run not found"))?;
    Ok(Json(run))
}

async fn get_run_detail(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> ApiResult<Json<RunDetailResponse>> {
    let index = workspace_index_or_cached(&state).await?;
    let run = index
        .runs
        .iter()
        .find(|run| run.run_id == run_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("run not found"))?;
    let campaign = index
        .campaigns
        .iter()
        .find(|campaign| campaign.campaign_id == run.campaign_id)
        .cloned();
    let campaign_dir = campaign_dir_for_id(&state.repo_root, &run.campaign_id)?;
    let campaign_detail = scan_campaign_detail(&state.repo_root, &campaign_dir)?;

    let attempt = run
        .latest_attempt
        .clone()
        .ok_or_else(|| anyhow::anyhow!("attempt not found"))?;
    let attempt_dir = PathBuf::from(&attempt.directory);
    let run_summary = maybe_json(&attempt_dir.join("run-summary.json"));
    let probe_summary = maybe_json(&attempt_dir.join("probe-summary.json"));

    let table_specs = [
        ("turnMetrics", "turn-metrics.jsonl"),
        ("messageMetrics", "message-metrics.jsonl"),
        ("toolEvents", "tool-events.jsonl"),
        ("commandEvents", "command-events.jsonl"),
        ("patchChain", "patch-chain.jsonl"),
        ("patchEvents", "patch-events.jsonl"),
        ("personalityEvents", "personality-events.jsonl"),
        ("skillMechanism", "skill-mechanism.jsonl"),
        ("verbosityToolCoupling", "verbosity-tool-coupling.jsonl"),
    ];
    let mut tables = BTreeMap::new();
    for (key, name) in table_specs {
        let path = attempt_dir.join(name);
        if path.exists() {
            tables.insert(key.to_string(), read_jsonl_file(&path)?);
        }
    }

    let preview_specs = [
        ("attemptLog", "attempt-log.txt"),
        ("runEvidence", "run-evidence.txt"),
        ("patchDiff", "patch.diff"),
    ];
    let mut previews = BTreeMap::new();
    for (key, name) in preview_specs {
        let path = attempt_dir.join(name);
        if path.exists() {
            let text = read_text_file(&path)?;
            previews.insert(key.to_string(), trim_preview(&text, 20_000));
        }
    }

    let timeline = build_timeline(&tables);

    Ok(Json(RunDetailResponse {
        run,
        campaign,
        reports: campaign_detail.reports,
        datasets: campaign_detail.datasets,
        attempt_artifacts: attempt.artifacts,
        run_summary,
        probe_summary,
        timeline,
        tables,
        previews,
        live_snapshot: state.live_runs.read().await.get(&run_id).cloned(),
    }))
}

async fn get_run_operational_summary(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> ApiResult<Json<RunOperationalSummary>> {
    let index = workspace_index_or_cached(&state).await?;
    let run = index
        .runs
        .iter()
        .find(|run| run.run_id == run_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("run not found"))?;
    let campaign_dir = campaign_dir_for_id(&state.repo_root, &run.campaign_id)?;
    let campaign_detail = scan_campaign_detail(&state.repo_root, &campaign_dir)?;
    let live_snapshot = state.live_runs.read().await.get(&run_id).cloned();

    let mut latest_reports = campaign_detail.reports;
    latest_reports.sort_by(|left, right| right.updated_at.cmp(&left.updated_at).then_with(|| left.name.cmp(&right.name)));
    latest_reports.truncate(6);
    let mut latest_datasets = campaign_detail.datasets;
    latest_datasets.sort_by(|left, right| right.updated_at.cmp(&left.updated_at).then_with(|| left.name.cmp(&right.name)));
    latest_datasets.truncate(6);

    let attempt_artifacts = run
        .latest_attempt
        .as_ref()
        .map(|attempt| attempt.artifacts.clone())
        .unwrap_or_default();
    let mut artifact_type_counts = BTreeMap::<String, usize>::new();
    for artifact in &attempt_artifacts {
        *artifact_type_counts.entry(artifact.format.clone()).or_insert(0) += 1;
    }

    let mut event_table_counts = BTreeMap::<String, usize>::new();
    if let Some(attempt) = &run.latest_attempt {
        let attempt_dir = PathBuf::from(&attempt.directory);
        for (key, name) in [
            ("messageMetrics", "message-metrics.jsonl"),
            ("toolEvents", "tool-events.jsonl"),
            ("commandEvents", "command-events.jsonl"),
            ("patchChain", "patch-chain.jsonl"),
            ("personalityEvents", "personality-events.jsonl"),
            ("skillMechanism", "skill-mechanism.jsonl"),
            ("verbosityToolCoupling", "verbosity-tool-coupling.jsonl"),
            ("rawAgentEvents", "raw-agent-events.jsonl"),
            ("codexProbeEvents", "codex-probe-events.jsonl"),
        ] {
            let path = attempt_dir.join(name);
            if path.exists() {
                event_table_counts.insert(key.to_string(), read_jsonl_file(&path)?.len());
            }
        }
    }

    let mut operational_warnings = Vec::new();
    if run.personality_fallback_count > 0 {
        operational_warnings.push(format!("personality fallback ×{}", run.personality_fallback_count));
    }
    if run.harness_friction_count > 0 {
        operational_warnings.push(format!("harness friction ×{}", run.harness_friction_count));
    }
    if run.grading_status.contains("failed") || run.grading_status.contains("error") {
        operational_warnings.push(format!("grading status: {}", run.grading_status));
    }
    if let Some(snapshot) = &live_snapshot {
        operational_warnings.extend(snapshot.warnings.clone());
    }
    operational_warnings.sort();
    operational_warnings.dedup();

    Ok(Json(RunOperationalSummary {
        run,
        live_snapshot: live_snapshot.clone(),
        latest_reports,
        latest_datasets,
        attempt_artifact_count: attempt_artifacts.len(),
        artifact_type_counts,
        event_table_counts,
        current_phase: live_snapshot.as_ref().map(|snapshot| snapshot.progress.current_phase.clone()),
        latest_focus: live_snapshot.as_ref().and_then(|snapshot| snapshot.current_focus.clone()),
        latest_message_preview: live_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.latest_message_preview.clone()),
        latest_tool: live_snapshot.as_ref().and_then(|snapshot| snapshot.latest_tool.clone()),
        latest_patch: live_snapshot.as_ref().and_then(|snapshot| snapshot.latest_patch.clone()),
        latest_command: live_snapshot.as_ref().and_then(|snapshot| snapshot.latest_command.clone()),
        live_warning_count: live_snapshot.as_ref().map(|snapshot| snapshot.warnings.len()).unwrap_or(0),
        operational_warnings,
    }))
}

async fn get_run_attempt(
    State(state): State<AppState>,
    AxumPath((run_id, attempt)): AxumPath<(String, u32)>,
) -> ApiResult<Json<serde_json::Value>> {
    let index = state
        .workspace_cache
        .read()
        .await
        .clone()
        .unwrap_or(scan_workspace(&state.repo_root)?);
    let run = index
        .runs
        .into_iter()
        .find(|run| run.run_id == run_id)
        .ok_or_else(|| anyhow::anyhow!("run not found"))?;
    let latest = run
        .latest_attempt
        .ok_or_else(|| anyhow::anyhow!("attempt not found"))?;
    if latest.attempt != attempt {
        return Err(ApiError::from(anyhow::anyhow!("attempt not indexed")));
    }
    Ok(Json(serde_json::to_value(latest)?))
}

async fn list_processes(State(state): State<AppState>) -> ApiResult<Json<Vec<crate::processes::ManagedProcessSnapshot>>> {
    Ok(Json(state.processes.list().await))
}

async fn get_process_detail(
    State(state): State<AppState>,
    AxumPath(process_id): AxumPath<String>,
) -> ApiResult<Json<ManagedProcessDetail>> {
    Ok(Json(state.processes.detail(&process_id).await?))
}

async fn list_live_runs(State(state): State<AppState>) -> ApiResult<Json<Vec<LiveRunSnapshot>>> {
    let snapshots = state.live_runs.read().await;
    let mut rows = snapshots.values().cloned().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .last_event_at
            .cmp(&left.last_event_at)
            .then_with(|| left.instance_id.cmp(&right.instance_id))
    });
    Ok(Json(rows))
}

async fn get_live_run(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
) -> ApiResult<Json<LiveRunSnapshot>> {
    let snapshots = state.live_runs.read().await;
    let snapshot = snapshots
        .get(&run_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("live run not found"))?;
    Ok(Json(snapshot))
}

async fn stream_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.events.subscribe();
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let payload = serde_json::to_string(&event).unwrap_or_else(|_| "{\"type\":\"error\",\"payload\":{}}".to_string());
                    yield Ok(Event::default().event(event.event_type.clone()).data(payload));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("system").data("{\"type\":\"system\",\"payload\":{\"message\":\"event stream lagged\"}}"));
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(5)).text("keep-alive"))
}

async fn stream_run_events(
    State(state): State<AppState>,
    AxumPath(run_id): AxumPath<String>,
    Query(query): Query<RunStreamQuery>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let requested_types = query
        .event_types
        .as_deref()
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut rx = state.events.subscribe();
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if !requested_types.is_empty() && !requested_types.contains(&event.event_type) {
                        continue;
                    }
                    if !event_matches_run_id(&event, &run_id) {
                        continue;
                    }
                    let payload = serde_json::to_string(&event).unwrap_or_else(|_| "{\"type\":\"error\",\"payload\":{}}".to_string());
                    yield Ok(Event::default().event(event.event_type.clone()).data(payload));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("system.warning").data("{\"type\":\"system.warning\",\"payload\":{\"message\":\"run stream lagged\"}}"));
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(5)).text("keep-alive"))
}

async fn get_artifact_file(
    State(state): State<AppState>,
    Query(query): Query<ArtifactFileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let path = safe_path(&state.repo_root, &PathBuf::from(&query.path))?;
    let format = query.format.unwrap_or_else(|| detect_format(&path));
    let payload = match format.as_str() {
        "jsonl" => serde_json::json!({ "kind": "jsonl", "rows": read_jsonl_file(&path)? }),
        "csv" => serde_json::json!({ "kind": "csv", "rows": read_csv_file(&path)? }),
        _ => serde_json::json!({ "kind": "text", "content": read_text_file(&path)? }),
    };
    Ok(Json(serde_json::json!({
        "path": path.display().to_string(),
        "format": format,
        "payload": payload,
    })))
}

async fn get_artifact_tail(
    State(state): State<AppState>,
    Query(query): Query<ArtifactTailQuery>,
) -> ApiResult<Json<ArtifactTailResponse>> {
    let path = safe_path(&state.repo_root, &PathBuf::from(&query.path))?;
    let content = read_text_file(&path)?;
    let mut lines = content.lines().map(|line| line.to_string()).collect::<Vec<_>>();
    let truncated = lines.len() > query.lines;
    if truncated {
        lines = lines.split_off(lines.len().saturating_sub(query.lines));
    }
    Ok(Json(ArtifactTailResponse {
        path: path.display().to_string(),
        lines,
        truncated,
    }))
}

async fn post_prepare(
    State(state): State<AppState>,
    Json(body): Json<PrepareActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let mut args = vec!["prepare".to_string(), "--campaign-root".to_string(), body.campaign_root];
    if let Some(preset_path) = body.preset_path { args.extend(["--preset-path".to_string(), preset_path]); }
    if let Some(sample_size) = body.sample_size { args.extend(["--sample-size".to_string(), sample_size.to_string()]); }
    if let Some(seed) = body.seed { args.extend(["--seed".to_string(), seed]); }
    if let Some(stage) = body.stage { args.extend(["--stage".to_string(), stage]); }
    if let Some(model) = body.model { args.extend(["--model".to_string(), model]); }
    if let Some(provider) = body.provider { args.extend(["--provider".to_string(), provider]); }
    if let Some(personality) = body.personality { args.extend(["--personality".to_string(), personality]); }
    if let Some(prompt_style) = body.prompt_style { args.extend(["--prompt-style".to_string(), prompt_style]); }
    if let Some(experiment_name) = body.experiment_name { args.extend(["--experiment-name".to_string(), experiment_name]); }
    if let Some(max_parallel_runs) = body.max_parallel_runs { args.extend(["--max-parallel-runs".to_string(), max_parallel_runs.to_string()]); }
    if let Some(parallelism) = body.per_repo_prepare_parallelism { args.extend(["--per-repo-prepare-parallelism".to_string(), parallelism.to_string()]); }
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "prepare", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_bootstrap_local(
    State(state): State<AppState>,
    Json(body): Json<CampaignActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let mut args = vec!["bootstrap-local".to_string()];
    if let Some(campaign_dir) = body.campaign_dir { args.extend(["--campaign-dir".to_string(), campaign_dir]); }
    if body.refresh_repo_cache.unwrap_or(false) { args.push("--refresh-repo-cache".to_string()); }
    if body.release.unwrap_or(false) { args.push("--release".to_string()); }
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "bootstrap-local", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_warm_cache(
    State(state): State<AppState>,
    Json(body): Json<CampaignActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let campaign_dir = body.campaign_dir.context("campaign_dir is required")?;
    let mut args = vec!["warm-cache".to_string(), campaign_dir];
    if body.refresh_repo_cache.unwrap_or(false) { args.push("--refresh-repo-cache".to_string()); }
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "warm-cache", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_run(
    State(state): State<AppState>,
    Json(body): Json<CampaignActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let campaign_dir = body.campaign_dir.context("campaign_dir is required")?;
    let mut args = vec!["run".to_string(), campaign_dir];
    if body.refresh_repo_cache.unwrap_or(false) { args.push("--refresh-repo-cache".to_string()); }
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "run", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_grade(
    State(state): State<AppState>,
    Json(body): Json<CampaignActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let campaign_dir = body.campaign_dir.context("campaign_dir is required")?;
    let mut args = vec!["grade".to_string(), campaign_dir];
    if let Some(command) = body.command { args.extend(["--command".to_string(), command]); }
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "grade", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_report(
    State(state): State<AppState>,
    Json(body): Json<CampaignActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let campaign_dir = body.campaign_dir.context("campaign_dir is required")?;
    let args = vec!["report".to_string(), campaign_dir];
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "report", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_replay(
    State(state): State<AppState>,
    Json(body): Json<CampaignActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let run_dir = body.campaign_dir.context("run/campaign dir is required")?;
    let args = vec!["replay".to_string(), run_dir];
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "replay", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_inspect_codex(
    State(state): State<AppState>,
    Json(body): Json<CampaignActionRequest>,
) -> ApiResult<Json<ActionResponse>> {
    let campaign_dir = body.campaign_dir.context("campaign_dir is required")?;
    let args = vec!["inspect-codex".to_string(), campaign_dir];
    let snapshot = state.processes.spawn_cli_process(&state.repo_root, "inspect-codex", &args).await?;
    Ok(Json(ActionResponse { process_id: snapshot.id, kind: snapshot.kind, status: snapshot.status }))
}

async fn post_stop(
    State(state): State<AppState>,
    Json(body): Json<StopActionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    state.processes.stop(&body.process_id).await?;
    Ok(Json(serde_json::json!({ "status": "stopping", "processId": body.process_id })))
}

pub async fn poll_workspace(state: AppState) {
    let mut last_snapshot = String::new();
    let mut last_campaign_snapshots = BTreeMap::<String, String>::new();
    let mut last_run_snapshots = BTreeMap::<String, String>::new();
    let mut last_live_run_snapshots = BTreeMap::<String, LiveRunSnapshot>::new();
    let mut last_campaign_summary_snapshots = BTreeMap::<String, String>::new();
    let mut last_artifact_snapshots = BTreeMap::<String, String>::new();
    let mut artifact_cursors = BTreeMap::<String, FileCursorState>::new();
    loop {
        if let Ok(index) = scan_workspace(&state.repo_root) {
            *state.workspace_cache.write().await = Some(index.clone());

            for campaign in &index.campaigns {
                if let Ok(serialized_campaign) = serde_json::to_string(campaign) {
                    let changed = last_campaign_snapshots
                        .get(&campaign.campaign_id)
                        .map(|previous| previous != &serialized_campaign)
                        .unwrap_or(true);
                    if changed {
                        last_campaign_snapshots
                            .insert(campaign.campaign_id.clone(), serialized_campaign);
                        let _ = state.events.send(UiEvent {
                            event_type: "campaign.updated".to_string(),
                            payload: serde_json::to_value(campaign)
                                .unwrap_or_else(|_| serde_json::json!({})),
                        });
                    }
                }
            }

            for run in &index.runs {
                if let Ok(serialized_run) = serde_json::to_string(run) {
                    let changed = last_run_snapshots
                        .get(&run.run_id)
                        .map(|previous| previous != &serialized_run)
                        .unwrap_or(true);
                    if changed {
                        last_run_snapshots.insert(run.run_id.clone(), serialized_run);
                        let _ = state.events.send(UiEvent {
                            event_type: "run.updated".to_string(),
                            payload: serde_json::to_value(run).unwrap_or_else(|_| serde_json::json!({})),
                        });
                    }
                }
            }
            emit_artifact_descriptor_updates(&state, &index, &mut last_artifact_snapshots);
            emit_live_run_updates(&state, &index, &mut last_live_run_snapshots).await;
            emit_campaign_summary_updates(&state, &index, &mut last_campaign_summary_snapshots).await;
            emit_live_artifact_appends(&state, &index, &mut artifact_cursors);
            if let Ok(serialized) = serde_json::to_string(&index) {
                if serialized != last_snapshot {
                    last_snapshot = serialized;
                    let _ = state.events.send(UiEvent {
                        event_type: "workspace.updated".to_string(),
                        payload: serde_json::to_value(index).unwrap_or_else(|_| serde_json::json!({})),
                    });
                }
            }
        }
        sleep(Duration::from_secs(2)).await;
    }
}

fn emit_artifact_descriptor_updates(
    state: &AppState,
    index: &WorkspaceIndex,
    last_artifact_snapshots: &mut BTreeMap<String, String>,
) {
    let mut current_paths = BTreeMap::<String, ArtifactDescriptor>::new();
    for campaign in &index.campaigns {
        if let Ok(detail) = scan_campaign_detail(&state.repo_root, &PathBuf::from(&campaign.path)) {
            for artifact in detail.reports.into_iter().chain(detail.datasets.into_iter()) {
                current_paths.insert(artifact.path.clone(), artifact);
            }
            for run in detail.runs {
                if let Some(attempt) = run.latest_attempt {
                    for artifact in attempt.artifacts {
                        current_paths.insert(artifact.path.clone(), artifact);
                    }
                }
            }
        }
    }
    for (path, descriptor) in current_paths {
        if let Ok(serialized) = serde_json::to_string(&descriptor) {
            let changed = last_artifact_snapshots
                .get(&path)
                .map(|previous| previous != &serialized)
                .unwrap_or(true);
            if changed {
                last_artifact_snapshots.insert(path.clone(), serialized);
                let payload = serde_json::to_value(&descriptor)
                    .unwrap_or_else(|_| serde_json::json!({ "path": path }));
                let _ = state.events.send(UiEvent {
                    event_type: "artifact.updated".to_string(),
                    payload: payload.clone(),
                });
                if descriptor.scope.starts_with("campaign_") {
                    let _ = state.events.send(UiEvent {
                        event_type: "campaign.artifact.updated".to_string(),
                        payload,
                    });
                }
            }
        }
    }
}

async fn emit_live_run_updates(
    state: &AppState,
    index: &WorkspaceIndex,
    last_live_run_snapshots: &mut BTreeMap<String, LiveRunSnapshot>,
) {
    let mut current = BTreeMap::<String, LiveRunSnapshot>::new();
    for run in &index.runs {
        if let Some(snapshot) = build_live_run_snapshot(run) {
            let previous = last_live_run_snapshots.get(&run.run_id).cloned();
            let changed = previous.as_ref().map(|prev| prev != &snapshot).unwrap_or(true);
            if changed {
                let _ = state.events.send(UiEvent {
                    event_type: "run.live.updated".to_string(),
                    payload: serde_json::to_value(&snapshot)
                        .unwrap_or_else(|_| serde_json::json!({})),
                });
                let _ = state.events.send(UiEvent {
                    event_type: "run.summary.updated".to_string(),
                    payload: serde_json::json!({
                        "campaignId": snapshot.campaign_id,
                        "runId": snapshot.run_id,
                        "instanceId": snapshot.instance_id,
                        "cohortId": snapshot.cohort_id,
                        "currentPhase": snapshot.progress.current_phase,
                        "activityHeat": snapshot.activity_heat,
                        "currentFocus": snapshot.current_focus,
                        "warningCount": snapshot.warnings.len(),
                        "messageCount": snapshot.progress.message_count,
                        "toolCount": snapshot.progress.tool_count,
                        "commandCount": snapshot.progress.command_count,
                        "patchEventCount": snapshot.progress.patch_event_count,
                        "totalTokens": snapshot.telemetry.total_tokens,
                        "visibleOutputTotalTokensEst": snapshot.telemetry.visible_output_total_tokens_est,
                        "latestMessagePreview": snapshot.latest_message_preview,
                        "latestTool": snapshot.latest_tool,
                        "latestPatch": snapshot.latest_patch,
                        "latestCommand": snapshot.latest_command,
                        "lastEventAt": snapshot.last_event_at,
                    }),
                });
                if previous
                    .as_ref()
                    .map(|prev| prev.progress.current_phase != snapshot.progress.current_phase)
                    .unwrap_or(true)
                {
                    let _ = state.events.send(UiEvent {
                        event_type: "run.phase.changed".to_string(),
                        payload: serde_json::json!({
                            "campaignId": snapshot.campaign_id,
                            "runId": snapshot.run_id,
                            "instanceId": snapshot.instance_id,
                            "cohortId": snapshot.cohort_id,
                            "previousPhase": previous.as_ref().map(|prev| prev.progress.current_phase.clone()),
                            "currentPhase": snapshot.progress.current_phase,
                        }),
                    });
                }
                if previous.as_ref().and_then(|prev| prev.current_focus.clone()) != snapshot.current_focus {
                    let _ = state.events.send(UiEvent {
                        event_type: "run.focus.changed".to_string(),
                        payload: serde_json::json!({
                            "campaignId": snapshot.campaign_id,
                            "runId": snapshot.run_id,
                            "instanceId": snapshot.instance_id,
                            "cohortId": snapshot.cohort_id,
                            "previousFocus": previous.as_ref().and_then(|prev| prev.current_focus.clone()),
                            "currentFocus": snapshot.current_focus,
                        }),
                    });
                }
                let previous_warnings = previous
                    .as_ref()
                    .map(|prev| prev.warnings.iter().cloned().collect::<BTreeSet<_>>())
                    .unwrap_or_default();
                for warning in snapshot
                    .warnings
                    .iter()
                    .filter(|warning| !previous_warnings.contains(*warning))
                {
                    let _ = state.events.send(UiEvent {
                        event_type: "run.warning.appended".to_string(),
                        payload: serde_json::json!({
                            "campaignId": snapshot.campaign_id,
                            "runId": snapshot.run_id,
                            "instanceId": snapshot.instance_id,
                            "cohortId": snapshot.cohort_id,
                            "warning": warning,
                            "activityHeat": snapshot.activity_heat,
                            "currentFocus": snapshot.current_focus,
                            "currentPhase": snapshot.progress.current_phase,
                        }),
                    });
                }
            }
            last_live_run_snapshots.insert(run.run_id.clone(), snapshot.clone());
            current.insert(run.run_id.clone(), snapshot);
        }
    }
    last_live_run_snapshots.retain(|run_id, _| current.contains_key(run_id));
    *state.live_runs.write().await = current;
}

async fn emit_campaign_summary_updates(
    state: &AppState,
    index: &WorkspaceIndex,
    last_campaign_summary_snapshots: &mut BTreeMap<String, String>,
) {
    let live_runs = state.live_runs.read().await.clone();
    for campaign in &index.campaigns {
        let campaign_live_runs = live_runs
            .values()
            .filter(|run| run.campaign_id == campaign.campaign_id)
            .cloned()
            .collect::<Vec<_>>();
        let payload = serde_json::json!({
            "campaignId": campaign.campaign_id,
            "status": campaign.status,
            "activeRunCount": campaign.active_run_count,
            "completedRunCount": campaign.completed_run_count,
            "failedRunCount": campaign.failed_run_count,
            "heatCounts": count_by_key(campaign_live_runs.iter().map(|run| run.activity_heat.clone())),
            "activeWarningCount": campaign_live_runs.iter().map(|run| run.warnings.len()).sum::<usize>(),
            "stalledLiveRunCount": campaign_live_runs.iter().filter(|run| run.progress.stalled).count(),
            "personalityFallbackLiveCount": campaign_live_runs.iter().map(|run| run.mechanism.personality_fallback_count).sum::<usize>(),
            "focusSamples": top_unique_strings(campaign_live_runs.iter().filter_map(|run| run.current_focus.clone()), 6),
            "latestMessagePreviews": top_unique_strings(campaign_live_runs.iter().filter_map(|run| run.latest_message_preview.clone()), 6),
        });
        if let Ok(serialized) = serde_json::to_string(&payload) {
            let changed = last_campaign_summary_snapshots
                .get(&campaign.campaign_id)
                .map(|previous| previous != &serialized)
                .unwrap_or(true);
            if changed {
                last_campaign_summary_snapshots.insert(campaign.campaign_id.clone(), serialized);
                let _ = state.events.send(UiEvent {
                    event_type: "campaign.summary.updated".to_string(),
                    payload,
                });
            }
        }
    }
    last_campaign_summary_snapshots.retain(|campaign_id, _| {
        index
            .campaigns
            .iter()
            .any(|campaign| &campaign.campaign_id == campaign_id)
    });
}

fn emit_live_artifact_appends(
    state: &AppState,
    index: &WorkspaceIndex,
    artifact_cursors: &mut BTreeMap<String, FileCursorState>,
) {
    for run in &index.runs {
        if run.status != "running" {
            continue;
        }
        let Some(attempt) = &run.latest_attempt else {
            continue;
        };
        let attempt_dir = PathBuf::from(&attempt.directory);
        let tracked = [
            ("rawAgentEvents", attempt_dir.join("raw-agent-events.jsonl")),
            ("messageMetrics", attempt_dir.join("message-metrics.jsonl")),
            ("toolEvents", attempt_dir.join("tool-events.jsonl")),
            ("patchEvents", attempt_dir.join("patch-events.jsonl")),
            ("patchChain", attempt_dir.join("patch-chain.jsonl")),
            ("commandEvents", attempt_dir.join("command-events.jsonl")),
            ("personalityEvents", attempt_dir.join("personality-events.jsonl")),
            ("skillEvents", attempt_dir.join("skill-events.jsonl")),
            ("skillMechanism", attempt_dir.join("skill-mechanism.jsonl")),
            ("tokenSnapshots", attempt_dir.join("token-snapshots.jsonl")),
            ("codexProbeEvents", attempt_dir.join("codex-probe-events.jsonl")),
            ("lifecycleEvents", attempt_dir.join("lifecycle-events.jsonl")),
            ("anomalies", attempt_dir.join("anomalies.jsonl")),
        ];
        for (artifact_key, path) in tracked {
            let cursor_key = format!("{}::{}::{}", run.run_id, attempt.attempt, artifact_key);
            let cursor = artifact_cursors.entry(cursor_key).or_default();
            let Ok(rows) = append_jsonl_rows_since(&path, cursor) else {
                continue;
            };
            for row in rows {
                let events = if artifact_key == "rawAgentEvents" {
                    live_events_from_raw_agent_event(run, attempt.attempt, &row)
                } else {
                    live_events_from_artifact_row(run, attempt.attempt, artifact_key, &row)
                };
                for event in events {
                    let _ = state.events.send(event);
                }
            }
        }
    }
}

fn maybe_json(path: &Path) -> Option<serde_json::Value> {
    if !path.exists() {
        return None;
    }
    read_json(path).ok()
}

async fn workspace_index_or_cached(state: &AppState) -> Result<WorkspaceIndex> {
    match scan_workspace(&state.repo_root) {
        Ok(index) => {
            *state.workspace_cache.write().await = Some(index.clone());
            Ok(index)
        }
        Err(error) => {
            if let Some(index) = state.workspace_cache.read().await.clone() {
                let _ = state.events.send(UiEvent {
                    event_type: "system.warning".to_string(),
                    payload: serde_json::json!({
                        "message": "workspace scan failed, serving cached snapshot",
                        "error": error.to_string(),
                    }),
                });
                Ok(index)
            } else {
                Err(error)
            }
        }
    }
}

async fn build_campaign_operational_summary(
    state: &AppState,
    index: &WorkspaceIndex,
    campaign: CampaignIndexEntry,
) -> Result<CampaignOperationalSummary> {
    let campaign_dir = campaign_dir_for_id(&state.repo_root, &campaign.campaign_id)?;
    let detail = scan_campaign_detail(&state.repo_root, &campaign_dir)?;
    let active_live_runs = {
        let snapshots = state.live_runs.read().await;
        detail
            .runs
            .iter()
            .filter_map(|run| snapshots.get(&run.run_id).cloned())
            .collect::<Vec<_>>()
    };
    let active_process_count = state
        .processes
        .list()
        .await
        .into_iter()
        .filter(|process| process.status == "running")
        .count();

    let mut solver_status_counts = BTreeMap::new();
    let mut grading_status_counts = BTreeMap::new();
    let mut cohort_counts = BTreeMap::new();
    let mut task_class_counts = BTreeMap::new();
    let mut model_counts = BTreeMap::new();
    let mut personality_counts = BTreeMap::new();
    let mut tool_route_counts = BTreeMap::new();
    let mut tool_name_counts = BTreeMap::new();
    let mut unresolved_infra_failure_count = 0usize;
    let mut active_cohorts = BTreeSet::new();
    let mut active_instances = BTreeSet::new();

    for run in &detail.runs {
        *solver_status_counts.entry(run.status.clone()).or_insert(0) += 1;
        *grading_status_counts.entry(run.grading_status.clone()).or_insert(0) += 1;
        *cohort_counts.entry(run.cohort_id.clone()).or_insert(0) += 1;
        *task_class_counts.entry(run.task_class.clone()).or_insert(0) += 1;
        *model_counts.entry(run.model.clone()).or_insert(0) += 1;
        *personality_counts
            .entry(run.personality_mode.clone().unwrap_or_else(|| "none".to_string()))
            .or_insert(0) += 1;
        for (route, count) in &run.tool_route_counts {
            *tool_route_counts.entry(route.clone()).or_insert(0) += *count;
        }
        for (name, count) in &run.tool_name_counts {
            *tool_name_counts.entry(name.clone()).or_insert(0) += *count;
        }
        if run.grading_status.contains("failed")
            || run.grading_status.contains("error")
            || run.grading_status.contains("env")
        {
            unresolved_infra_failure_count += 1;
        }
    }

    for run in &active_live_runs {
        active_cohorts.insert(run.cohort_id.clone());
        active_instances.insert(run.instance_id.clone());
    }

    let latest_activity_at = active_live_runs
        .iter()
        .filter_map(|run| run.last_event_at.clone())
        .max();
    let live_visible_output_total_tokens_est = active_live_runs
        .iter()
        .map(|run| run.telemetry.visible_output_total_tokens_est)
        .sum();
    let live_total_tokens = active_live_runs
        .iter()
        .map(|run| run.telemetry.total_tokens.unwrap_or_default())
        .sum();
    let live_message_count = active_live_runs.iter().map(|run| run.progress.message_count).sum();
    let live_command_count = active_live_runs.iter().map(|run| run.progress.command_count).sum();
    let live_tool_count = active_live_runs.iter().map(|run| run.progress.tool_count).sum();
    let live_patch_event_count = active_live_runs.iter().map(|run| run.progress.patch_event_count).sum();
    let active_warning_count = active_live_runs.iter().map(|run| run.warnings.len()).sum::<usize>();
    let stalled_live_run_count = active_live_runs.iter().filter(|run| run.progress.stalled).count();
    let personality_fallback_live_count = active_live_runs
        .iter()
        .map(|run| run.mechanism.personality_fallback_count)
        .sum::<usize>();
    let heat_counts = count_by_key(active_live_runs.iter().map(|run| run.activity_heat.clone()));
    let focus_samples = top_unique_strings(
        active_live_runs.iter().filter_map(|run| run.current_focus.clone()),
        6,
    );
    let latest_message_previews = top_unique_strings(
        active_live_runs.iter().filter_map(|run| run.latest_message_preview.clone()),
        6,
    );
    let mut operational_warnings = Vec::new();
    if unresolved_infra_failure_count > 0 {
        operational_warnings.push(format!("infra failures pending: {unresolved_infra_failure_count}"));
    }
    if stalled_live_run_count > 0 {
        operational_warnings.push(format!("stalled live runs: {stalled_live_run_count}"));
    }
    let fallbacks = active_live_runs
        .iter()
        .map(|run| run.mechanism.personality_fallback_count)
        .sum::<usize>();
    if fallbacks > 0 {
        operational_warnings.push(format!("personality fallback observed ×{fallbacks}"));
    }
    let mut latest_reports = detail.reports.clone();
    latest_reports.sort_by(|left, right| right.updated_at.cmp(&left.updated_at).then_with(|| left.name.cmp(&right.name)));
    latest_reports.truncate(8);
    let mut latest_datasets = detail.datasets.clone();
    latest_datasets.sort_by(|left, right| right.updated_at.cmp(&left.updated_at).then_with(|| left.name.cmp(&right.name)));
    latest_datasets.truncate(8);

    let _ = index;
    Ok(CampaignOperationalSummary {
        campaign,
        active_live_runs,
        latest_reports,
        latest_datasets,
        active_process_count,
        latest_activity_at,
        live_visible_output_total_tokens_est,
        live_total_tokens,
        live_message_count,
        live_command_count,
        live_tool_count,
        live_patch_event_count,
        solver_status_counts,
        grading_status_counts,
        cohort_counts,
        task_class_counts,
        model_counts,
        personality_counts,
        tool_route_counts,
        tool_name_counts,
        active_cohorts: active_cohorts.into_iter().collect(),
        active_instances: active_instances.into_iter().collect(),
        unresolved_infra_failure_count,
        active_warning_count,
        stalled_live_run_count,
        personality_fallback_live_count,
        heat_counts,
        focus_samples,
        latest_message_previews,
        operational_warnings,
    })
}

fn live_heat_score(snapshot: &LiveRunSnapshot) -> i64 {
    let heat = match snapshot.activity_heat.as_str() {
        "hot" => 3,
        "warm" => 2,
        "cool" => 1,
        "stalled" => -1,
        _ => 0,
    };
    (heat * 10_000) as i64
        + snapshot.progress.tool_count as i64 * 100
        + snapshot.progress.command_count as i64 * 10
        + snapshot.progress.message_count as i64
}

fn trim_preview(text: &str, limit: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= limit {
        return text.to_string();
    }
    let preview = text.chars().take(limit).collect::<String>();
    format!("{preview}\n\n[... truncated ...]")
}

fn count_by_key<I>(values: I) -> BTreeMap<String, usize>
where
    I: IntoIterator<Item = String>,
{
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
}

fn top_unique_strings<I>(values: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed.to_string();
        if seen.insert(normalized.clone()) {
            rows.push(normalized);
        }
        if rows.len() >= limit {
            break;
        }
    }
    rows
}

fn event_matches_run_id(event: &UiEvent, run_id: &str) -> bool {
    let Some(payload) = event.payload.as_object() else {
        return false;
    };
    payload.get("run_id").and_then(serde_json::Value::as_str) == Some(run_id)
        || payload.get("runId").and_then(serde_json::Value::as_str) == Some(run_id)
}

#[cfg(test)]
mod tests {
    use super::event_matches_run_id;
    use crate::processes::UiEvent;

    #[test]
    fn matches_snake_case_run_id() {
        let event = UiEvent {
            event_type: "run.updated".to_string(),
            payload: serde_json::json!({ "run_id": "run-123" }),
        };
        assert!(event_matches_run_id(&event, "run-123"));
        assert!(!event_matches_run_id(&event, "run-999"));
    }

    #[test]
    fn matches_camel_case_run_id() {
        let event = UiEvent {
            event_type: "run.live.updated".to_string(),
            payload: serde_json::json!({ "runId": "run-abc" }),
        };
        assert!(event_matches_run_id(&event, "run-abc"));
        assert!(!event_matches_run_id(&event, "other"));
    }
}

fn build_timeline(tables: &BTreeMap<String, Vec<serde_json::Value>>) -> Vec<TimelineRow> {
    let mut rows = Vec::new();
    append_timeline_rows(&mut rows, tables.get("messageMetrics"), "message", |row| {
        (
            row.get("timestamp").and_then(serde_json::Value::as_str).map(str::to_string),
            row.get("primaryCategory").and_then(serde_json::Value::as_str).unwrap_or("message").to_string(),
            row.get("messageId").and_then(serde_json::Value::as_str).unwrap_or("Message").to_string(),
            row.get("textPreview").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
        )
    });
    append_timeline_rows(&mut rows, tables.get("toolEvents"), "tool", |row| {
        (
            row.get("timestamp").and_then(serde_json::Value::as_str).map(str::to_string),
            row.get("phase").and_then(serde_json::Value::as_str).unwrap_or("tool").to_string(),
            row.get("toolName").and_then(serde_json::Value::as_str).unwrap_or("tool").to_string(),
            format!(
                "{} / {}",
                row.get("toolKind").and_then(serde_json::Value::as_str).unwrap_or("-"),
                row.get("toolRoute").and_then(serde_json::Value::as_str).unwrap_or("-")
            ),
        )
    });
    append_timeline_rows(&mut rows, tables.get("patchChain"), "patch", |row| {
        (
            row.get("timestamp").and_then(serde_json::Value::as_str).map(str::to_string),
            row.get("phase").and_then(serde_json::Value::as_str).unwrap_or("patch").to_string(),
            row.get("title").and_then(serde_json::Value::as_str).unwrap_or("patch").to_string(),
            row.get("summary").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
        )
    });
    append_timeline_rows(&mut rows, tables.get("personalityEvents"), "mechanism", |row| {
        (
            row.get("timestamp").and_then(serde_json::Value::as_str).map(str::to_string),
            row.get("kind").and_then(serde_json::Value::as_str).unwrap_or("personality").to_string(),
            row.get("requestedPersonality").and_then(serde_json::Value::as_str).unwrap_or("personality").to_string(),
            row.get("summary").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
        )
    });
    append_timeline_rows(&mut rows, tables.get("skillMechanism"), "mechanism", |row| {
        (
            row.get("timestamp").and_then(serde_json::Value::as_str).map(str::to_string),
            row.get("kind").and_then(serde_json::Value::as_str).unwrap_or("skill").to_string(),
            row.get("skillName").and_then(serde_json::Value::as_str).unwrap_or("skill").to_string(),
            row.get("summary").and_then(serde_json::Value::as_str).unwrap_or("").to_string(),
        )
    });
    rows.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    rows
}

fn append_timeline_rows<F>(
    rows: &mut Vec<TimelineRow>,
    source: Option<&Vec<serde_json::Value>>,
    lane: &str,
    mapper: F,
) where
    F: Fn(&serde_json::Value) -> (Option<String>, String, String, String),
{
    let Some(source) = source else {
        return;
    };
    for row in source {
        let (timestamp, kind, title, summary) = mapper(row);
        rows.push(TimelineRow {
            lane: lane.to_string(),
            kind,
            timestamp,
            title,
            summary,
            payload: row.clone(),
        });
    }
}

fn campaign_dir_for_id(repo_root: &Path, campaign_id: &str) -> Result<PathBuf> {
    let artifacts_root = repo_root.join("artifacts");
    for entry in std::fs::read_dir(artifacts_root)? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("campaign-manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let manifest: CampaignManifest = read_json(&manifest_path)?;
        if manifest.campaign_id == campaign_id {
            return Ok(path);
        }
    }
    anyhow::bail!("campaign not found")
}

fn detect_format(path: &Path) -> String {
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    if file_name.ends_with(".diff") || file_name == "patch.diff" {
        return "diff".to_string();
    }
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "csv" => "csv".to_string(),
        "jsonl" => "jsonl".to_string(),
        _ => "text".to_string(),
    }
}

fn safe_path(repo_root: &Path, path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    };
    let canonical = absolute.canonicalize()?;
    let repo_root = repo_root.canonicalize()?;
    if !canonical.starts_with(&repo_root) {
        anyhow::bail!("artifact path escapes repo root");
    }
    Ok(canonical)
}

type ApiResult<T> = std::result::Result<T, ApiError>;

pub struct ApiError(anyhow::Error);

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let payload = Json(serde_json::json!({
            "error": self.0.to_string(),
        }));
        (StatusCode::BAD_REQUEST, payload).into_response()
    }
}
