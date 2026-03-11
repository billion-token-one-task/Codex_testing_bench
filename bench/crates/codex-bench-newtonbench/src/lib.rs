use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use chrono::Utc;
use codex_bench_codex::{run_codex_task, write_architecture_map};
use codex_bench_core::{
    CampaignManifest, CodexRunRequest, DatasetRecord, PrepareCampaignArgs, RunManifest,
    SelectedInstance, attempt_artifact_paths, command_capture, ensure_absolute_dir, git_commit_all,
    init_git_workspace, preferred_python, read_json, reset_dir, write_json_pretty,
};
use codex_bench_probes::{derive_run_outputs, write_claim_catalog_assets};
use codex_bench_report::render_run_evidence;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::process::Command;

pub const SCHEMA_VERSION: &str = "codex-bench.v1";
pub const STUDY_MODE: &str = "codex_live_observation";
pub const BENCHMARK_NAME: &str = "NewtonBench";
pub const BENCHMARK_ADAPTER: &str = "newtonbench";
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
        .unwrap_or_else(|| repo_root.join("studies").join("task-presets").join("newtonbench-v0.json"));
    let preset = codex_bench_core::load_study_preset(&preset_path)?;
    let (stage_name, sample_size) = preset.resolve_stage(args.stage.as_deref(), args.sample_size)?;

    let campaign_root = ensure_absolute_dir(&args.campaign_root)?;
    let campaign_id = format!(
        "newtonbench-study-{}-{}",
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
        .unwrap_or_else(|| campaign_dir.join("_repo-cache"));
    fs::create_dir_all(&repo_cache_root)?;

    let vendor_root = repo_root.join("vendor-benchmarks").join("NewtonBench");
    let mut dataset = load_newtonbench_records(&vendor_root)?;
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
            paired_instance_key: record.instance_id.clone(),
            cohort_id: "default".to_string(),
            model: args.model.clone(),
            provider: args.provider.clone(),
            personality_mode: args.personality.clone(),
            prompt_style: args.prompt_style.clone(),
        });
    }

    let manifest = CampaignManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        campaign_id: campaign_id.clone(),
        experiment_id: format!("exp-{}", short_hash(&format!("{}:{}", BENCHMARK_NAME, args.seed))),
        experiment_name: args
            .experiment_name
            .clone()
            .unwrap_or_else(|| BENCHMARK_NAME.to_string()),
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
        model: args.model.clone(),
        provider: args.provider.clone(),
        personality_mode: args.personality.clone(),
        prompt_style: args.prompt_style.clone(),
        comparison_axes: vec!["model".to_string(), "personality".to_string()],
        cohorts: vec![codex_bench_core::ExperimentCohort {
            cohort_id: "default".to_string(),
            label: BENCHMARK_NAME.to_string(),
            model: args.model.clone(),
            provider: args.provider.clone(),
            personality_mode: args.personality.clone(),
            prompt_style: args.prompt_style.clone(),
        }],
        seed: args.seed,
        sample_size: selected_instances.len(),
        study_mode: STUDY_MODE.to_string(),
        required_task_classes: preset.required_task_classes.clone(),
        preferred_task_classes: preset.preferred_task_classes.clone(),
        future_benchmarks: preset.future_benchmarks.clone(),
        grounding_documents: vec![TOKEN_BUDGET_DOC.to_string(), SCHEDULER_DOC.to_string()],
        reference_documents: vec![DEEPWIKI_DOC.to_string(), OPENAI_HARNESS_DOC.to_string()],
        model_catalog_snapshot_path: None,
        hypothesis_catalog_path: None,
        experiment_lock_path: None,
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
    let mut exact_pass = 0usize;
    let mut numeric_pass = 0usize;

    for selected in &manifest.selected_instances {
        let run_dir = &selected.run_dir;
        let attempt_dir = run_dir.join("attempt-01");
        let workspace_dir = run_dir.join("workspace");

        let output = Command::new(preferred_python())
            .arg("tools/newton_lab.py")
            .arg("eval")
            .arg("--submission-file")
            .arg("submission.py")
            .current_dir(&workspace_dir)
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let payload: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|_| {
            json!({
                "error": "failed_to_parse_evaluator_output",
                "stdout": stdout,
                "stderr": stderr,
            })
        });
        let exact = payload
            .get("exact_accuracy")
            .and_then(Value::as_f64)
            .unwrap_or_default();
        let rmsle = payload.get("rmsle").and_then(Value::as_f64);
        let numeric_ok = rmsle.map(|value| value.is_finite() && value < 1e-6).unwrap_or(false);
        let grading_status = if exact >= 1.0 {
            exact_pass += 1;
            "graded_pass_exact"
        } else if numeric_ok {
            numeric_pass += 1;
            "graded_pass_numeric"
        } else {
            "graded_fail"
        };

        codex_bench_core::write_jsonl(
            &attempt_dir.join("grade-events.jsonl"),
            &[json!({
                "command": "python3 tools/newton_lab.py eval --submission-file submission.py",
                "exitCode": output.status.code(),
                "status": grading_status,
                "payload": payload,
                "stdout": stdout,
                "stderr": stderr,
            })],
        )?;
        update_run_manifest_grading(run_dir, grading_status)?;
        rows.push(json!({
            "instanceId": selected.instance_id,
            "taskClass": selected.task_class,
            "status": grading_status,
            "metrics": payload,
        }));
    }

    write_json_pretty(
        &campaign_dir.join("reports").join("grader.json"),
        &json!({
            "benchmark": BENCHMARK_NAME,
            "adapter": BENCHMARK_ADAPTER,
            "exactPass": exact_pass,
            "numericPass": numeric_pass,
            "failed": manifest.selected_instances.len().saturating_sub(exact_pass + numeric_pass),
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
            "module": raw_string(&record, "moduleName"),
            "difficulty": raw_string(&record, "difficulty"),
            "system": raw_string(&record, "system"),
            "lawVersion": raw_string(&record, "lawVersion"),
            "worktreeDir": workspace_dir,
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
        worktree_dir: workspace_dir.clone(),
        attempt: 1,
        status: "running".to_string(),
        grading_status: "pending".to_string(),
        artifact_paths: artifact_paths.clone(),
    };
    write_json_pretty(&run_dir.join("manifest.json"), &run_manifest)?;

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

    let vendor_root = PathBuf::from(
        raw_string(record, "vendorRoot")
            .ok_or_else(|| anyhow!("record missing vendorRoot"))?,
    );
    let module_name = raw_string(record, "moduleName")
        .ok_or_else(|| anyhow!("record missing moduleName"))?;
    let difficulty = raw_string(record, "difficulty")
        .ok_or_else(|| anyhow!("record missing difficulty"))?;
    let system = raw_string(record, "system")
        .ok_or_else(|| anyhow!("record missing system"))?;
    let law_version = raw_string(record, "lawVersion");

    let meta = fetch_module_metadata(&vendor_root, &module_name, &difficulty, &system, law_version.as_deref()).await?;
    let meta_dir = workspace_dir.join(".bench-meta");
    let tools_dir = workspace_dir.join("tools");
    fs::create_dir_all(&meta_dir)?;
    fs::create_dir_all(&tools_dir)?;

    write_json_pretty(
        &meta_dir.join("task.json"),
        &json!({
            "vendorRoot": vendor_root,
            "moduleName": module_name,
            "difficulty": difficulty,
            "system": system,
            "lawVersion": law_version,
            "functionSignature": meta.function_signature,
            "paramDescription": meta.param_description,
            "taskPrompt": meta.task_prompt,
        }),
    )?;
    fs::write(
        workspace_dir.join("task.md"),
        format!(
            "# NewtonBench Task\n\nModule: {}\nDifficulty: {}\nSystem: {}\n\n## Discovery Brief\n\n{}\n\n## Function Signature\n\n{}\n\n## Parameter Description\n\n{}\n\n## Local Lab Commands\n\n- `python3 tools/newton_lab.py show-task`\n- `python3 tools/newton_lab.py run --params-json '{{\"param\": 1.0}}'`\n- `python3 tools/newton_lab.py eval --submission-file submission.py`\n",
            raw_string(record, "moduleName").unwrap_or_default(),
            raw_string(record, "difficulty").unwrap_or_default(),
            raw_string(record, "system").unwrap_or_default(),
            meta.task_prompt,
            meta.function_signature,
            meta.param_description,
        ),
    )?;
    fs::write(
        workspace_dir.join("submission.py"),
        format!("{}\n    raise NotImplementedError(\"discover the law\")\n", meta.function_signature),
    )?;
    fs::write(tools_dir.join("newton_lab.py"), render_newton_lab_script())?;
    fs::write(
        workspace_dir.join(".gitignore"),
        ".pytest_cache/\n__pycache__/\n*.pyc\n.venv/\n",
    )?;
    git_commit_all(workspace_dir, "bench: initialize NewtonBench task baseline").await?;
    Ok(())
}

fn build_prompt(record: &DatasetRecord) -> String {
    let mut prompt = String::new();
    let python_cmd = preferred_python().display().to_string();
    prompt.push_str("You are solving a NewtonBench scientific law discovery task.\n");
    prompt.push_str("Read ./task.md first. Use the local Newton lab helper to run experiments and validate your candidate law.\n");
    prompt.push_str("Write your final answer into ./submission.py as a valid discovered_law function.\n");
    prompt.push_str("You may create scratch scripts, but keep the final deliverable focused on submission.py and any minimal supporting files.\n\n");
    prompt.push_str("Task metadata:\n");
    prompt.push_str(&format!("- Module: {}\n", raw_string(record, "moduleName").unwrap_or_default()));
    prompt.push_str(&format!("- Difficulty: {}\n", raw_string(record, "difficulty").unwrap_or_default()));
    prompt.push_str(&format!("- System: {}\n", raw_string(record, "system").unwrap_or_default()));
    prompt.push_str(&format!(
        "\nUse `{python_cmd} tools/newton_lab.py show-task` to inspect the full benchmark task and `{python_cmd} tools/newton_lab.py run --params-json ...` to perform experiments.\n"
    ));
    prompt
}

fn load_newtonbench_records(vendor_root: &Path) -> Result<Vec<DatasetRecord>> {
    let modules_root = vendor_root.join("modules");
    let mut records = Vec::new();
    for entry in fs::read_dir(&modules_root)? {
        let entry = entry?;
        let module_dir = entry.path();
        let module_name = entry.file_name().to_string_lossy().to_string();
        if !module_dir.is_dir() || module_name == "common" || !module_name.starts_with('m') {
            continue;
        }
        for difficulty in ["easy", "medium", "hard"] {
            for system in ["vanilla_equation", "simple_system", "complex_system"] {
                let instance_id = format!("newtonbench__{}__{}__{}", module_name, difficulty, system);
                records.push(DatasetRecord {
                    instance_id,
                    repo: format!("newtonbench/{}", module_name),
                    base_commit: "LOCAL_SIMULATION".to_string(),
                    patch: None,
                    test_patch: None,
                    problem_statement: format!(
                        "Rediscover the scientific law in module `{}` under difficulty `{}` and system `{}` using interactive experimentation.",
                        module_name, difficulty, system
                    ),
                    hints_text: Some("Use the generated local Newton lab helper to run experiments and evaluate submission.py.".to_string()),
                    version: Some("v1".to_string()),
                    environment_setup_commit: None,
                    fail_to_pass: vec!["python3 tools/newton_lab.py eval --submission-file submission.py".to_string()],
                    pass_to_pass: Vec::new(),
                    raw: json!({
                        "benchmark": BENCHMARK_NAME,
                        "adapter": BENCHMARK_ADAPTER,
                        "vendorRoot": vendor_root,
                        "moduleName": module_name,
                        "difficulty": difficulty,
                        "system": system,
                        "lawVersion": Value::Null,
                    }),
                });
            }
        }
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
    let system = raw_string(record, "system").unwrap_or_default();
    let difficulty = raw_string(record, "difficulty").unwrap_or_default();
    if system == "complex_system" {
        "compaction-likely".to_string()
    } else if difficulty == "hard" {
        "failure-prone".to_string()
    } else if system == "simple_system" {
        "experimentation-heavy".to_string()
    } else {
        "scientific-equation".to_string()
    }
}

async fn fetch_module_metadata(
    vendor_root: &Path,
    module_name: &str,
    difficulty: &str,
    system: &str,
    law_version: Option<&str>,
) -> Result<NewtonTaskMetadata> {
    let law_version_arg = law_version.unwrap_or("");
    let script = r#"
import importlib
import json
import sys

vendor_root, module_name, difficulty, system, law_version = sys.argv[1:6]
sys.path.insert(0, vendor_root)
module = importlib.import_module(f"modules.{module_name}")
prompt = module.get_task_prompt(system, is_code_assisted=True, noise_level=0.0)
print(json.dumps({
    "functionSignature": getattr(module, "FUNCTION_SIGNATURE", ""),
    "paramDescription": getattr(module, "PARAM_DESCRIPTION", ""),
    "taskPrompt": prompt,
    "difficulty": difficulty,
    "system": system,
    "lawVersion": law_version or None,
}, ensure_ascii=False))
"#;
    let output = Command::new(preferred_python())
        .arg("-c")
        .arg(script)
        .arg(vendor_root)
        .arg(module_name)
        .arg(difficulty)
        .arg(system)
        .arg(law_version_arg)
        .output()
        .await?;
    if !output.status.success() {
        bail!(
            "failed to inspect NewtonBench module {}: {}",
            module_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let value: Value = serde_json::from_slice(&output.stdout)?;
    Ok(NewtonTaskMetadata {
        function_signature: value
            .get("functionSignature")
            .and_then(Value::as_str)
            .unwrap_or("def discovered_law(x):")
            .to_string(),
        param_description: value
            .get("paramDescription")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        task_prompt: value
            .get("taskPrompt")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

fn raw_string(record: &DatasetRecord, key: &str) -> Option<String> {
    record
        .raw
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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

fn render_newton_lab_script() -> String {
    r#"import argparse
import json
import os
import sys
from pathlib import Path


def load_task():
    task_path = Path(__file__).resolve().parent.parent / ".bench-meta" / "task.json"
    return json.loads(task_path.read_text())


def load_module(task):
    sys.path.insert(0, task["vendorRoot"])
    import importlib
    return importlib.import_module(f"modules.{task['moduleName']}")


def main():
    parser = argparse.ArgumentParser(description="Local NewtonBench lab helper")
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("show-task")
    run_parser = sub.add_parser("run")
    run_parser.add_argument("--params-json", required=True)
    eval_parser = sub.add_parser("eval")
    eval_parser.add_argument("--submission-file", required=True)
    args = parser.parse_args()

    task = load_task()
    module = load_module(task)

    if args.cmd == "show-task":
        print(json.dumps(task, ensure_ascii=False, indent=2))
        return

    if args.cmd == "run":
        params = json.loads(args.params_json)
        result = module.run_experiment_for_module(
            noise_level=0.0,
            difficulty=task["difficulty"],
            system=task["system"],
            law_version=task.get("lawVersion"),
            **params,
        )
        print(json.dumps(result, ensure_ascii=False))
        return

    if args.cmd == "eval":
        submission = Path(args.submission_file).read_text()
        result = module.evaluate_law(
            submission,
            param_description=task.get("paramDescription") or getattr(module, "PARAM_DESCRIPTION", ""),
            difficulty=task["difficulty"],
            law_version=task.get("lawVersion"),
            judge_model_name=os.environ.get("NEWTONBENCH_JUDGE_MODEL", "gpt41"),
            trial_info={"trial_id": "codex-bench"},
        )
        print(json.dumps(result, ensure_ascii=False))
        return


if __name__ == "__main__":
    main()
"#
    .to_string()
}

struct NewtonTaskMetadata {
    function_signature: String,
    param_description: String,
    task_prompt: String,
}
