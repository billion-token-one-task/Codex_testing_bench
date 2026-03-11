use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use codex_bench_codex::{run_codex_task, write_architecture_map};
use codex_bench_core::{
    CampaignManifest, CodexRunRequest, DatasetRecord, PrepareCampaignArgs, RunManifest, SelectedInstance,
    attempt_artifact_paths, default_swebench_preset_path, ensure_absolute_dir, load_study_preset,
    preferred_python, read_json, write_json_pretty,
};
use codex_bench_probes::{derive_run_outputs, write_claim_catalog_assets};
use codex_bench_report::render_run_evidence;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use tokio::process::Command;

pub const SCHEMA_VERSION: &str = "codex-bench.v1";
pub const STUDY_MODE: &str = "codex_live_observation";
pub const TOKEN_BUDGET_DOC: &str = "/Users/kevinlin/Downloads/Token预算实验_完整分析报告_v3.docx";
pub const SCHEDULER_DOC: &str =
    "/Users/kevinlin/Downloads/TokenMartCC/docs/papers/2026-03-09-单任务十亿级token调度架构初论.md";
pub const DEEPWIKI_DOC: &str = "https://deepwiki.com/openai/codex";
pub const OPENAI_HARNESS_DOC: &str = "https://openai.com/index/unlocking-the-codex-harness/";

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

    let mut selected_instances = Vec::new();
    for record in &selected_records {
        let run_dir = campaign_dir.join("runs").join(&record.instance_id);
        fs::create_dir_all(&run_dir)?;
        write_json_pretty(&run_dir.join("record.json"), record)?;
        selected_instances.push(SelectedInstance {
            instance_id: record.instance_id.clone(),
            repo: record.repo.clone(),
            task_class: classify_task(record),
            run_dir,
        });
    }

    let manifest = CampaignManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        campaign_id: campaign_id.clone(),
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
        seed: args.seed,
        sample_size: selected_instances.len(),
        study_mode: STUDY_MODE.to_string(),
        required_task_classes: preset.required_task_classes.clone(),
        preferred_task_classes: preset.preferred_task_classes.clone(),
        future_benchmarks: preset.future_benchmarks.clone(),
        grounding_documents: vec![TOKEN_BUDGET_DOC.to_string(), SCHEDULER_DOC.to_string()],
        reference_documents: vec![DEEPWIKI_DOC.to_string(), OPENAI_HARNESS_DOC.to_string()],
        selected_instances,
    };

    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;
    write_json_pretty(&campaign_dir.join("selected-dataset.json"), &selected_records)?;
    write_architecture_map(&campaign_dir)?;
    write_claim_catalog_assets(
        &campaign_dir,
        Path::new(TOKEN_BUDGET_DOC),
        Path::new(SCHEDULER_DOC),
    )?;
    Ok(campaign_dir)
}

pub async fn run_campaign(campaign_dir: &Path, refresh_repo_cache: bool) -> Result<()> {
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    for selected in &manifest.selected_instances {
        run_instance(&manifest, selected, refresh_repo_cache).await?;
    }
    write_predictions_jsonl(&manifest, campaign_dir).await?;
    Ok(())
}

