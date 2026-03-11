use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use codex_app_server_client::{
    InProcessAppServerClient, InProcessClientStartArgs, InProcessServerEvent,
};
use codex_app_server_protocol::{
    AskForApproval as AppServerAskForApproval, ClientRequest, ConfigWarningNotification,
    JSONRPCErrorError, JSONRPCNotification, RequestId, SandboxMode, SandboxPolicy, SessionSource,
    ThreadStartParams, ThreadStartResponse, TurnStartParams, TurnStartResponse, UserInput,
};
use codex_arg0::Arg0DispatchPaths;
use codex_core::config::{ConfigBuilder, ConfigOverrides};
use codex_core::config_loader::{CloudRequirementsLoader, LoaderOverrides};
use codex_feedback::CodexFeedback;
use codex_protocol::config_types::SandboxMode as CoreSandboxMode;
use codex_protocol::protocol::{
    AskForApproval, Event, EventMsg, StudyMetadata, StudyProbeEvent,
};
use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use toml::Value as TomlValue;

use crate::architecture::write_architecture_map;
use crate::claims::write_claim_catalogs;
use crate::report::{derive_run_outputs, render_run_evidence};
use crate::types::{CampaignManifest, DatasetRecord, RunManifest, SelectedInstance};

const SCHEMA_VERSION: &str = "codex-swebench-study.v2";
const STUDY_MODE: &str = "codex_live_observation";
const TOKEN_BUDGET_DOC: &str = "/Users/kevinlin/Downloads/Token预算实验_完整分析报告_v3.docx";
const SCHEDULER_DOC: &str =
    "/Users/kevinlin/Downloads/TokenMartCC/docs/papers/2026-03-09-单任务十亿级token调度架构初论.md";
const DEEPWIKI_DOC: &str = "https://deepwiki.com/openai/codex";
const OPENAI_HARNESS_DOC: &str = "https://openai.com/index/unlocking-the-codex-harness/";

#[derive(Debug)]
pub struct PrepareArgs {
    pub campaign_root: PathBuf,
    pub sample_size: usize,
    pub seed: String,
    pub dataset_jsonl: Option<PathBuf>,
    pub model: String,
    pub provider: String,
    pub repo_cache_root: Option<PathBuf>,
}

