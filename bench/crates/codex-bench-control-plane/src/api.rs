use std::collections::BTreeMap;
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
use tokio::sync::broadcast;
use tokio::time::sleep;

use codex_bench_core::{CampaignManifest, read_json};
use crate::index::{
    ArtifactDescriptor, CampaignDetail, RunIndexEntry, WorkspaceIndex, read_csv_file, read_jsonl_file,
    read_text_file, scan_campaign_detail, scan_workspace,
};
use crate::processes::{ProcessRegistry, UiEvent};

#[derive(Clone)]
pub struct AppState {
    pub repo_root: PathBuf,
    pub processes: ProcessRegistry,
    pub events: broadcast::Sender<UiEvent>,
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
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/workspace/index", get(get_workspace_index))
        .route("/api/campaigns", get(list_campaigns))
        .route("/api/campaigns/{campaign_id}", get(get_campaign))
        .route("/api/campaigns/{campaign_id}/reports", get(get_campaign_reports))
        .route("/api/campaigns/{campaign_id}/datasets", get(get_campaign_datasets))
        .route("/api/runs/{run_id}", get(get_run))
        .route("/api/runs/{run_id}/detail", get(get_run_detail))
        .route("/api/runs/{run_id}/attempts/{attempt}", get(get_run_attempt))
        .route("/api/processes", get(list_processes))
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
    Ok(Json(scan_workspace(&state.repo_root)?))
}

async fn list_campaigns(State(state): State<AppState>) -> ApiResult<Json<Vec<crate::index::CampaignIndexEntry>>> {
    let index = scan_workspace(&state.repo_root)?;
    Ok(Json(index.campaigns))
}

async fn get_campaign(
    State(state): State<AppState>,
    AxumPath(campaign_id): AxumPath<String>,
) -> ApiResult<Json<CampaignDetail>> {
    let campaign_dir = campaign_dir_for_id(&state.repo_root, &campaign_id)?;
    Ok(Json(scan_campaign_detail(&state.repo_root, &campaign_dir)?))
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
    let index = scan_workspace(&state.repo_root)?;
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
    let index = scan_workspace(&state.repo_root)?;
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
    }))
}

async fn get_run_attempt(
    State(state): State<AppState>,
    AxumPath((run_id, attempt)): AxumPath<(String, u32)>,
) -> ApiResult<Json<serde_json::Value>> {
    let index = scan_workspace(&state.repo_root)?;
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
    let mut last_run_snapshots = BTreeMap::<String, String>::new();
    loop {
        if let Ok(index) = scan_workspace(&state.repo_root) {
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

fn maybe_json(path: &Path) -> Option<serde_json::Value> {
    if !path.exists() {
        return None;
    }
    read_json(path).ok()
}

fn trim_preview(text: &str, limit: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= limit {
        return text.to_string();
    }
    let preview = text.chars().take(limit).collect::<String>();
    format!("{preview}\n\n[... truncated ...]")
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