pub async fn warm_repo_cache(campaign_dir: &Path, refresh_repo_cache: bool) -> Result<()> {
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    for selected in &manifest.selected_instances {
        let record: DatasetRecord = read_json(&selected.run_dir.join("record.json"))?;
        ensure_repo_commit_cached(&manifest.repo_cache_root, &record, refresh_repo_cache).await?;
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
        let mut repos = BTreeSet::new();
        for selected in &manifest.selected_instances {
            let record: DatasetRecord = read_json(&selected.run_dir.join("record.json"))?;
            ensure_repo_commit_cached(&manifest.repo_cache_root, &record, refresh_repo_cache).await?;
            warmed_instances += 1;
            repos.insert(record.repo);
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
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    let predictions_path = campaign_dir.join("predictions.jsonl");
    if !predictions_path.exists() {
        write_predictions_jsonl(&manifest, campaign_dir).await?;
    }

    let reports_dir = campaign_dir.join("reports");
    fs::create_dir_all(&reports_dir)?;
    let grader_path = reports_dir.join("grader.json");

    if let Some(command_template) = command {
        let command = command_template.replace("{predictions}", &predictions_path.display().to_string());
        let output = Command::new("zsh").arg("-lc").arg(command).output().await?;
        write_json_pretty(
            &grader_path,
            &json!({
                "status": if output.status.success() { "ok" } else { "failed" },
                "exitCode": output.status.code(),
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }),
        )?;
    } else {
        write_json_pretty(
            &grader_path,
            &json!({
                "status": "not_run",
                "message": "No grading command was provided. Supply --command with a {predictions} placeholder to invoke the official SWE-bench harness locally."
            }),
        )?;
    }
    Ok(())
}

async fn run_instance(
    manifest: &CampaignManifest,
    selected: &SelectedInstance,
    refresh_repo_cache: bool,
) -> Result<()> {
    let run_dir = &selected.run_dir;
    let attempt_dir = run_dir.join("attempt-01");
    fs::create_dir_all(&attempt_dir)?;

    let record: DatasetRecord = read_json(&run_dir.join("record.json"))?;
    let worktree_dir = run_dir.join("workspace");
    prepare_repo_workspace(
        &manifest.repo_cache_root,
        &record,
        &worktree_dir,
        refresh_repo_cache,
    )
    .await?;

    let prompt = build_prompt(&record);
    fs::write(attempt_dir.join("prompt.txt"), &prompt)?;
    write_json_pretty(
        &attempt_dir.join("environment-plan.json"),
        &json!({
            "repo": record.repo,
            "baseCommit": record.base_commit,
            "environmentSetupCommit": record.environment_setup_commit,
            "worktreeDir": worktree_dir,
            "taskClass": selected.task_class,
            "requestedModel": manifest.model,
            "requestedProvider": manifest.provider,
            "groundingDocuments": manifest.grounding_documents,
            "referenceDocuments": manifest.reference_documents,
        }),
    )?;

    let run_id = format!("{}-attempt-01", record.instance_id);
    let mut artifact_paths = attempt_artifact_paths(&attempt_dir);
    let run_manifest = RunManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        campaign_id: manifest.campaign_id.clone(),
        run_id: run_id.clone(),
        instance_id: record.instance_id.clone(),
        repo: record.repo.clone(),
        task_class: selected.task_class.clone(),
        base_commit: record.base_commit.clone(),
        worktree_dir: worktree_dir.clone(),
        attempt: 1,
        status: "running".to_string(),
        grading_status: "pending".to_string(),
        artifact_paths: artifact_paths.clone(),
    };
    write_json_pretty(&run_dir.join("manifest.json"), &run_manifest)?;

    let capture = run_codex_task(CodexRunRequest {
        model: manifest.model.clone(),
        provider: manifest.provider.clone(),
        run_id: run_id.clone(),
        repo: record.repo.clone(),
        instance_id: record.instance_id.clone(),
        task_class: selected.task_class.clone(),
        prompt,
        worktree_dir: worktree_dir.clone(),
        attempt_dir: attempt_dir.clone(),
        approval_never: true,
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

    for (name, path) in attempt_artifact_paths(&attempt_dir) {
        artifact_paths.insert(name, path);
    }
    let finished_manifest = RunManifest {
        status: summary.status.clone(),
        artifact_paths,
        ..run_manifest
    };
    write_json_pretty(&run_dir.join("manifest.json"), &finished_manifest)?;
    Ok(())
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

fn build_prompt(record: &DatasetRecord) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are solving a SWE-bench task inside a local git worktree.\n");
    prompt.push_str("Investigate the repository, make the minimal correct code changes, and stop when the patch is ready.\n");
    prompt.push_str("Do not produce a narrative essay. Use tools, inspect files, edit code, and verify when useful.\n\n");
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
    repo_root.join(".local-cache").join("repos").join("swebench")
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
    let fail_to_pass =
        normalize_test_list(object.remove("FAIL_TO_PASS").or_else(|| object.remove("fail_to_pass")));
    let pass_to_pass =
        normalize_test_list(object.remove("PASS_TO_PASS").or_else(|| object.remove("pass_to_pass")));
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
    let predictions_path = campaign_dir.join("predictions.jsonl");
    let mut lines = Vec::new();
    for selected in &manifest.selected_instances {
        let record: DatasetRecord = read_json(&selected.run_dir.join("record.json"))?;
        let patch_path = selected.run_dir.join("attempt-01").join("patch.diff");
        if !patch_path.exists() {
            continue;
        }
        let patch = fs::read_to_string(patch_path)?;
        lines.push(serde_json::to_string(&json!({
            "instance_id": record.instance_id,
            "model_name_or_path": format!("{}:{}", manifest.provider, manifest.model),
            "model_patch": patch,
        }))?);
    }
    fs::write(predictions_path, lines.join("\n"))?;
    Ok(())
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
