use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use codex_bench_codex::{run_codex_task, write_architecture_map};
use codex_bench_core::{
    CampaignManifest, CodexRunRequest, DatasetRecord, PrepareCampaignArgs, RunManifest,
    SelectedInstance, attempt_artifact_paths, command_capture, ensure_absolute_dir, git_commit_all,
    init_git_workspace, read_json, reset_dir, write_json_pretty,
};
use codex_bench_probes::{derive_run_outputs, write_claim_catalog_assets};
use codex_bench_report::render_run_evidence;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::process::Command;

pub const SCHEMA_VERSION: &str = "codex-bench.v1";
pub const STUDY_MODE: &str = "codex_live_observation";
pub const BENCHMARK_NAME: &str = "NL2RepoBench";
pub const BENCHMARK_ADAPTER: &str = "nl2repo";
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
        .unwrap_or_else(|| repo_root.join("studies").join("task-presets").join("nl2repo-v0.json"));
    let preset = codex_bench_core::load_study_preset(&preset_path)?;
    let (stage_name, sample_size) = preset.resolve_stage(args.stage.as_deref(), args.sample_size)?;

    let campaign_root = ensure_absolute_dir(&args.campaign_root)?;
    let campaign_id = format!(
        "nl2repo-study-{}-{}",
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
        .unwrap_or_else(|| campaign_dir.join("_repo-cache"));
    fs::create_dir_all(&repo_cache_root)?;

    let vendor_root = repo_root.join("vendor-benchmarks").join("NL2RepoBench");
    let mut dataset = load_nl2repo_records(&vendor_root)?;
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
        benchmark_name: BENCHMARK_NAME.to_string(),
        benchmark_adapter: BENCHMARK_ADAPTER.to_string(),
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

pub async fn run_campaign(campaign_dir: &Path) -> Result<()> {
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    for selected in &manifest.selected_instances {
        run_instance(&manifest, selected).await?;
    }
    Ok(())
}

pub async fn grade_campaign(campaign_dir: &Path) -> Result<()> {
    let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
    let mut rows = Vec::<Value>::new();
    let mut passed = 0usize;

    for selected in &manifest.selected_instances {
        let run_dir = &selected.run_dir;
        let attempt_dir = run_dir.join("attempt-01");
        let workspace_dir = run_dir.join("workspace");
        let record: DatasetRecord = read_json(&run_dir.join("record.json"))?;
        let commands = raw_string_vec(&record, "testCommands");
        let expected_cases = record
            .raw
            .get("testCaseCount")
            .and_then(Value::as_i64)
            .unwrap_or_default();

        let mut event_rows = Vec::<Value>::new();
        let mut all_ok = true;
        for command in commands {
            let output = Command::new("zsh")
                .arg("-lc")
                .arg(&command)
                .current_dir(&workspace_dir)
                .output()
                .await?;
            let ok = output.status.success();
            all_ok &= ok;
            event_rows.push(json!({
                "command": command,
                "exitCode": output.status.code(),
                "ok": ok,
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }));
        }
        codex_bench_core::write_jsonl(&attempt_dir.join("grade-events.jsonl"), &event_rows)?;
        if all_ok {
            passed += 1;
        }
        let grading_status = if all_ok { "graded_pass" } else { "graded_fail" };
        update_run_manifest_grading(run_dir, grading_status)?;
        rows.push(json!({
            "instanceId": record.instance_id,
            "taskName": raw_string(&record, "taskName").unwrap_or_default(),
            "expectedCases": expected_cases,
            "status": grading_status,
            "commands": event_rows,
        }));
    }

    write_json_pretty(
        &campaign_dir.join("reports").join("grader.json"),
        &json!({
            "benchmark": BENCHMARK_NAME,
            "adapter": BENCHMARK_ADAPTER,
            "passed": passed,
            "failed": manifest.selected_instances.len().saturating_sub(passed),
            "results": rows,
        }),
    )?;
    Ok(())
}

async fn run_instance(manifest: &CampaignManifest, selected: &SelectedInstance) -> Result<()> {
    let run_dir = &selected.run_dir;
    let attempt_dir = run_dir.join("attempt-01");
    fs::create_dir_all(&attempt_dir)?;

    let record: DatasetRecord = read_json(&run_dir.join("record.json"))?;
    let workspace_dir = run_dir.join("workspace");
    prepare_workspace(&record, &workspace_dir).await?;

    let prompt = build_prompt(&record);
    fs::write(attempt_dir.join("prompt.txt"), &prompt)?;
    write_json_pretty(
        &attempt_dir.join("environment-plan.json"),
        &json!({
            "benchmark": BENCHMARK_NAME,
            "adapter": BENCHMARK_ADAPTER,
            "worktreeDir": workspace_dir,
            "taskClass": selected.task_class,
            "taskBundleDir": raw_string(&record, "taskBundleDir"),
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
        worktree_dir: workspace_dir.clone(),
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
        worktree_dir: workspace_dir.clone(),
        attempt_dir: attempt_dir.clone(),
        approval_never: true,
    })
    .await?;

    let patch = command_capture(
        Command::new("git")
            .arg("-C")
            .arg(&workspace_dir)
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

async fn prepare_workspace(record: &DatasetRecord, workspace_dir: &Path) -> Result<()> {
    reset_dir(workspace_dir)?;
    init_git_workspace(workspace_dir).await?;

    let bundle_dir = PathBuf::from(
        raw_string(record, "taskBundleDir")
            .ok_or_else(|| anyhow!("record missing taskBundleDir"))?,
    );
    let meta_dir = workspace_dir.join(".bench-meta");
    fs::create_dir_all(&meta_dir)?;
    fs::copy(bundle_dir.join("start.md"), workspace_dir.join("start.md"))?;
    fs::copy(
        bundle_dir.join("test_commands.json"),
        meta_dir.join("test_commands.json"),
    )?;
    fs::copy(
        bundle_dir.join("test_files.json"),
        meta_dir.join("test_files.json"),
    )?;
    fs::copy(
        bundle_dir.join("test_case_count.txt"),
        meta_dir.join("test_case_count.txt"),
    )?;
    fs::write(
        workspace_dir.join(".gitignore"),
        ".pytest_cache/\n__pycache__/\n*.pyc\n.venv/\n",
    )?;
    git_commit_all(workspace_dir, "bench: initialize NL2Repo task baseline").await?;
    Ok(())
}

fn build_prompt(record: &DatasetRecord) -> String {
    let test_commands = raw_string_vec(record, "testCommands");
    let test_files = raw_string_vec(record, "testFiles");
    let test_case_count = record
        .raw
        .get("testCaseCount")
        .and_then(Value::as_i64)
        .unwrap_or_default();

    let mut prompt = String::new();
    prompt.push_str("You are solving an NL2RepoBench task inside a blank git repository.\n");
    prompt.push_str("Read ./start.md and implement the entire project in the current directory.\n");
    prompt.push_str("The benchmark metadata lives in ./.bench-meta/.\n");
    prompt.push_str("Do not edit benchmark metadata files unless absolutely necessary.\n");
    prompt.push_str("Your goal is to make the repository runnable and pass the listed test commands.\n\n");
    prompt.push_str("Task bundle:\n");
    prompt.push_str(&format!(
        "- Task name: {}\n",
        raw_string(record, "taskName").unwrap_or_else(|| record.instance_id.clone())
    ));
    prompt.push_str(&format!("- Expected test cases: {}\n", test_case_count));
    if !test_files.is_empty() {
        prompt.push_str("- Expected test files:\n");
        for file in test_files {
            prompt.push_str(&format!("  - {}\n", file));
        }
    }
    if !test_commands.is_empty() {
        prompt.push_str("- Grade commands:\n");
        for command in test_commands {
            prompt.push_str(&format!("  - {}\n", command));
        }
    }
    prompt.push_str("\nStart by reading ./start.md, then create the repository files, install or declare dependencies as needed, and use the local environment to validate the build.\n");
    prompt
}

fn load_nl2repo_records(vendor_root: &Path) -> Result<Vec<DatasetRecord>> {
    let tasks_root = vendor_root.join("test_files");
    let mut records = Vec::new();
    for entry in fs::read_dir(&tasks_root)? {
        let entry = entry?;
        let task_dir = entry.path();
        if !task_dir.is_dir() {
            continue;
        }
        let task_name = entry.file_name().to_string_lossy().to_string();
        let start_md = fs::read_to_string(task_dir.join("start.md"))
            .with_context(|| format!("missing start.md for {task_name}"))?;
        let test_commands: Vec<String> =
            serde_json::from_slice(&fs::read(task_dir.join("test_commands.json"))?)?;
        let test_files: Vec<String> =
            serde_json::from_slice(&fs::read(task_dir.join("test_files.json"))?)?;
        let test_case_count = fs::read_to_string(task_dir.join("test_case_count.txt"))?
            .trim()
            .parse::<i64>()
            .unwrap_or_default();
        records.push(DatasetRecord {
            instance_id: format!("nl2repo__{}", task_name.replace('/', "__")),
            repo: format!("nl2repo/{}", task_name),
            base_commit: "BLANK_REPOSITORY".to_string(),
            patch: None,
            test_patch: None,
            problem_statement: start_md,
            hints_text: None,
            version: None,
            environment_setup_commit: None,
            fail_to_pass: test_commands.clone(),
            pass_to_pass: Vec::new(),
            raw: json!({
                "benchmark": BENCHMARK_NAME,
                "adapter": BENCHMARK_ADAPTER,
                "taskName": task_name,
                "taskBundleDir": task_dir,
                "testCommands": test_commands,
                "testFiles": test_files,
                "testCaseCount": test_case_count,
            }),
        });
    }
    Ok(records)
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
    let test_commands = raw_string_vec(record, "testCommands");
    let test_case_count = record
        .raw
        .get("testCaseCount")
        .and_then(Value::as_i64)
        .unwrap_or_default();

    if text.contains("docker")
        || text.contains("dependency")
        || text.contains("install")
        || test_commands.iter().any(|command| command.contains("pip install"))
    {
        "bootstrap-heavy".to_string()
    } else if text.len() > 12_000 {
        "compaction-likely".to_string()
    } else if test_case_count >= 30 || test_commands.len() > 1 {
        "verification-heavy".to_string()
    } else {
        "repo-generation".to_string()
    }
}

fn raw_string(record: &DatasetRecord, key: &str) -> Option<String> {
    record
        .raw
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn raw_string_vec(record: &DatasetRecord, key: &str) -> Vec<String> {
    record
        .raw
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn update_run_manifest_grading(run_dir: &Path, grading_status: &str) -> Result<()> {
    let mut manifest: RunManifest = read_json(&run_dir.join("manifest.json"))?;
    manifest.grading_status = grading_status.to_string();
    write_json_pretty(&run_dir.join("manifest.json"), &manifest)?;
    Ok(())
}

fn short_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())[..8].to_string()
}
