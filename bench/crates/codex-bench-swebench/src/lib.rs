use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use codex_bench_codex::{run_codex_task, write_architecture_map};
use codex_bench_core::{
    BenchmarkResearchProfile, BenchmarkTaskClassProfile, CampaignManifest, CodexRunRequest,
    DatasetRecord, ExperimentCohort, PrepareCampaignArgs, RunManifest, RunSummary,
    SelectedInstance, StudyCohortPreset, attempt_artifact_paths, default_swebench_preset_path,
    ensure_absolute_dir, load_study_preset, preferred_python, read_json,
    reconcile_campaign_state, write_json_pretty, write_jsonl,
};
use codex_bench_probes::{derive_run_outputs, write_claim_catalog_assets};
use codex_bench_report::{render_campaign_report, render_run_evidence};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

pub const SCHEMA_VERSION: &str = "codex-bench.v1";
pub const STUDY_MODE: &str = "codex_live_observation";
pub const TOKEN_BUDGET_DOC: &str = "/Users/kevinlin/Downloads/Token预算实验_完整分析报告_v3.docx";
pub const SCHEDULER_DOC: &str =
    "/Users/kevinlin/Downloads/TokenMartCC/docs/papers/2026-03-09-单任务十亿级token调度架构初论.md";
pub const DEEPWIKI_DOC: &str = "https://deepwiki.com/openai/codex";
pub const OPENAI_HARNESS_DOC: &str = "https://openai.com/index/unlocking-the-codex-harness/";
pub const MODEL_BEHAVIOR_HYPOTHESES: &str = "studies/hypotheses/model-behavior-v1.json";

pub async fn prepare_campaign(args: PrepareCampaignArgs) -> Result<PathBuf> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let preset_path = args
        .preset_path
        .clone()
        .unwrap_or_else(|| default_swebench_preset_path(&repo_root));
    let preset = load_study_preset(&preset_path)?;
    let (stage_name, sample_size) =
        preset.resolve_stage(args.stage.as_deref(), args.sample_size)?;

    let campaign_root = ensure_absolute_dir(&args.campaign_root)?;
    let campaign_id = format!(
        "swebench-study-{}-{}",
        Utc::now().format("%Y-%m-%dT%H-%M-%SZ"),
        short_hash(&format!("{}:{sample_size}", args.seed))
    );
    let campaign_dir = campaign_root.join(&campaign_id);
    fs::create_dir_all(campaign_dir.join("reports"))?;
    fs::create_dir_all(campaign_dir.join("runs"))?;
    let repo_cache_root = args
        .repo_cache_root
        .clone()
        .map(|path| ensure_absolute_dir(&path))
        .transpose()?
        .unwrap_or_else(|| default_shared_repo_cache_root(&repo_root));
    fs::create_dir_all(&repo_cache_root)?;

    if args.dataset_jsonl.is_none() && preset.benchmark_adapter != "swebench" {
        bail!(
            "preset `{}` uses adapter `{}` and therefore requires --dataset-jsonl with repo-patch task records",
            preset.name,
            preset.benchmark_adapter
        );
    }

    let mut dataset = load_dataset_records(
        &repo_root,
        args.dataset_jsonl.as_deref(),
        preset.benchmark_adapter == "swebench",
    )
    .await?;
    dataset.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    let selected_records = select_records(
        &dataset,
        &args.seed,
        sample_size,
        &preset.required_task_classes,
        &preset.preferred_task_classes,
    );

    let experiment_name = args
        .experiment_name
        .clone()
        .or_else(|| preset.experiment_name.clone())
        .unwrap_or_else(|| format!("{} model behavior study", preset.name));
    let experiment_id = format!(
        "exp-{}-{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        short_hash(&format!("{}:{}", experiment_name, args.seed))
    );
    let cohorts = resolve_cohorts(&preset, &args);
    let model_catalog_snapshot_path = campaign_dir.join("model-catalog-snapshot.json");
    let experiment_lock_path = campaign_dir.join("experiment-lock.json");
    let benchmark_research_profile_path = campaign_dir.join("benchmark-research-profile.json");
    let hypothesis_catalog_path = repo_root.join(MODEL_BEHAVIOR_HYPOTHESES);
    write_model_catalog_snapshot(&repo_root, &cohorts, &model_catalog_snapshot_path)?;
    write_json_pretty(
        &experiment_lock_path,
        &json!({
            "experimentId": experiment_id,
            "experimentName": experiment_name,
            "seed": args.seed,
            "benchmark": preset.benchmark,
            "benchmarkAdapter": preset.benchmark_adapter,
            "cohorts": cohorts,
            "webSearchPolicy": "disabled",
            "sandboxPolicy": "workspace-write-no-network",
            "studyMode": STUDY_MODE,
        }),
    )?;

    let mut selected_instances = Vec::new();
    for cohort in &cohorts {
        for record in &selected_records {
            let run_dir = campaign_dir
                .join("runs")
                .join(&cohort.cohort_id)
                .join(&record.instance_id);
            fs::create_dir_all(&run_dir)?;
            write_json_pretty(&run_dir.join("record.json"), record)?;
            selected_instances.push(SelectedInstance {
                instance_id: record.instance_id.clone(),
                repo: record.repo.clone(),
                task_class: classify_task(record),
                run_dir,
                paired_instance_key: record.instance_id.clone(),
                cohort_id: cohort.cohort_id.clone(),
                model: cohort.model.clone(),
                provider: cohort.provider.clone(),
                personality_mode: cohort.personality_mode.clone(),
                prompt_style: cohort.prompt_style.clone(),
            });
        }
    }

    let manifest = CampaignManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        campaign_id: campaign_id.clone(),
        campaign_status: "prepared".to_string(),
        experiment_id,
        experiment_name,
        created_at: Utc::now().to_rfc3339(),
        campaign_root: campaign_dir.clone(),
        repo_cache_root,
        benchmark_name: preset.benchmark.clone(),
        benchmark_adapter: preset.benchmark_adapter.clone(),
        preset_name: preset.name.clone(),
        preset_path,
        stage_name,
        probe_profile: preset.probe_profile.clone(),
        report_profile: preset.report_profile.clone(),
        model: args.model,
        provider: args.provider,
        personality_mode: args.personality.clone(),
        prompt_style: args.prompt_style.clone(),
        comparison_axes: if preset.comparison_axes.is_empty() {
            vec!["model".to_string(), "personality".to_string()]
        } else {
            preset.comparison_axes.clone()
        },
        cohorts,
        seed: args.seed,
        sample_size: selected_records.len(),
        study_mode: STUDY_MODE.to_string(),
        max_parallel_runs: args
            .max_parallel_runs
            .unwrap_or(preset.max_parallel_runs)
            .max(1),
        per_repo_prepare_parallelism: args
            .per_repo_prepare_parallelism
            .unwrap_or(preset.per_repo_prepare_parallelism)
            .max(1),
        run_timeout_seconds: preset.run_timeout_seconds,
        idle_timeout_seconds: preset.idle_timeout_seconds,
        required_task_classes: preset.required_task_classes.clone(),
        preferred_task_classes: preset.preferred_task_classes.clone(),
        future_benchmarks: preset.future_benchmarks.clone(),
        grounding_documents: vec![TOKEN_BUDGET_DOC.to_string(), SCHEDULER_DOC.to_string()],
        reference_documents: vec![DEEPWIKI_DOC.to_string(), OPENAI_HARNESS_DOC.to_string()],
        model_catalog_snapshot_path: Some(model_catalog_snapshot_path),
        hypothesis_catalog_path: Some(hypothesis_catalog_path),
        experiment_lock_path: Some(experiment_lock_path),
        benchmark_research_profile_path: Some(benchmark_research_profile_path.clone()),
        last_report_path: None,
        last_report_generated_at: None,
        selected_instances,
    };

    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;
    write_json_pretty(
        &campaign_dir.join("selected-dataset.json"),
        &selected_records,
    )?;
    write_json_pretty(
        &benchmark_research_profile_path,
        &swebench_benchmark_research_profile(),
    )?;
    write_architecture_map(&campaign_dir)?;
    write_claim_catalog_assets(
        &campaign_dir,
        Path::new(TOKEN_BUDGET_DOC),
        Path::new(SCHEDULER_DOC),
    )?;
    Ok(campaign_dir)
}

