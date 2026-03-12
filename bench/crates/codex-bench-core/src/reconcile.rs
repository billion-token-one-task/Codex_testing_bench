use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::io::{read_json, write_json_pretty};
use crate::types::{CampaignManifest, RunManifest};

const DEFAULT_STALE_RUNNING_SECS: i64 = 15 * 60;

#[derive(Debug, Clone, Default)]
pub struct ReconciliationReport {
    pub stale_runs_fixed: usize,
    pub campaign_status_changed: bool,
    pub touched_run_dirs: Vec<PathBuf>,
}

pub fn default_stale_running_secs() -> i64 {
    DEFAULT_STALE_RUNNING_SECS
}

pub fn reconcile_campaign_state(campaign_dir: &Path) -> Result<ReconciliationReport> {
    reconcile_campaign_state_with_threshold(campaign_dir, DEFAULT_STALE_RUNNING_SECS)
}

pub fn reconcile_campaign_state_with_threshold(
    campaign_dir: &Path,
    stale_after_secs: i64,
) -> Result<ReconciliationReport> {
    let manifest_path = campaign_dir.join("campaign-manifest.json");
    if !manifest_path.exists() {
        return Ok(ReconciliationReport::default());
    }

    let mut report = ReconciliationReport::default();
    let mut campaign_manifest: CampaignManifest = read_json(&manifest_path)?;
    let run_dirs = discover_run_dirs(&campaign_dir.join("runs"));

    for run_dir in &run_dirs {
        let run_manifest_path = run_dir.join("manifest.json");
        if !run_manifest_path.exists() {
            continue;
        }
        let mut run_manifest: RunManifest = read_json(&run_manifest_path)?;
        if run_manifest.status != "running" {
            continue;
        }

        let stale = latest_run_activity(run_dir, &run_manifest)
            .map(|activity| (Utc::now() - activity).num_seconds() > stale_after_secs)
            .unwrap_or(false);

        if !stale {
            continue;
        }

        let now = Utc::now().to_rfc3339();
        run_manifest.status = "failed".to_string();
        if run_manifest.grading_status.trim().is_empty()
            || run_manifest.grading_status == "pending"
        {
            run_manifest.grading_status = "grader_not_run".to_string();
        }
        if run_manifest.failure_reason.is_none() {
            run_manifest.failure_reason = Some(format!(
                "run was left in `running` state without fresh activity for more than {} seconds",
                stale_after_secs
            ));
        }
        if run_manifest.failure_class.is_none() {
            run_manifest.failure_class = Some("interrupted".to_string());
        }
        run_manifest.last_updated_at = Some(now.clone());
        run_manifest.completed_at = Some(now);
        if run_dir.join("attempt-01").join("probe-summary.json").exists() {
            run_manifest.derivations_status = "completed".to_string();
        } else if run_manifest.derivations_status.trim().is_empty()
            || run_manifest.derivations_status == "pending"
        {
            run_manifest.derivations_status = "failed".to_string();
        }
        if run_dir.join("attempt-01").join("run-evidence.txt").exists() {
            run_manifest.evidence_status = "run_evidence_generated".to_string();
        } else if run_manifest.evidence_status.trim().is_empty()
            || run_manifest.evidence_status == "pending"
        {
            run_manifest.evidence_status = "pending".to_string();
        }
        write_json_pretty(&run_manifest_path, &run_manifest)?;
        report.stale_runs_fixed += 1;
        report.touched_run_dirs.push(run_dir.clone());
    }

    let statuses = discover_run_dirs(&campaign_dir.join("runs"))
        .into_iter()
        .filter_map(|run_dir| {
            let path = run_dir.join("manifest.json");
            if !path.exists() {
                return None;
            }
            read_json::<RunManifest>(&path).ok().map(|manifest| manifest.status)
        })
        .collect::<Vec<_>>();

    if !statuses.iter().any(|status| status == "running") {
        let next_status = if campaign_dir.join("reports").join("report.txt").exists() {
            if campaign_dir.join("reports").join("grader.json").exists() {
                "graded_report_generated"
            } else {
                "report_generated"
            }
        } else if campaign_dir.join("reports").join("grader.json").exists() {
            "graded"
        } else if !statuses.is_empty() {
            "run_completed"
        } else {
            campaign_manifest.campaign_status.as_str()
        };

        if campaign_manifest.campaign_status != next_status {
            campaign_manifest.campaign_status = next_status.to_string();
            write_json_pretty(&manifest_path, &campaign_manifest)?;
            report.campaign_status_changed = true;
        }
    }

    Ok(report)
}

fn discover_run_dirs(runs_root: &Path) -> Vec<PathBuf> {
    let mut run_dirs = Vec::new();
    if !runs_root.exists() {
        return run_dirs;
    }
    let Ok(entries) = fs::read_dir(runs_root) else {
        return run_dirs;
    };
    for cohort_entry in entries.flatten() {
        let cohort_dir = cohort_entry.path();
        if !cohort_dir.is_dir() {
            continue;
        }
        if cohort_dir.join("manifest.json").exists() {
            run_dirs.push(cohort_dir);
            continue;
        }
        let Ok(run_entries) = fs::read_dir(&cohort_dir) else {
            continue;
        };
        for run_entry in run_entries.flatten() {
            let run_dir = run_entry.path();
            if run_dir.is_dir() && run_dir.join("manifest.json").exists() {
                run_dirs.push(run_dir);
            }
        }
    }
    run_dirs.sort();
    run_dirs
}