pub async fn prepare_campaign(args: PrepareArgs) -> Result<PathBuf> {
    fs::create_dir_all(&args.campaign_root)?;
    let campaign_id = format!(
        "swebench-study-{}-{}",
        Utc::now().format("%Y-%m-%dT%H-%M-%SZ"),
        short_hash(&format!("{}:{}", args.seed, args.sample_size))
    );
    let campaign_dir = args.campaign_root.join(&campaign_id);
    fs::create_dir_all(campaign_dir.join("reports"))?;
    fs::create_dir_all(campaign_dir.join("runs"))?;
    let repo_cache_root = args
        .repo_cache_root
        .unwrap_or_else(|| campaign_dir.join("_repo-cache"));
    fs::create_dir_all(&repo_cache_root)?;

    let mut dataset = load_dataset_records(args.dataset_jsonl.as_deref()).await?;
    dataset.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
    let selected_records = select_records(&dataset, &args.seed, args.sample_size);

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
        model: args.model,
        provider: args.provider,
        seed: args.seed,
        sample_size: selected_instances.len(),
        study_mode: STUDY_MODE.to_string(),
        grounding_documents: vec![TOKEN_BUDGET_DOC.to_string(), SCHEDULER_DOC.to_string()],
        reference_documents: vec![DEEPWIKI_DOC.to_string(), OPENAI_HARNESS_DOC.to_string()],
        selected_instances,
    };

    write_json_pretty(&campaign_dir.join("campaign-manifest.json"), &manifest)?;
    write_json_pretty(&campaign_dir.join("selected-dataset.json"), &selected_records)?;
    write_architecture_map(&campaign_dir)?;
    write_claim_catalogs(
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

pub fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub fn write_jsonl<T: Serialize>(path: &Path, rows: &[T]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    for row in rows {
        out.push_str(&serde_json::to_string(row)?);
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
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

    let config = Arc::new(
        ConfigBuilder::default()
            .cli_overrides(Vec::<(String, TomlValue)>::new())
            .harness_overrides(ConfigOverrides {
                cwd: Some(worktree_dir.clone()),
                model: Some(manifest.model.clone()),
                model_provider: Some(manifest.provider.clone()),
                approval_policy: Some(AskForApproval::Never),
                sandbox_mode: Some(CoreSandboxMode::WorkspaceWrite),
                tools_web_search_request: Some(false),
                show_raw_agent_reasoning: Some(true),
                ..Default::default()
            })
            .cloud_requirements(CloudRequirementsLoader::default())
            .build()
            .await?,
    );

    let mut client = InProcessAppServerClient::start(InProcessClientStartArgs {
        arg0_paths: Arg0DispatchPaths::default(),
        config,
        cli_overrides: Vec::new(),
        loader_overrides: LoaderOverrides::default(),
        cloud_requirements: CloudRequirementsLoader::default(),
        feedback: CodexFeedback::new(),
        config_warnings: Vec::<ConfigWarningNotification>::new(),
        session_source: SessionSource::AppServer.into(),
        enable_codex_api_key_env: true,
        client_name: "codex-swebench-study".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: 512,
    })
    .await?;

    let study_metadata = StudyMetadata {
        campaign_id: manifest.campaign_id.clone(),
        run_id: run_id.clone(),
        instance_id: record.instance_id.clone(),
        repo: record.repo.clone(),
        attempt: 1,
        study_mode: manifest.study_mode.clone(),
        task_class: Some(selected.task_class.clone()),
        artifact_root: attempt_dir.clone(),
    };

    let thread_start: ThreadStartResponse = client
        .request_typed(ClientRequest::ThreadStart {
            request_id: RequestId::Integer(1),
            params: ThreadStartParams {
                model: Some(manifest.model.clone()),
                model_provider: Some(manifest.provider.clone()),
                cwd: Some(worktree_dir.display().to_string()),
                approval_policy: Some(AppServerAskForApproval::Never),
                sandbox: Some(SandboxMode::WorkspaceWrite),
                experimental_raw_events: true,
                persist_extended_history: true,
                study_metadata: Some(study_metadata),
                ..ThreadStartParams::default()
            },
        })
        .await?;

    let _turn: TurnStartResponse = client
        .request_typed(ClientRequest::TurnStart {
            request_id: RequestId::Integer(2),
            params: TurnStartParams {
                thread_id: thread_start.thread.id.clone(),
                input: vec![UserInput::Text {
                    text: prompt.clone(),
                    text_elements: Vec::new(),
                }],
                cwd: Some(worktree_dir.clone()),
                model: Some(manifest.model.clone()),
                approval_policy: Some(AppServerAskForApproval::Never),
                sandbox_policy: Some(SandboxPolicy::WorkspaceWrite {
                    writable_roots: Vec::new(),
                    read_only_access: Default::default(),
                    network_access: false,
                    exclude_tmpdir_env_var: false,
                    exclude_slash_tmp: false,
                }),
                ..TurnStartParams::default()
            },
        })
        .await?;

    let mut raw_agent_file = tokio::fs::File::create(attempt_dir.join("raw-agent-events.jsonl")).await?;
    let mut raw_diag_file = tokio::fs::File::create(attempt_dir.join("raw-diagnostics.jsonl")).await?;
    let mut probe_file = tokio::fs::File::create(attempt_dir.join("codex-probe-events.jsonl")).await?;

    let mut decoded_events = Vec::<Event>::new();
    let mut probe_events = Vec::<StudyProbeEvent>::new();
    let mut raw_diagnostics = Vec::<Value>::new();

    loop {
        let Some(server_event) = client.next_event().await else {
            break;
        };
        match server_event {
            InProcessServerEvent::LegacyNotification(notification) => {
                raw_agent_file
                    .write_all(&(serde_json::to_string(&notification)? + "\n").into_bytes())
                    .await?;
                if let Some(decoded) = decode_legacy_notification(notification)? {
                    if let EventMsg::StudyProbe(probe) = &decoded.msg {
                        probe_file
                            .write_all(&(serde_json::to_string(probe)? + "\n").into_bytes())
                            .await?;
                        probe_events.push(probe.clone());
                    }
                    let done = matches!(
                        decoded.msg,
                        EventMsg::TurnComplete(_) | EventMsg::TurnAborted(_)
                    );
                    decoded_events.push(decoded);
                    if done {
                        break;
                    }
                }
            }
            InProcessServerEvent::ServerNotification(notification) => {
                let row = json!({
                    "kind": "server_notification",
                    "payload": notification,
                });
                raw_diag_file
                    .write_all(&(serde_json::to_string(&row)? + "\n").into_bytes())
                    .await?;
                raw_diagnostics.push(row);
            }
            InProcessServerEvent::Lagged { skipped } => {
                let row = json!({
                    "kind": "lagged",
                    "skipped": skipped,
                });
                raw_diag_file
                    .write_all(&(serde_json::to_string(&row)? + "\n").into_bytes())
                    .await?;
                raw_diagnostics.push(row);
            }
            InProcessServerEvent::ServerRequest(request) => {
                let request_id = request.id().clone();
                let row = json!({
                    "kind": "server_request",
                    "request": request,
                });
                raw_diag_file
                    .write_all(&(serde_json::to_string(&row)? + "\n").into_bytes())
                    .await?;
                raw_diagnostics.push(row);
                client
                    .reject_server_request(
                        request_id,
                        JSONRPCErrorError {
                            code: -32000,
                            data: None,
                            message: "codex-swebench-study does not interactively answer server requests during benchmark runs".to_string(),
                        },
                    )
                    .await?;
            }
        }
    }
    client.shutdown().await?;

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
        &decoded_events,
        &probe_events,
        &raw_diagnostics,
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
    let repo_cache = repo_cache_root.join(record.repo.replace('/', "__"));
    if !repo_cache.exists() {
        run_command(
            Command::new("git")
                .arg("clone")
                .arg(format!("https://github.com/{}.git", record.repo))
                .arg(&repo_cache),
        )
        .await
        .with_context(|| format!("failed to clone {}", record.repo))?;
    } else if refresh {
        run_command(
            Command::new("git")
                .arg("-C")
                .arg(&repo_cache)
                .arg("fetch")
                .arg("--all")
                .arg("--tags")
                .arg("--prune"),
        )
        .await?;
    }

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
            .arg(&record.base_commit),
    )
    .await?;
    Ok(())
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

async fn load_dataset_records(dataset_jsonl: Option<&Path>) -> Result<Vec<DatasetRecord>> {
    if let Some(path) = dataset_jsonl {
        return parse_dataset_jsonl(path);
    }
    let script = r#"
import json
from datasets import load_dataset
dataset = load_dataset("princeton-nlp/SWE-bench_Verified", split="test")
for row in dataset:
    print(json.dumps(row, ensure_ascii=False))
"#;
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .output()
        .await
        .context("failed to invoke python3 for SWE-bench dataset download")?;
    if !output.status.success() {
        bail!("dataset fetch failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    parse_dataset_bytes(&output.stdout)
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

fn select_records(dataset: &[DatasetRecord], seed: &str, sample_size: usize) -> Vec<DatasetRecord> {
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

fn decode_legacy_notification(notification: JSONRPCNotification) -> Result<Option<Event>> {
    let method = notification.method;
    if !method.starts_with("codex/event/") {
        return Ok(None);
    }

    let params = notification
        .params
        .unwrap_or_else(|| Value::Object(Map::new()));
    let original_object = match params {
        Value::Object(object) => object,
        _ => bail!("legacy notification params were not an object"),
    };
    let mut payload_object = original_object.clone();

    let mut event_payload = if let Some(Value::Object(msg_payload)) = payload_object.remove("msg") {
        msg_payload
    } else {
        let mut flattened = original_object;
        flattened.remove("conversationId");
        flattened
    };
    event_payload.insert(
        "type".to_string(),
        Value::String(
            method
                .strip_prefix("codex/event/")
                .unwrap_or(&method)
                .to_string(),
        ),
    );

    let event_id = payload_object
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    Ok(Some(Event {
        id: event_id.to_string(),
        msg: serde_json::from_value(Value::Object(event_payload))?,
    }))
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

fn attempt_artifact_paths(attempt_dir: &Path) -> BTreeMap<String, PathBuf> {
    BTreeMap::from([
        ("prompt".to_string(), attempt_dir.join("prompt.txt")),
        (
            "environmentPlan".to_string(),
            attempt_dir.join("environment-plan.json"),
        ),
        (
            "rawAgentEvents".to_string(),
            attempt_dir.join("raw-agent-events.jsonl"),
        ),
        (
            "rawDiagnostics".to_string(),
            attempt_dir.join("raw-diagnostics.jsonl"),
        ),
        (
            "codexProbeEvents".to_string(),
            attempt_dir.join("codex-probe-events.jsonl"),
        ),
        (
            "lifecycleEvents".to_string(),
            attempt_dir.join("lifecycle-events.jsonl"),
        ),
        (
            "tokenSnapshots".to_string(),
            attempt_dir.join("token-snapshots.jsonl"),
        ),
        (
            "commandEvents".to_string(),
            attempt_dir.join("command-events.jsonl"),
        ),
        ("toolEvents".to_string(), attempt_dir.join("tool-events.jsonl")),
        ("patchEvents".to_string(), attempt_dir.join("patch-events.jsonl")),
        ("anomalies".to_string(), attempt_dir.join("anomalies.jsonl")),
        ("probeEvents".to_string(), attempt_dir.join("probe-events.jsonl")),
        ("probeSummary".to_string(), attempt_dir.join("probe-summary.json")),
        ("claimEvidence".to_string(), attempt_dir.join("claim-evidence.json")),
        ("patch".to_string(), attempt_dir.join("patch.diff")),
        ("runSummary".to_string(), attempt_dir.join("run-summary.json")),
        ("runEvidence".to_string(), attempt_dir.join("run-evidence.txt")),
        ("replay".to_string(), attempt_dir.join("replay.json")),
    ])
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