fn swebench_benchmark_research_profile() -> BenchmarkResearchProfile {
    BenchmarkResearchProfile {
        benchmark_name: "SWE-bench Verified".to_string(),
        benchmark_adapter: "swebench".to_string(),
        summary: "Repository-grounded software maintenance benchmark with strong verification pressure, moderate-to-high bootstrap risk, and rich tool-mediated debugging/edit/test loops.".to_string(),
        benchmark_notes: vec![
            "SWE-bench is suitable for observing Codex search-edit-verify cycles under realistic repository state.".to_string(),
            "Official grading failures must be separated from solver behavior because environment reconstruction cost is often large.".to_string(),
            "Different repos induce very different bootstrap tax and context pressure, so task-class-aware analysis is mandatory.".to_string(),
        ],
        task_class_profiles: vec![
            BenchmarkTaskClassProfile {
                task_class: "bootstrap-heavy".to_string(),
                expected_verification_strength: "high".to_string(),
                expected_context_pressure: "medium".to_string(),
                expected_tool_mix: vec!["shell".to_string(), "apply_patch".to_string()],
                expected_bootstrap_risk: "high".to_string(),
                expected_language_need: "medium".to_string(),
                language_profile_hint: Some("Expect more environment diagnosis, dependency narration, and confidence hedging around setup steps.".to_string()),
                tool_profile_hint: Some("Shell-heavy with repeated environment inspection, install, and test invocations.".to_string()),
                interaction_style_hint: Some("Likely to expose harness overhead and control-rod behavior before productive edits stabilize.".to_string()),
                default_analysis_overrides: BTreeMap::from([
                    ("prioritize_bootstrap_tax".to_string(), "true".to_string()),
                    ("interpret_grader_failures_as_solver_failures".to_string(), "false".to_string()),
                ]),
            },
            BenchmarkTaskClassProfile {
                task_class: "verification-heavy".to_string(),
                expected_verification_strength: "very_high".to_string(),
                expected_context_pressure: "medium".to_string(),
                expected_tool_mix: vec!["shell".to_string(), "apply_patch".to_string()],
                expected_bootstrap_risk: "medium".to_string(),
                expected_language_need: "high".to_string(),
                language_profile_hint: Some("Expect verification framing, confidence claims, and result interpretation language to rise.".to_string()),
                tool_profile_hint: Some("Frequent test commands and alternating patch/test micro-cycles.".to_string()),
                interaction_style_hint: Some("Best regime for measuring language-to-verification closure and actionable narration.".to_string()),
                default_analysis_overrides: BTreeMap::from([
                    ("prioritize_verification_grounded_commentary".to_string(), "true".to_string()),
                ]),
            },
            BenchmarkTaskClassProfile {
                task_class: "search-heavy".to_string(),
                expected_verification_strength: "medium".to_string(),
                expected_context_pressure: "medium".to_string(),
                expected_tool_mix: vec!["shell".to_string()],
                expected_bootstrap_risk: "low".to_string(),
                expected_language_need: "high".to_string(),
                language_profile_hint: Some("Expect orientation, observation, and decision-explanation language around repo exploration.".to_string()),
                tool_profile_hint: Some("High grep/find/read density before the first controlled change.".to_string()),
                interaction_style_hint: Some("Useful for testing whether models narrate exploration or remain silent operators.".to_string()),
                default_analysis_overrides: BTreeMap::from([
                    ("prioritize_activation_threshold".to_string(), "true".to_string()),
                ]),
            },
            BenchmarkTaskClassProfile {
                task_class: "patch-heavy".to_string(),
                expected_verification_strength: "medium".to_string(),
                expected_context_pressure: "medium".to_string(),
                expected_tool_mix: vec!["apply_patch".to_string(), "shell".to_string()],
                expected_bootstrap_risk: "low".to_string(),
                expected_language_need: "medium".to_string(),
                language_profile_hint: Some("Expect decision explanation and result framing around retained edits.".to_string()),
                tool_profile_hint: Some("Patch application becomes the key controlled-change signal.".to_string()),
                interaction_style_hint: Some("Good regime for comparing silent patch bursts versus narrated patch bursts.".to_string()),
                default_analysis_overrides: BTreeMap::from([
                    ("prioritize_patch_bursts".to_string(), "true".to_string()),
                ]),
            },
            BenchmarkTaskClassProfile {
                task_class: "compaction-likely".to_string(),
                expected_verification_strength: "medium".to_string(),
                expected_context_pressure: "high".to_string(),
                expected_tool_mix: vec!["shell".to_string(), "apply_patch".to_string()],
                expected_bootstrap_risk: "medium".to_string(),
                expected_language_need: "high".to_string(),
                language_profile_hint: Some("Expect recap, bridge language, and rediscovery signs if state continuity degrades.".to_string()),
                tool_profile_hint: Some("Long search/edit/test sequences with higher chance of repeated reads or verification loops.".to_string()),
                interaction_style_hint: Some("Most relevant class for compaction continuity and persistence half-life probes.".to_string()),
                default_analysis_overrides: BTreeMap::from([
                    ("prioritize_compaction_continuity".to_string(), "true".to_string()),
                ]),
            },
        ],
    }
}

