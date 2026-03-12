use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use codex_bench_core::{
    CampaignManifest, ProbeSummary, RunManifest, RunSummary, artifact_inventory_for_attempt,
    artifact_role_map_for_attempt, read_json,
};

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactDescriptor {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub exists: bool,
    pub role: Option<String>,
    pub scope: String,
    pub format: String,
    pub size_bytes: Option<u64>,
    pub updated_at: Option<String>,
    pub line_count: Option<usize>,
    pub row_count: Option<usize>,
    pub previewable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttemptIndex {
    pub attempt: u32,
    pub directory: String,
    pub artifacts: Vec<ArtifactDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunIndexEntry {
    pub campaign_id: String,
    pub run_id: String,
    pub manifest_run_id: String,
    pub instance_id: String,
    pub repo: String,
    pub task_class: String,
    pub cohort_id: String,
    pub model: String,
    pub provider: String,
    pub personality_mode: Option<String>,
    pub prompt_style: Option<String>,
    pub status: String,
    pub grading_status: String,
    pub run_dir: String,
    pub manifest_path: String,
    pub latest_updated_at: Option<String>,
    pub command_count: usize,
    pub tool_count: usize,
    pub patch_file_count: usize,
    pub message_metric_count: usize,
    pub visible_output_total_tokens_est: i64,
    pub total_tokens: Option<i64>,
    pub anomaly_count: usize,
    pub tool_kind_counts: BTreeMap<String, usize>,
    pub tool_name_counts: BTreeMap<String, usize>,
    pub tool_route_counts: BTreeMap<String, usize>,
    pub message_category_counts: BTreeMap<String, usize>,
    pub ignition_shell_search_count: usize,
    pub verification_closure_count: usize,
    pub personality_fallback_count: usize,
    pub harness_friction_count: usize,
    pub latest_attempt: Option<AttemptIndex>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignIndexEntry {
    pub campaign_id: String,
    pub experiment_name: String,
    pub benchmark_name: String,
    pub benchmark_adapter: String,
    pub stage_name: Option<String>,
    pub created_at: String,
    pub status: String,
    pub sample_size: usize,
    pub cohort_count: usize,
    pub max_parallel_runs: usize,
    pub report_paths: Vec<String>,
    pub dataset_paths: Vec<String>,
    pub selected_instances: usize,
    pub active_run_count: usize,
    pub completed_run_count: usize,
    pub failed_run_count: usize,
    pub report_count: usize,
    pub dataset_count: usize,
    pub total_tokens: i64,
    pub total_visible_output_tokens_est: i64,
    pub total_tool_calls: usize,
    pub total_commands: usize,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignDetail {
    pub manifest: CampaignManifest,
    pub reports: Vec<ArtifactDescriptor>,
    pub datasets: Vec<ArtifactDescriptor>,
    pub runs: Vec<RunIndexEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceIndex {
    pub repo_root: String,
    pub generated_at: String,
    pub campaigns: Vec<CampaignIndexEntry>,
    pub runs: Vec<RunIndexEntry>,
    pub summary: WorkspaceSummary,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct WorkspaceSummary {
    pub campaign_count: usize,
    pub run_count: usize,
    pub active_run_count: usize,
    pub completed_run_count: usize,
    pub failed_run_count: usize,
    pub total_tokens: i64,
    pub total_visible_output_tokens_est: i64,
    pub total_tool_calls: usize,
    pub total_commands: usize,
}

fn sorted_child_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            files.push(entry.path());
        }
    }
    files.sort();
    files
}

fn detect_artifact_format(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("file")
        .to_string()
}

fn artifact_stats(path: &Path) -> (Option<u64>, Option<String>, Option<usize>, Option<usize>) {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return (None, None, None, None),
    };
    let size_bytes = Some(metadata.len());
    let updated_at = metadata
        .modified()
        .ok()
        .map(|time| DateTime::<Utc>::from(time).to_rfc3339());
    let format = detect_artifact_format(path);
    if metadata.len() > 1_500_000 {
        return (size_bytes, updated_at, None, None);
    }
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return (size_bytes, updated_at, None, None),
    };
    let line_count = Some(raw.lines().count());
    let row_count = match format.as_str() {
        "jsonl" => line_count,
        "csv" => line_count.map(|count| count.saturating_sub(1)),
        _ => None,
    };
    (size_bytes, updated_at, line_count, row_count)
}

fn is_previewable_format(format: &str) -> bool {
    matches!(format, "txt" | "md" | "json" | "jsonl" | "csv" | "diff" | "patch" | "log")
}

fn build_artifact_descriptor(path: &Path, role: Option<String>, scope: &str) -> ArtifactDescriptor {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact")
        .to_string();
    let kind = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("file")
        .to_string();
    let format = detect_artifact_format(path);
    let (size_bytes, updated_at, line_count, row_count) = if path.exists() {
        artifact_stats(path)
    } else {
        (None, None, None, None)
    };
    ArtifactDescriptor {
        name,
        path: path.display().to_string(),
        kind,
        exists: path.exists(),
        role,
        scope: scope.to_string(),
        format: format.clone(),
        size_bytes,
        updated_at,
        line_count,
        row_count,
        previewable: is_previewable_format(&format),
    }
}

fn artifact_descriptors(dir: &Path, scope: &str, role: Option<&str>) -> Vec<ArtifactDescriptor> {
    sorted_child_files(dir)
        .into_iter()
        .filter(|path| path.is_file())
        .map(|path| build_artifact_descriptor(&path, role.map(str::to_string), scope))
        .collect()
}

fn latest_attempt_for_run(run_dir: &Path) -> Option<AttemptIndex> {
    let attempts_dir = run_dir;
    let mut attempts = sorted_child_files(attempts_dir)
        .into_iter()
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_string();
            if !name.starts_with("attempt-") {
                return None;
            }
            let attempt = name.trim_start_matches("attempt-").parse::<u32>().ok()?;
            Some((attempt, path))
        })
        .collect::<Vec<_>>();
    attempts.sort_by_key(|(attempt, _)| *attempt);
    let (attempt, dir) = attempts.pop()?;
    let artifact_roles = artifact_role_map_for_attempt();
    let inventory = artifact_inventory_for_attempt(&dir);
    let mut artifacts = Vec::new();
    for (artifact_name, exists) in inventory {
        let path = dir.join(match artifact_name.as_str() {
            "prompt" => "prompt.txt",
            "environmentPlan" => "environment-plan.json",
            "rawAgentEvents" => "raw-agent-events.jsonl",
            "rawDiagnostics" => "raw-diagnostics.jsonl",
            "codexProbeEvents" => "codex-probe-events.jsonl",
            "lifecycleEvents" => "lifecycle-events.jsonl",
            "tokenSnapshots" => "token-snapshots.jsonl",
            "turnMetrics" => "turn-metrics.jsonl",
            "messageMetrics" => "message-metrics.jsonl",
            "personalityEvents" => "personality-events.jsonl",
            "commandEvents" => "command-events.jsonl",
            "toolEvents" => "tool-events.jsonl",
            "skillEvents" => "skill-events.jsonl",
            "skillMechanism" => "skill-mechanism.jsonl",
            "patchEvents" => "patch-events.jsonl",
            "patchChain" => "patch-chain.jsonl",
            "gradeEvents" => "grade-events.jsonl",
            "anomalies" => "anomalies.jsonl",
            "verbosityToolCoupling" => "verbosity-tool-coupling.jsonl",
            "probeEvents" => "probe-events.jsonl",
            "probeSummary" => "probe-summary.json",
            "claimEvidence" => "claim-evidence.json",
            "patch" => "patch.diff",
            "runSummary" => "run-summary.json",
            "runEvidence" => "run-evidence.txt",
            "attemptLog" => "attempt-log.txt",
            "replay" => "replay.json",
            _ => return None,
        });
        let mut descriptor = build_artifact_descriptor(
            &path,
            artifact_roles.get(&artifact_name).cloned(),
            "attempt_artifact",
        );
        descriptor.name = artifact_name.clone();
        descriptor.exists = exists;
        artifacts.push(descriptor);
    }
    Some(AttemptIndex {
        attempt,
        directory: dir.display().to_string(),
        artifacts,
    })
}