fn latest_run_activity(run_dir: &Path, manifest: &RunManifest) -> Option<DateTime<Utc>> {
    let mut timestamps = Vec::new();
    timestamps.extend(
        manifest
            .last_updated_at
            .as_deref()
            .and_then(parse_rfc3339)
            .into_iter(),
    );
    timestamps.extend(
        manifest
            .started_at
            .as_deref()
            .and_then(parse_rfc3339)
            .into_iter(),
    );
    timestamps.extend(file_modified_utc(&run_dir.join("manifest.json")).into_iter());

    let Ok(entries) = fs::read_dir(run_dir) else {
        return timestamps.into_iter().max();
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.starts_with("attempt-") || !path.is_dir() {
            continue;
        }
        let Ok(attempt_entries) = fs::read_dir(&path) else {
            continue;
        };
        for artifact in attempt_entries.flatten() {
            let artifact_path = artifact.path();
            if artifact_path.is_file() {
                timestamps.extend(file_modified_utc(&artifact_path).into_iter());
            }
        }
    }
    timestamps.into_iter().max()
}

fn file_modified_utc(path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(DateTime::<Utc>::from)
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::write_json_pretty;
    use crate::types::{CampaignManifest, RunManifest};
    use filetime::{FileTime, set_file_mtime};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[test]
    fn stale_running_run_is_reconciled_to_failed() {
        let dir = tempdir().unwrap();
        let campaign_dir = dir.path().join("campaign");
        let run_dir = campaign_dir.join("runs").join("cohort").join("instance");
        let attempt_dir = run_dir.join("attempt-01");
        fs::create_dir_all(&attempt_dir).unwrap();

        let manifest = CampaignManifest {
            schema_version: "v1".into(),
            campaign_id: "campaign".into(),
            campaign_status: "running".into(),
            experiment_id: "exp".into(),
            experiment_name: "exp".into(),
            created_at: Utc::now().to_rfc3339(),
            campaign_root: campaign_dir.clone(),
            repo_cache_root: campaign_dir.join("_cache"),
            benchmark_name: "bench".into(),
            benchmark_adapter: "swebench".into(),
            preset_name: "preset".into(),
            preset_path: campaign_dir.join("preset.json"),
            stage_name: None,
            probe_profile: "probe".into(),
            report_profile: "report".into(),
            model: "gpt-5.4".into(),
            provider: "openai".into(),
            personality_mode: None,
            prompt_style: None,
            comparison_axes: Vec::new(),
            cohorts: Vec::new(),
            seed: "seed".into(),
            sample_size: 1,
            study_mode: "study".into(),
            max_parallel_runs: 1,
            per_repo_prepare_parallelism: 1,
            run_timeout_seconds: 2700,
            idle_timeout_seconds: 600,
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
            selected_instances: Vec::new(),
        };
        write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest).unwrap();

        let old = (Utc::now() - chrono::Duration::minutes(45)).to_rfc3339();
        let run_manifest = RunManifest {
            schema_version: "v1".into(),
            campaign_id: "campaign".into(),
            experiment_id: "exp".into(),
            experiment_name: "exp".into(),
            run_id: "run".into(),
            instance_id: "instance".into(),
            repo: "repo".into(),
            task_class: "search-heavy".into(),
            paired_instance_key: "instance".into(),
            cohort_id: "cohort".into(),
            model: "gpt-5.4".into(),
            provider: "openai".into(),
            personality_mode: None,
            prompt_style: None,
            base_commit: "abc".into(),
            worktree_dir: run_dir.join("workspace"),
            attempt: 1,
            status: "running".into(),
            started_at: Some(old.clone()),
            last_updated_at: Some(old),
            completed_at: None,
            derivations_status: "pending".into(),
            evidence_status: "pending".into(),
            grading_status: "pending".into(),
            failure_reason: None,
            failure_class: None,
            artifact_paths: BTreeMap::new(),
        };
        write_json_pretty(&run_dir.join("manifest.json"), &run_manifest).unwrap();
        let raw_events = attempt_dir.join("raw-agent-events.jsonl");
        fs::write(&raw_events, "{\"x\":1}\n").unwrap();
        let old_file_time = FileTime::from_unix_time((Utc::now() - chrono::Duration::minutes(45)).timestamp(), 0);
        set_file_mtime(&raw_events, old_file_time).unwrap();
        set_file_mtime(&run_dir.join("manifest.json"), old_file_time).unwrap();

        let report = reconcile_campaign_state_with_threshold(&campaign_dir, 1).unwrap();
        assert_eq!(report.stale_runs_fixed, 1);

        let reconciled: RunManifest = read_json(&run_dir.join("manifest.json")).unwrap();
        assert_eq!(reconciled.status, "failed");
        assert_eq!(reconciled.grading_status, "grader_not_run");
        assert_eq!(reconciled.failure_class.as_deref(), Some("interrupted"));

        let campaign: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json")).unwrap();
        assert_eq!(campaign.campaign_status, "run_completed");
    }
}