pub async fn run_campaign(campaign_dir: &Path, refresh_repo_cache: bool) -> Result<()> {
    let _ = reconcile_campaign_state(campaign_dir);
    let mut manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    manifest.campaign_status = "running".to_string();
    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;
    let max_parallel_runs = manifest.max_parallel_runs.max(1);
    let run_limiter = Arc::new(Semaphore::new(max_parallel_runs));
    let repo_prepare_limiters = build_repo_prepare_limiters(&manifest);
    let manifest_arc = Arc::new(manifest.clone());
    let mut join_set = JoinSet::new();
    for selected in manifest.selected_instances.clone() {
        let permit = run_limiter.clone().acquire_owned().await?;
        let manifest = Arc::clone(&manifest_arc);
        let repo_prepare_limiter = repo_prepare_limiters
            .get(&selected.repo)
            .cloned()
            .unwrap_or_else(|| Arc::new(Semaphore::new(1)));
        join_set.spawn(async move {
            let _permit = permit;
            run_instance(manifest, selected, refresh_repo_cache, repo_prepare_limiter).await
        });
    }
    let mut failure_count = 0usize;
    while let Some(joined) = join_set.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                failure_count += 1;
                eprintln!("swebench run instance failed: {error:#}");
            }
            Err(error) => {
                failure_count += 1;
                eprintln!("swebench join task failed: {error:#}");
            }
        }
    }
    write_predictions_jsonl(manifest_arc.as_ref(), campaign_dir).await?;
    manifest.campaign_status = "run_completed".to_string();
    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;
    let report_path = render_campaign_report(campaign_dir)?;
    manifest.campaign_status = "report_generated".to_string();
    manifest.last_report_path = Some(report_path);
    manifest.last_report_generated_at = Some(Utc::now().to_rfc3339());
    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;
    if failure_count > 0 {
        eprintln!("swebench campaign completed with {failure_count} failed run(s)");
    }
    Ok(())
}

pub async fn warm_repo_cache(campaign_dir: &Path, refresh_repo_cache: bool) -> Result<()> {
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    let limiter = Arc::new(Semaphore::new(manifest.max_parallel_runs.max(1)));
    let unique_records = load_unique_repo_records(&manifest)?;
    let mut join_set = JoinSet::new();
    for record in unique_records {
        let repo_cache_root = manifest.repo_cache_root.clone();
        let permit = limiter.clone().acquire_owned().await?;
        join_set.spawn(async move {
            let _permit = permit;
            ensure_repo_commit_cached(&repo_cache_root, &record, refresh_repo_cache).await
        });
    }
    while let Some(joined) = join_set.join_next().await {
        joined??;
    }
    Ok(())
}

pub async fn bootstrap_local_assets(
    repo_root: &Path,
    campaign_dir: Option<&Path>,
    refresh_repo_cache: bool,
) -> Result<Value> {
    let dataset_snapshot_path = hydrate_local_dataset_snapshot(repo_root).await?;
    let shared_repo_cache_root = default_shared_repo_cache_root(repo_root);
    fs::create_dir_all(&shared_repo_cache_root)?;

    let mut warmed_instances = 0usize;
    let mut warmed_repos = Vec::new();
    if let Some(campaign_dir) = campaign_dir {
        let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
        let limiter = Arc::new(Semaphore::new(manifest.max_parallel_runs.max(1)));
        let unique_records = load_unique_repo_records(&manifest)?;
        warmed_instances = unique_records.len();
        let mut join_set = JoinSet::new();
        for record in unique_records {
            let repo_cache_root = manifest.repo_cache_root.clone();
            let repo_name = record.repo.clone();
            let permit = limiter.clone().acquire_owned().await?;
            join_set.spawn(async move {
                let _permit = permit;
                ensure_repo_commit_cached(&repo_cache_root, &record, refresh_repo_cache)
                    .await
                    .map(|_| repo_name)
            });
        }
        let mut repos = BTreeSet::new();
        while let Some(joined) = join_set.join_next().await {
            repos.insert(joined??);
        }
        warmed_repos.extend(repos);
    }

    Ok(json!({
        "datasetSnapshotPath": dataset_snapshot_path,
        "sharedRepoCacheRoot": shared_repo_cache_root,
        "warmedInstances": warmed_instances,
        "warmedRepos": warmed_repos,
    }))
}

fn build_repo_prepare_limiters(manifest: &CampaignManifest) -> BTreeMap<String, Arc<Semaphore>> {
    let per_repo_parallelism = manifest.per_repo_prepare_parallelism.max(1);
    manifest
        .selected_instances
        .iter()
        .map(|selected| {
            (
                selected.repo.clone(),
                Arc::new(Semaphore::new(per_repo_parallelism)),
            )
        })
        .collect()
}

fn load_unique_repo_records(manifest: &CampaignManifest) -> Result<Vec<DatasetRecord>> {
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    for selected in &manifest.selected_instances {
        let record: DatasetRecord = read_json(&selected.run_dir.join("record.json"))?;
        let key = (
            record.repo.clone(),
            record.base_commit.clone(),
            record.environment_setup_commit.clone(),
        );
        if seen.insert(key) {
            records.push(record);
        }
    }
    Ok(records)
}