fn load_run_manifest(run_dir: &Path) -> Option<RunManifest> {
    let manifest_path = run_dir.join("manifest.json");
    if !manifest_path.exists() {
        return None;
    }
    read_json(&manifest_path).ok()
}

fn load_run_metrics(run_dir: &Path) -> (Option<RunSummary>, Option<ProbeSummary>, Option<String>) {
    let attempt = latest_attempt_for_run(run_dir);
    let Some(attempt) = attempt else {
        return (None, None, None);
    };
    let attempt_dir = PathBuf::from(&attempt.directory);
    let summary = read_json::<RunSummary>(&attempt_dir.join("run-summary.json")).ok();
    let probe_summary = read_json::<ProbeSummary>(&attempt_dir.join("probe-summary.json")).ok();
    let latest_updated_at = std::fs::metadata(&attempt_dir)
        .and_then(|meta| meta.modified())
        .ok()
        .map(|time| DateTime::<Utc>::from(time).to_rfc3339());
    (summary, probe_summary, latest_updated_at)
}

pub fn scan_campaign_detail(_repo_root: &Path, campaign_dir: &Path) -> Result<CampaignDetail> {
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    let mut runs = Vec::new();
    let runs_root = campaign_dir.join("runs");
    if runs_root.exists() {
        for cohort_dir in sorted_child_files(&runs_root) {
            if !cohort_dir.is_dir() {
                continue;
            }
            let cohort_entries = sorted_child_files(&cohort_dir);
            let contains_run_manifest = cohort_dir.join("manifest.json").exists();
            if contains_run_manifest {
                if let Some(run_manifest) = load_run_manifest(&cohort_dir) {
                    let (summary, probe_summary, latest_updated_at) = load_run_metrics(&cohort_dir);
                    runs.push(RunIndexEntry {
                        campaign_id: run_manifest.campaign_id.clone(),
                        run_id: format!(
                            "{}--{}--{}",
                            run_manifest.campaign_id, run_manifest.cohort_id, run_manifest.instance_id
                        ),
                        manifest_run_id: run_manifest.run_id.clone(),
                        instance_id: run_manifest.instance_id.clone(),
                        repo: run_manifest.repo.clone(),
                        task_class: run_manifest.task_class.clone(),
                        cohort_id: run_manifest.cohort_id.clone(),
                        model: run_manifest.model.clone(),
                        provider: run_manifest.provider.clone(),
                        personality_mode: run_manifest.personality_mode.clone(),
                        prompt_style: run_manifest.prompt_style.clone(),
                        status: run_manifest.status.clone(),
                        grading_status: run_manifest.grading_status.clone(),
                        run_dir: cohort_dir.display().to_string(),
                        manifest_path: cohort_dir.join("manifest.json").display().to_string(),
                        latest_updated_at,
                        command_count: summary.as_ref().map(|item| item.command_count).unwrap_or_default(),
                        tool_count: summary.as_ref().map(|item| item.tool_count).unwrap_or_default(),
                        patch_file_count: summary.as_ref().map(|item| item.patch_file_count).unwrap_or_default(),
                        message_metric_count: summary.as_ref().map(|item| item.message_metric_count).unwrap_or_default(),
                        visible_output_total_tokens_est: summary.as_ref().map(|item| item.visible_output_total_tokens_est).unwrap_or_default(),
                        total_tokens: summary.as_ref().and_then(|item| item.total_tokens),
                        anomaly_count: summary.as_ref().map(|item| item.anomaly_count).unwrap_or_default(),
                        tool_kind_counts: summary.as_ref().map(|item| item.tool_kind_counts.clone()).unwrap_or_default(),
                        tool_name_counts: summary.as_ref().map(|item| item.tool_name_counts.clone()).unwrap_or_default(),
                        tool_route_counts: summary.as_ref().map(|item| item.tool_route_counts.clone()).unwrap_or_default(),
                        message_category_counts: summary.as_ref().map(|item| item.message_category_counts.clone()).unwrap_or_default(),
                        ignition_shell_search_count: probe_summary.as_ref().map(|item| item.ignition_shell_search_count).unwrap_or_default(),
                        verification_closure_count: probe_summary.as_ref().map(|item| item.verification_closure_count).unwrap_or_default(),
                        personality_fallback_count: probe_summary.as_ref().map(|item| item.personality_fallback_count).unwrap_or_default(),
                        harness_friction_count: probe_summary.as_ref().map(|item| item.harness_friction_count).unwrap_or_default(),
                        latest_attempt: latest_attempt_for_run(&cohort_dir),
                    });
                }
                continue;
            }

            for run_dir in cohort_entries {
                if !run_dir.is_dir() {
                    continue;
                }
                if let Some(run_manifest) = load_run_manifest(&run_dir) {
                    let (summary, probe_summary, latest_updated_at) = load_run_metrics(&run_dir);
                    runs.push(RunIndexEntry {
                        campaign_id: run_manifest.campaign_id.clone(),
                        run_id: format!(
                            "{}--{}--{}",
                            run_manifest.campaign_id, run_manifest.cohort_id, run_manifest.instance_id
                        ),
                        manifest_run_id: run_manifest.run_id.clone(),
                        instance_id: run_manifest.instance_id.clone(),
                        repo: run_manifest.repo.clone(),
                        task_class: run_manifest.task_class.clone(),
                        cohort_id: run_manifest.cohort_id.clone(),
                        model: run_manifest.model.clone(),
                        provider: run_manifest.provider.clone(),
                        personality_mode: run_manifest.personality_mode.clone(),
                        prompt_style: run_manifest.prompt_style.clone(),
                        status: run_manifest.status.clone(),
                        grading_status: run_manifest.grading_status.clone(),
                        run_dir: run_dir.display().to_string(),
                        manifest_path: run_dir.join("manifest.json").display().to_string(),
                        latest_updated_at,
                        command_count: summary.as_ref().map(|item| item.command_count).unwrap_or_default(),
                        tool_count: summary.as_ref().map(|item| item.tool_count).unwrap_or_default(),
                        patch_file_count: summary.as_ref().map(|item| item.patch_file_count).unwrap_or_default(),
                        message_metric_count: summary.as_ref().map(|item| item.message_metric_count).unwrap_or_default(),
                        visible_output_total_tokens_est: summary.as_ref().map(|item| item.visible_output_total_tokens_est).unwrap_or_default(),
                        total_tokens: summary.as_ref().and_then(|item| item.total_tokens),
                        anomaly_count: summary.as_ref().map(|item| item.anomaly_count).unwrap_or_default(),
                        tool_kind_counts: summary.as_ref().map(|item| item.tool_kind_counts.clone()).unwrap_or_default(),
                        tool_name_counts: summary.as_ref().map(|item| item.tool_name_counts.clone()).unwrap_or_default(),
                        tool_route_counts: summary.as_ref().map(|item| item.tool_route_counts.clone()).unwrap_or_default(),
                        message_category_counts: summary.as_ref().map(|item| item.message_category_counts.clone()).unwrap_or_default(),
                        ignition_shell_search_count: probe_summary.as_ref().map(|item| item.ignition_shell_search_count).unwrap_or_default(),
                        verification_closure_count: probe_summary.as_ref().map(|item| item.verification_closure_count).unwrap_or_default(),
                        personality_fallback_count: probe_summary.as_ref().map(|item| item.personality_fallback_count).unwrap_or_default(),
                        harness_friction_count: probe_summary.as_ref().map(|item| item.harness_friction_count).unwrap_or_default(),
                        latest_attempt: latest_attempt_for_run(&run_dir),
                    });
                }
            }
        }
    }
    runs.sort_by(|a, b| a.run_id.cmp(&b.run_id));

    let reports = artifact_descriptors(&campaign_dir.join("reports"), "campaign_report", Some("campaign_report"));
    let datasets = artifact_descriptors(&campaign_dir.join("datasets"), "campaign_dataset", Some("campaign_dataset"));
    Ok(CampaignDetail {
        manifest,
        reports,
        datasets,
        runs,
    })
}