pub async fn hydrate_local_dataset_snapshot(repo_root: &Path) -> Result<PathBuf> {
    let snapshot_path = default_local_dataset_snapshot_path(repo_root);
    if snapshot_path.exists() {
        return Ok(snapshot_path);
    }
    if let Some(parent) = snapshot_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let script = r#"
import json
from datasets import load_dataset
dataset = load_dataset("princeton-nlp/SWE-bench_Verified", split="test")
for row in dataset:
    print(json.dumps(row, ensure_ascii=False))
"#;
    let output = Command::new(preferred_python())
        .arg("-c")
        .arg(script)
        .output()
        .await
        .context("failed to invoke configured python for SWE-bench dataset snapshot hydration")?;
    if !output.status.success() {
        bail!(
            "dataset snapshot fetch failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::write(&snapshot_path, &output.stdout)?;
    Ok(snapshot_path)
}

pub async fn grade_campaign(campaign_dir: &Path, command: Option<String>) -> Result<()> {
    let _ = reconcile_campaign_state(campaign_dir);
    let mut manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    let predictions_path = campaign_dir.join("predictions.jsonl");
    if !predictions_path.exists() {
        write_predictions_jsonl(&manifest, campaign_dir).await?;
    }

    let reports_dir = campaign_dir.join("reports");
    fs::create_dir_all(&reports_dir)?;
    let grader_path = reports_dir.join("grader.json");
    manifest.campaign_status = "grading".to_string();
    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;

    if let Some(command_template) = command {
        let predictions_dir = campaign_dir.join("predictions");
        let official_root = reports_dir.join("official");
        fs::create_dir_all(&official_root)?;
        let cohort_ids = if manifest.cohorts.is_empty() {
            vec!["default".to_string()]
        } else {
            manifest
                .cohorts
                .iter()
                .map(|cohort| cohort.cohort_id.clone())
                .collect::<Vec<_>>()
        };
        let mut cohort_results = Vec::new();
        for cohort_id in cohort_ids {
            let cohort_predictions_path = predictions_dir.join(format!("{cohort_id}.jsonl"));
            if !cohort_predictions_path.exists() {
                continue;
            }
            let cohort_official_dir = official_root.join(&cohort_id);
            fs::create_dir_all(&cohort_official_dir)?;
            let command = command_template.replace(
                "{predictions}",
                &cohort_predictions_path.display().to_string(),
            );
            let output = Command::new("zsh")
                .arg("-lc")
                .arg(command)
                .current_dir(&cohort_official_dir)
                .output()
                .await?;
            let cohort_grader_path = cohort_official_dir.join("grader.json");
            write_json_pretty(
                &cohort_grader_path,
                &json!({
                    "cohortId": cohort_id,
                    "status": if output.status.success() { "ok" } else { "failed" },
                    "exitCode": output.status.code(),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                }),
            )?;
            let official_summary_path = find_official_summary_path(&cohort_official_dir)?;
            if let Some(summary_path) = official_summary_path.as_ref() {
                ingest_official_grading(&manifest, campaign_dir, &cohort_id, summary_path)?;
            }
            cohort_results.push(json!({
                "cohortId": cohort_id,
                "graderPath": cohort_grader_path,
                "officialSummaryPath": official_summary_path,
                "status": if output.status.success() { "ok" } else { "failed" },
                "exitCode": output.status.code(),
            }));
        }
        write_json_pretty(
            &grader_path,
            &json!({
                "status": "completed",
                "cohorts": cohort_results,
            }),
        )?;
        manifest.campaign_status = "graded".to_string();
    } else {
        write_json_pretty(
            &grader_path,
            &json!({
                "status": "not_run",
                "message": "No grading command was provided. Supply --command with a {predictions} placeholder to invoke the official SWE-bench harness locally."
            }),
        )?;
        manifest.campaign_status = "run_completed".to_string();
    }
    let report_path = render_campaign_report(campaign_dir)?;
    manifest.last_report_path = Some(report_path);
    manifest.last_report_generated_at = Some(Utc::now().to_rfc3339());
    manifest.campaign_status = if manifest.campaign_status == "graded" {
        "graded_report_generated".to_string()
    } else {
        "report_generated".to_string()
    };
    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;
    Ok(())
}

fn ingest_official_grading(
    manifest: &CampaignManifest,
    campaign_dir: &Path,
    cohort_id: &str,
    official_summary_path: &Path,
) -> Result<()> {
    let official_summary: Value = read_json(&official_summary_path)?;

    let resolved_ids = value_string_set(&official_summary, "resolved_ids");
    let unresolved_ids = value_string_set(&official_summary, "unresolved_ids");
    let error_ids = value_string_set(&official_summary, "error_ids");
    let completed_ids = value_string_set(&official_summary, "completed_ids");

    for selected in &manifest.selected_instances {
        if selected.cohort_id != cohort_id {
            continue;
        }
        let grading_status = if resolved_ids.contains(&selected.instance_id) {
            "resolved"
        } else if unresolved_ids.contains(&selected.instance_id) {
            "unresolved"
        } else if error_ids.contains(&selected.instance_id) {
            "error"
        } else if completed_ids.contains(&selected.instance_id) {
            "completed"
        } else {
            "pending"
        };

        let run_manifest_path = selected.run_dir.join("manifest.json");
        if run_manifest_path.exists() {
            let mut run_manifest: RunManifest = read_json(&run_manifest_path)?;
            run_manifest.grading_status = grading_status.to_string();
            run_manifest.evidence_status = "run_evidence_generated".to_string();
            write_json_pretty(&run_manifest_path, &run_manifest)?;
        }

        let attempt_dir = selected.run_dir.join("attempt-01");
        let summary_path = attempt_dir.join("run-summary.json");
        if summary_path.exists() {
            let mut summary: RunSummary = read_json(&summary_path)?;
            summary.grading_status = grading_status.to_string();
            write_json_pretty(&summary_path, &summary)?;
        }

        let per_instance_report =
            find_official_instance_report(campaign_dir, manifest, &selected.instance_id);
        let grade_row = json!({
            "instanceId": selected.instance_id,
            "cohortId": selected.cohort_id,
            "gradingStatus": grading_status,
            "officialSummaryPath": official_summary_path,
            "officialInstanceReportPath": per_instance_report,
            "classification": "exact",
        });
        write_jsonl(&attempt_dir.join("grade-events.jsonl"), &[grade_row])?;
    }

    Ok(())
}

fn find_official_summary_path(official_dir: &Path) -> Result<Option<PathBuf>> {
    if !official_dir.exists() {
        return Ok(None);
    }
    let mut candidates = fs::read_dir(&official_dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    candidates.sort();
    Ok(candidates.pop())
}

fn find_official_instance_report(
    campaign_dir: &Path,
    manifest: &CampaignManifest,
    instance_id: &str,
) -> Option<PathBuf> {
    let model_dir = manifest.model.replace('/', "__");
    let candidate_roots = [
        campaign_dir.join("logs").join("run_evaluation"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join("bench")
            .join("logs")
            .join("run_evaluation"),
    ];
    for root in candidate_roots {
        let path = root
            .join(&manifest.campaign_id)
            .join(&model_dir)
            .join(instance_id)
            .join("report.json");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn value_string_set(value: &Value, key: &str) -> BTreeSet<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

async fn run_instance(
    manifest: Arc<CampaignManifest>,
    selected: SelectedInstance,
    refresh_repo_cache: bool,
    repo_prepare_limiter: Arc<Semaphore>,
) -> Result<()> {
    let run_dir = &selected.run_dir;
    let attempt_dir = run_dir.join("attempt-01");
    fs::create_dir_all(&attempt_dir)?;

    let record: DatasetRecord = read_json(&run_dir.join("record.json"))?;
    let worktree_dir = run_dir.join("workspace");
    {
        let _permit = repo_prepare_limiter.acquire().await?;
        prepare_repo_workspace(
            &manifest.repo_cache_root,
            &record,
            &worktree_dir,
            refresh_repo_cache,
        )
        .await?;
    }

    let prompt = build_prompt(
        &record,
        selected.personality_mode.as_deref(),
        selected.prompt_style.as_deref(),
    );
    fs::write(attempt_dir.join("prompt.txt"), &prompt)?;
    write_json_pretty(
        &attempt_dir.join("environment-plan.json"),
        &json!({
            "repo": record.repo,
            "baseCommit": record.base_commit,
            "environmentSetupCommit": record.environment_setup_commit,
            "worktreeDir": worktree_dir,
            "taskClass": selected.task_class,
            "requestedModel": selected.model,
            "requestedProvider": selected.provider,
            "requestedPersonality": selected.personality_mode,
            "promptStyle": selected.prompt_style,
            "cohortId": selected.cohort_id,
            "groundingDocuments": manifest.grounding_documents,
            "referenceDocuments": manifest.reference_documents,
        }),
    )?;

    let run_id = format!("{}-attempt-01", record.instance_id);
    let mut artifact_paths = attempt_artifact_paths(&attempt_dir);
    let started_at = Utc::now().to_rfc3339();
    let run_manifest = RunManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        campaign_id: manifest.campaign_id.clone(),
        experiment_id: manifest.experiment_id.clone(),
        experiment_name: manifest.experiment_name.clone(),
        run_id: run_id.clone(),
        instance_id: record.instance_id.clone(),
        repo: record.repo.clone(),
        task_class: selected.task_class.clone(),
        paired_instance_key: selected.paired_instance_key.clone(),
        cohort_id: selected.cohort_id.clone(),
        model: selected.model.clone(),
        provider: selected.provider.clone(),
        personality_mode: selected.personality_mode.clone(),
        prompt_style: selected.prompt_style.clone(),
        base_commit: record.base_commit.clone(),
        worktree_dir: worktree_dir.clone(),
        attempt: 1,
        status: "running".to_string(),
        started_at: Some(started_at.clone()),
        last_updated_at: Some(started_at.clone()),
        completed_at: None,
        derivations_status: "pending".to_string(),
        evidence_status: "pending".to_string(),
        grading_status: "pending".to_string(),
        failure_reason: None,
        failure_class: None,
        artifact_paths: artifact_paths.clone(),
    };
    write_json_pretty(&run_dir.join("manifest.json"), &run_manifest)?;
    let execution = async {
        let capture = run_codex_task(CodexRunRequest {
            model: selected.model.clone(),
            provider: selected.provider.clone(),
            personality_mode: selected.personality_mode.clone(),
            prompt_style: selected.prompt_style.clone(),
            cohort_id: Some(selected.cohort_id.clone()),
            run_id: run_id.clone(),
            repo: record.repo.clone(),
            instance_id: record.instance_id.clone(),
            task_class: selected.task_class.clone(),
            prompt,
            worktree_dir: worktree_dir.clone(),
            attempt_dir: attempt_dir.clone(),
            approval_never: true,
            run_timeout_seconds: Some(manifest.run_timeout_seconds),
            idle_timeout_seconds: Some(manifest.idle_timeout_seconds),
        })
        .await?;

        let patch = command_capture(
            Command::new("git")
                .arg("-C")
                .arg(&worktree_dir)
                .arg("diff")
                .arg("--binary")
                .arg("--no-ext-diff"),
        )
        .await?;
        fs::write(attempt_dir.join("patch.diff"), &patch.stdout)?;

        let summary = derive_run_outputs(
            &attempt_dir,
            &run_id,
            &selected.task_class,
            &record,
            &capture.decoded_events,
            &capture.probe_events,
            &capture.raw_diagnostics,
            &patch.stdout,
        )?;
        render_run_evidence(&attempt_dir, &record, &summary)?;
        Ok::<_, anyhow::Error>(summary)
    }
    .await;

    for (name, path) in attempt_artifact_paths(&attempt_dir) {
        artifact_paths.insert(name, path);
    }

    match execution {
        Ok(summary) => {
            let finished_manifest = RunManifest {
                status: summary.status.clone(),
                last_updated_at: Some(Utc::now().to_rfc3339()),
                completed_at: Some(Utc::now().to_rfc3339()),
                derivations_status: "completed".to_string(),
                evidence_status: "run_evidence_generated".to_string(),
                artifact_paths,
                ..run_manifest
            };
            write_json_pretty(&run_dir.join("manifest.json"), &finished_manifest)?;
            Ok(())
        }
        Err(error) => {
            fs::write(attempt_dir.join("run-error.txt"), format!("{error:#}\n"))?;
            let failure_reason = error.to_string();
            let failure_class = classify_run_failure(&failure_reason).to_string();
            let failed_manifest = RunManifest {
                status: "failed".to_string(),
                last_updated_at: Some(Utc::now().to_rfc3339()),
                completed_at: Some(Utc::now().to_rfc3339()),
                derivations_status: if attempt_dir.join("probe-summary.json").exists() {
                    "completed".to_string()
                } else {
                    "failed".to_string()
                },
                evidence_status: if attempt_dir.join("run-evidence.txt").exists() {
                    "run_evidence_generated".to_string()
                } else {
                    "pending".to_string()
                },
                failure_reason: Some(failure_reason),
                failure_class: Some(failure_class),
                artifact_paths,
                ..run_manifest
            };
            write_json_pretty(&run_dir.join("manifest.json"), &failed_manifest)?;
            Err(error)
        }
    }
}

fn classify_run_failure(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("idle timeout") || lower.contains("run timeout") {
        "timeout"
    } else if lower.contains("web_search_begin") {
        "policy_violation"
    } else if lower.contains("prepare_repo_workspace")
        || lower.contains("git worktree")
        || lower.contains("failed to fetch commit")
    {
        "workspace_prepare"
    } else if lower.contains("failed to invoke configured python")
        || lower.contains("dataset snapshot fetch failed")
    {
        "bootstrap_env"
    } else if lower.contains("patch") {
        "patch_capture"
    } else {
        "solver_runtime"
    }
}

async fn prepare_repo_workspace(
    repo_cache_root: &Path,
    record: &DatasetRecord,
    worktree_dir: &Path,
    refresh: bool,
) -> Result<()> {
    let repo_cache = ensure_repo_commit_cached(repo_cache_root, record, refresh).await?;

    if worktree_dir.exists() {
        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_cache)
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(worktree_dir)
            .output()
            .await;
        fs::remove_dir_all(worktree_dir)?;
    }
    if let Some(parent) = worktree_dir.parent() {
        fs::create_dir_all(parent)?;
    }

    run_command(
        Command::new("git")
            .arg("-C")
            .arg(&repo_cache)
            .arg("worktree")
            .arg("add")
            .arg("--force")
            .arg("--detach")
            .arg(worktree_dir)
            .arg(format!("refs/codex-bench/{}", record.base_commit)),
    )
    .await?;
    Ok(())
}

async fn ensure_repo_commit_cached(
    repo_cache_root: &Path,
    record: &DatasetRecord,
    refresh: bool,
) -> Result<PathBuf> {
    let repo_cache = repo_cache_root.join(record.repo.replace('/', "__"));
    if !repo_cache.exists() {
        fs::create_dir_all(&repo_cache)?;
        run_command(Command::new("git").arg("init").arg(&repo_cache))
            .await
            .with_context(|| format!("failed to initialize repo cache for {}", record.repo))?;
        run_command(
            Command::new("git")
                .arg("-C")
                .arg(&repo_cache)
                .arg("remote")
                .arg("add")
                .arg("origin")
                .arg(format!("https://github.com/{}.git", record.repo)),
        )
        .await?;
    } else if refresh {
        run_command(
            Command::new("git")
                .arg("-C")
                .arg(&repo_cache)
                .arg("remote")
                .arg("set-url")
                .arg("origin")
                .arg(format!("https://github.com/{}.git", record.repo)),
        )
        .await?;
    }

    let bench_ref = format!("refs/codex-bench/{}", record.base_commit);
    let has_commit = has_commit(&repo_cache, &record.base_commit).await?;
    if !has_commit {
        run_command(
            Command::new("git")
                .arg("-C")
                .arg(&repo_cache)
                .arg("fetch")
                .arg("--filter=blob:none")
                .arg("--depth")
                .arg("1")
                .arg("origin")
                .arg(&record.base_commit),
        )
        .await
        .with_context(|| {
            format!(
                "failed to fetch commit {} for {}",
                record.base_commit, record.repo
            )
        })?;
        run_command(
            Command::new("git")
                .arg("-C")
                .arg(&repo_cache)
                .arg("update-ref")
                .arg(&bench_ref)
                .arg("FETCH_HEAD"),
        )
        .await?;
    } else if !has_ref(&repo_cache, &bench_ref).await? {
        run_command(
            Command::new("git")
                .arg("-C")
                .arg(&repo_cache)
                .arg("update-ref")
                .arg(&bench_ref)
                .arg(&record.base_commit),
        )
        .await?;
    }
    Ok(repo_cache)
}

async fn has_commit(repo_cache: &Path, commit: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_cache)
        .arg("cat-file")
        .arg("-e")
        .arg(format!("{commit}^{{commit}}"))
        .output()
        .await?;
    Ok(output.status.success())
}

async fn has_ref(repo_cache: &Path, git_ref: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_cache)
        .arg("show-ref")
        .arg("--verify")
        .arg("--quiet")
        .arg(git_ref)
        .output()
        .await?;
    Ok(output.status.success())
}

fn build_prompt(
    record: &DatasetRecord,
    personality_mode: Option<&str>,
    prompt_style: Option<&str>,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are solving a SWE-bench task inside a local git worktree.\n");
    prompt.push_str("Investigate the repository, make the minimal correct code changes, and stop when the patch is ready.\n");
    prompt.push_str("Do not produce a narrative essay. Use tools, inspect files, edit code, and verify when useful.\n\n");
    if let Some(personality_mode) = personality_mode {
        prompt.push_str("Requested personality mode: ");
        prompt.push_str(personality_mode);
        prompt.push('\n');
    }
    if let Some(prompt_style) = prompt_style {
        prompt.push_str("Prompt-style control: ");
        prompt.push_str(prompt_style);
        prompt.push('\n');
        match prompt_style {
            "terse_engineer" => {
                prompt.push_str("Use terse engineering updates and minimize extra phrasing unless it helps execution.\n\n");
            }
            "warm_collaborative" => {
                prompt.push_str("Use collaborative, warm bridge language when explaining actions or results.\n\n");
            }
            "structured_checklist" => {
                prompt.push_str("Prefer short structured checklists and explicit next steps.\n\n");
            }
            "research_exploratory" => {
                prompt.push_str("Expose intermediate hypotheses, observations, and verification framing more explicitly.\n\n");
            }
            _ => {
                prompt.push_str("Follow the style request while staying task-focused.\n\n");
            }
        }
    }
    prompt.push_str("Problem statement:\n");
    prompt.push_str(&record.problem_statement);
    prompt.push_str("\n\nRepository: ");
    prompt.push_str(&record.repo);
    prompt.push_str("\nInstance: ");
    prompt.push_str(&record.instance_id);
    if let Some(version) = &record.version {
        prompt.push_str("\nVersion: ");
        prompt.push_str(version);
    }
    if let Some(hints) = &record.hints_text {
        prompt.push_str("\n\nHints:\n");
        prompt.push_str(hints);
    }
    if !record.fail_to_pass.is_empty() {
        prompt.push_str("\n\nFail-to-pass tests:\n");
        for test in &record.fail_to_pass {
            prompt.push_str("- ");
            prompt.push_str(test);
            prompt.push('\n');
        }
    }
    if !record.pass_to_pass.is_empty() {
        prompt.push_str("\nPass-to-pass tests:\n");
        for test in &record.pass_to_pass {
            prompt.push_str("- ");
            prompt.push_str(test);
            prompt.push('\n');
        }
    }
    prompt
}

async fn load_dataset_records(
    repo_root: &Path,
    dataset_jsonl: Option<&Path>,
    allow_hf_swebench_fetch: bool,
) -> Result<Vec<DatasetRecord>> {
    if let Some(path) = dataset_jsonl {
        return parse_dataset_jsonl(path);
    }
    if !allow_hf_swebench_fetch {
        bail!("dataset_jsonl is required for non-SWE-bench adapters");
    }
    let snapshot_path = default_local_dataset_snapshot_path(repo_root);
    let path = if snapshot_path.exists() {
        snapshot_path
    } else {
        hydrate_local_dataset_snapshot(repo_root).await?
    };
    parse_dataset_jsonl(&path)
}

pub fn default_shared_repo_cache_root(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".local-cache")
        .join("repos")
        .join("swebench")
}

pub fn default_local_dataset_snapshot_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join("vendor-benchmarks")
        .join("swebench-verified")
        .join("verified-test.jsonl")
}