pub fn scan_workspace(repo_root: &Path) -> Result<WorkspaceIndex> {
    let artifacts_root = repo_root.join("artifacts");
    let mut campaigns = Vec::new();
    let mut all_runs = Vec::new();
    if artifacts_root.exists() {
        for campaign_dir in sorted_child_files(&artifacts_root) {
            if !campaign_dir.is_dir() {
                continue;
            }
            let manifest_path = campaign_dir.join("campaign-manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            let detail = scan_campaign_detail(repo_root, &campaign_dir)?;
            let active_run_count = detail
                .runs
                .iter()
                .filter(|run| run.status == "running")
                .count();
            let completed_run_count = detail
                .runs
                .iter()
                .filter(|run| run.status == "completed")
                .count();
            let failed_run_count = detail
                .runs
                .iter()
                .filter(|run| run.status == "failed")
                .count();
            let total_tokens = detail
                .runs
                .iter()
                .map(|run| run.total_tokens.unwrap_or_default())
                .sum();
            let total_visible_output_tokens_est = detail
                .runs
                .iter()
                .map(|run| run.visible_output_total_tokens_est)
                .sum();
            let total_tool_calls = detail.runs.iter().map(|run| run.tool_count).sum();
            let total_commands = detail.runs.iter().map(|run| run.command_count).sum();
            campaigns.push(CampaignIndexEntry {
                campaign_id: detail.manifest.campaign_id.clone(),
                experiment_name: detail.manifest.experiment_name.clone(),
                benchmark_name: detail.manifest.benchmark_name.clone(),
                benchmark_adapter: detail.manifest.benchmark_adapter.clone(),
                stage_name: detail.manifest.stage_name.clone(),
                created_at: detail.manifest.created_at.clone(),
                status: detail.manifest.campaign_status.clone(),
                sample_size: detail.manifest.sample_size,
                cohort_count: detail.manifest.cohorts.len(),
                max_parallel_runs: detail.manifest.max_parallel_runs,
                report_paths: detail.reports.iter().map(|report| report.path.clone()).collect(),
                dataset_paths: detail.datasets.iter().map(|dataset| dataset.path.clone()).collect(),
                selected_instances: detail.manifest.selected_instances.len(),
                active_run_count,
                completed_run_count,
                failed_run_count,
                report_count: detail.reports.len(),
                dataset_count: detail.datasets.len(),
                total_tokens,
                total_visible_output_tokens_est,
                total_tool_calls,
                total_commands,
                path: campaign_dir.display().to_string(),
            });
            all_runs.extend(detail.runs);
        }
    }
    campaigns.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    all_runs.sort_by(|a, b| {
        a.campaign_id
            .cmp(&b.campaign_id)
            .then(a.cohort_id.cmp(&b.cohort_id))
            .then(a.instance_id.cmp(&b.instance_id))
    });
    let summary = WorkspaceSummary {
        campaign_count: campaigns.len(),
        run_count: all_runs.len(),
        active_run_count: all_runs.iter().filter(|run| run.status == "running").count(),
        completed_run_count: all_runs.iter().filter(|run| run.status == "completed").count(),
        failed_run_count: all_runs.iter().filter(|run| run.status == "failed").count(),
        total_tokens: all_runs.iter().map(|run| run.total_tokens.unwrap_or_default()).sum(),
        total_visible_output_tokens_est: all_runs.iter().map(|run| run.visible_output_total_tokens_est).sum(),
        total_tool_calls: all_runs.iter().map(|run| run.tool_count).sum(),
        total_commands: all_runs.iter().map(|run| run.command_count).sum(),
    };
    Ok(WorkspaceIndex {
        repo_root: repo_root.display().to_string(),
        generated_at: DateTime::<Utc>::from(std::time::SystemTime::now()).to_rfc3339(),
        campaigns,
        runs: all_runs,
        summary,
    })
}

pub fn read_text_file(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

pub fn read_csv_file(path: &Path) -> Result<Vec<BTreeMap<String, String>>> {
    let raw = fs::read_to_string(path)?;
    let mut lines = raw.lines();
    let headers = match lines.next() {
        Some(header) => header
            .split(',')
            .map(|value| value.trim().to_string())
            .collect::<Vec<_>>(),
        None => return Ok(Vec::new()),
    };
    let mut rows = Vec::new();
    for line in lines {
        let values = line.split(',').map(|value| value.trim().to_string()).collect::<Vec<_>>();
        let mut row = BTreeMap::new();
        for (index, header) in headers.iter().enumerate() {
            row.insert(header.clone(), values.get(index).cloned().unwrap_or_default());
        }
        rows.push(row);
    }
    Ok(rows)
}

pub fn read_jsonl_file(path: &Path) -> Result<Vec<Value>> {
    codex_bench_core::read_jsonl_values(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn build_artifact_descriptor_includes_role_and_stats() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.txt");
        fs::write(&path, "line1\nline2\n").unwrap();
        let descriptor = build_artifact_descriptor(&path, Some("campaign_report".into()), "campaign_report");
        assert_eq!(descriptor.role.as_deref(), Some("campaign_report"));
        assert_eq!(descriptor.scope, "campaign_report");
        assert_eq!(descriptor.format, "txt");
        assert_eq!(descriptor.line_count, Some(2));
        assert!(descriptor.previewable);
    }
}