fn parse_dataset_jsonl(path: &Path) -> Result<Vec<DatasetRecord>> {
    parse_dataset_bytes(&fs::read(path)?)
}

fn parse_dataset_bytes(bytes: &[u8]) -> Result<Vec<DatasetRecord>> {
    let mut records = Vec::new();
    for line in String::from_utf8_lossy(bytes)
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let value: Value = serde_json::from_str(line)?;
        records.push(normalize_record(value)?);
    }
    Ok(records)
}

fn normalize_record(value: Value) -> Result<DatasetRecord> {
    let mut raw = value;
    let object = raw
        .as_object_mut()
        .ok_or_else(|| anyhow!("dataset row was not an object"))?;
    let fail_to_pass = normalize_test_list(
        object
            .remove("FAIL_TO_PASS")
            .or_else(|| object.remove("fail_to_pass")),
    );
    let pass_to_pass = normalize_test_list(
        object
            .remove("PASS_TO_PASS")
            .or_else(|| object.remove("pass_to_pass")),
    );
    Ok(DatasetRecord {
        instance_id: string_field(object, "instance_id")?,
        repo: string_field(object, "repo")?,
        base_commit: string_field(object, "base_commit")?,
        patch: optional_string(object, "patch"),
        test_patch: optional_string(object, "test_patch"),
        problem_statement: string_field(object, "problem_statement").unwrap_or_default(),
        hints_text: optional_string(object, "hints_text"),
        version: optional_string(object, "version"),
        environment_setup_commit: optional_string(object, "environment_setup_commit"),
        fail_to_pass,
        pass_to_pass,
        raw,
    })
}

fn normalize_test_list(value: Option<Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(ToOwned::to_owned))
            .collect(),
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.starts_with('[') {
                serde_json::from_str::<Vec<String>>(trimmed)
                    .unwrap_or_else(|_| vec![trimmed.to_string()])
            } else {
                trimmed
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

fn string_field(object: &Map<String, Value>, key: &str) -> Result<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("missing `{key}`"))
}

fn optional_string(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn select_records(
    dataset: &[DatasetRecord],
    seed: &str,
    sample_size: usize,
    required_task_classes: &[String],
    preferred_task_classes: &[String],
) -> Vec<DatasetRecord> {
    let mut decorated = dataset
        .iter()
        .map(|record| {
            let class = classify_task(record);
            let score = short_hash(&format!("{seed}:{}:{class}", record.instance_id));
            (class, score, record.clone())
        })
        .collect::<Vec<_>>();
    decorated.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.instance_id.cmp(&b.2.instance_id))
    });

    let mut picked = Vec::new();
    let mut seen_classes = BTreeSet::new();
    for required in required_task_classes {
        if picked.len() >= sample_size {
            break;
        }
        if let Some((class, _, record)) = decorated.iter().find(|(class, _, record)| {
            class == required
                && !picked
                    .iter()
                    .any(|existing: &DatasetRecord| existing.instance_id == record.instance_id)
        }) {
            seen_classes.insert(class.clone());
            picked.push(record.clone());
        }
    }
    for preferred in preferred_task_classes {
        if picked.len() >= sample_size {
            break;
        }
        if let Some((class, _, record)) = decorated.iter().find(|(class, _, record)| {
            class == preferred
                && !picked
                    .iter()
                    .any(|existing: &DatasetRecord| existing.instance_id == record.instance_id)
        }) {
            seen_classes.insert(class.clone());
            picked.push(record.clone());
        }
    }
    for (class, _, record) in &decorated {
        if picked.len() >= sample_size {
            break;
        }
        if seen_classes.insert(class.clone()) {
            picked.push(record.clone());
        }
    }
    for (_, _, record) in decorated {
        if picked.len() >= sample_size {
            break;
        }
        if !picked
            .iter()
            .any(|existing| existing.instance_id == record.instance_id)
        {
            picked.push(record);
        }
    }
    picked
}

pub fn classify_task(record: &DatasetRecord) -> String {
    let text = record.problem_statement.to_ascii_lowercase();
    if text.contains("build") || text.contains("dependency") || text.contains("install") {
        "bootstrap-heavy".to_string()
    } else if record.fail_to_pass.len() > 3 {
        "verification-heavy".to_string()
    } else if text.contains("parser") || text.contains("search") || text.contains("find") {
        "search-heavy".to_string()
    } else if text.len() > 2_000 {
        "compaction-likely".to_string()
    } else {
        "patch-heavy".to_string()
    }
}

fn short_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..8].to_string()
}

async fn run_command(cmd: &mut Command) -> Result<()> {
    let output = cmd.output().await?;
    if !output.status.success() {
        bail!(
            "command failed: {}\nstdout:\n{}\nstderr:\n{}",
            render_command(cmd),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

async fn command_capture(cmd: &mut Command) -> Result<std::process::Output> {
    let output = cmd.output().await?;
    if !output.status.success() {
        bail!(
            "command failed: {}\nstdout:\n{}\nstderr:\n{}",
            render_command(cmd),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}

fn render_command(cmd: &Command) -> String {
    let program = cmd.as_std().get_program().to_string_lossy().to_string();
    let args = cmd
        .as_std()
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    std::iter::once(program)
        .chain(args)
        .collect::<Vec<_>>()
        .join(" ")
}

async fn write_predictions_jsonl(manifest: &CampaignManifest, campaign_dir: &Path) -> Result<()> {
    let predictions_dir = campaign_dir.join("predictions");
    fs::create_dir_all(&predictions_dir)?;
    let mut combined_lines = Vec::new();
    let mut per_cohort_lines = BTreeMap::<String, Vec<String>>::new();
    for selected in &manifest.selected_instances {
        let record: DatasetRecord = read_json(&selected.run_dir.join("record.json"))?;
        let patch_path = selected.run_dir.join("attempt-01").join("patch.diff");
        if !patch_path.exists() {
            continue;
        }
        let patch = fs::read_to_string(patch_path)?;
        let row = serde_json::to_string(&json!({
            "instance_id": record.instance_id,
            "paired_instance_key": selected.paired_instance_key,
            "cohort_id": selected.cohort_id,
            "model_name_or_path": format!("{}:{}", selected.provider, selected.model),
            "personality_mode": selected.personality_mode,
            "prompt_style": selected.prompt_style,
            "model_patch": patch,
        }))?;
        combined_lines.push(row.clone());
        per_cohort_lines
            .entry(selected.cohort_id.clone())
            .or_default()
            .push(row);
    }
    fs::write(
        campaign_dir.join("predictions.jsonl"),
        combined_lines.join("\n"),
    )?;
    for (cohort_id, lines) in per_cohort_lines {
        fs::write(
            predictions_dir.join(format!("{cohort_id}.jsonl")),
            lines.join("\n"),
        )?;
    }
    Ok(())
}

fn resolve_cohorts(
    preset: &codex_bench_core::StudyPreset,
    args: &PrepareCampaignArgs,
) -> Vec<ExperimentCohort> {
    if !preset.cohorts.is_empty() {
        return preset
            .cohorts
            .iter()
            .map(|cohort| map_cohort(cohort))
            .collect();
    }
    vec![ExperimentCohort {
        cohort_id: "default".to_string(),
        label: format!(
            "{} / {}{}",
            args.model,
            args.provider,
            args.personality
                .as_ref()
                .map(|personality| format!(" / {personality}"))
                .unwrap_or_default()
        ),
        model: args.model.clone(),
        provider: args.provider.clone(),
        personality_mode: args.personality.clone(),
        prompt_style: args.prompt_style.clone(),
    }]
}

fn map_cohort(cohort: &StudyCohortPreset) -> ExperimentCohort {
    ExperimentCohort {
        cohort_id: cohort.id.clone(),
        label: cohort.label.clone(),
        model: cohort.model.clone(),
        provider: cohort.provider.clone(),
        personality_mode: cohort.personality.clone(),
        prompt_style: cohort.prompt_style.clone(),
    }
}

fn write_model_catalog_snapshot(
    repo_root: &Path,
    cohorts: &[ExperimentCohort],
    output_path: &Path,
) -> Result<()> {
    let models_path = repo_root
        .join("repos")
        .join("codex")
        .join("codex-rs")
        .join("core")
        .join("models.json");
    let models: Value = read_json(&models_path)?;
    let requested_models = cohorts
        .iter()
        .map(|cohort| cohort.model.clone())
        .collect::<BTreeSet<_>>();
    let filtered = models
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(model, _)| requested_models.contains(*model))
                .map(|(model, value)| (model.clone(), value.clone()))
                .collect::<Map<String, Value>>()
        })
        .unwrap_or_default();
    write_json_pretty(
        output_path,
        &json!({
            "source": models_path,
            "requestedModels": requested_models,
            "models": filtered,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_test_list_parses_json_string_lists() {
        let list = normalize_test_list(Some(Value::String(
            "[\"a::test_one\", \"a::test_two\"]".to_string(),
        )));
        assert_eq!(list, vec!["a::test_one", "a::test_two"]);
    }

    #[test]
    fn classify_task_marks_long_records_as_compaction_likely() {
        let record = DatasetRecord {
            instance_id: "demo__repo-1".to_string(),
            repo: "demo/repo".to_string(),
            base_commit: "abc".to_string(),
            patch: None,
            test_patch: None,
            problem_statement: "x".repeat(2200),
            hints_text: None,
            version: None,
            environment_setup_commit: None,
            fail_to_pass: Vec::new(),
            pass_to_pass: Vec::new(),
            raw: Value::Null,
        };
        assert_eq!(classify_task(&record), "compaction-likely");
    }
}
