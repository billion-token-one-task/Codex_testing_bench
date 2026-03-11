use std::sync::Arc;

use anyhow::{Result, bail};
use codex_app_server_client::{
    InProcessAppServerClient, InProcessClientStartArgs, InProcessServerEvent,
};
use codex_app_server_protocol::{
    AskForApproval as AppServerAskForApproval, ClientRequest, ConfigWarningNotification,
    JSONRPCErrorError, JSONRPCNotification, RequestId, SandboxMode, SandboxPolicy, SessionSource,
    ThreadStartParams, ThreadStartResponse, TurnStartParams, TurnStartResponse, UserInput,
};
use codex_arg0::Arg0DispatchPaths;
use codex_bench_core::CodexRunRequest;
use codex_core::config::{ConfigBuilder, ConfigOverrides};
use codex_core::config_loader::{CloudRequirementsLoader, LoaderOverrides};
use codex_feedback::CodexFeedback;
use codex_protocol::config_types::Personality;
use codex_protocol::config_types::SandboxMode as CoreSandboxMode;
use codex_protocol::protocol::{AskForApproval, Event, EventMsg, StudyMetadata, StudyProbeEvent};
use serde_json::{Map, Value, json};
use tokio::io::AsyncWriteExt;
use toml::Value as TomlValue;

#[derive(Debug, Clone, Default)]
pub struct CodexRuntimeCapture {
    pub decoded_events: Vec<Event>,
    pub probe_events: Vec<StudyProbeEvent>,
    pub raw_diagnostics: Vec<Value>,
}

fn benchmark_cli_overrides() -> Vec<(String, TomlValue)> {
    vec![(
        "web_search".to_string(),
        TomlValue::String("disabled".to_string()),
    )]
}

pub async fn run_codex_task(request: CodexRunRequest) -> Result<CodexRuntimeCapture> {
    let worktree_dir = request
        .worktree_dir
        .canonicalize()
        .unwrap_or_else(|_| request.worktree_dir.clone());
    let attempt_dir = request
        .attempt_dir
        .canonicalize()
        .unwrap_or_else(|_| request.attempt_dir.clone());

    let cli_overrides = benchmark_cli_overrides();

    let config = Arc::new(
        ConfigBuilder::default()
            .cli_overrides(cli_overrides.clone())
            .harness_overrides(ConfigOverrides {
                cwd: Some(worktree_dir.clone()),
                model: Some(request.model.clone()),
                model_provider: Some(request.provider.clone()),
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
        cli_overrides,
        loader_overrides: LoaderOverrides::default(),
        cloud_requirements: CloudRequirementsLoader::default(),
        feedback: CodexFeedback::new(),
        config_warnings: Vec::<ConfigWarningNotification>::new(),
        session_source: SessionSource::AppServer.into(),
        enable_codex_api_key_env: true,
        client_name: "codex-bench".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: 512,
    })
    .await?;

    let study_metadata = StudyMetadata {
        campaign_id: request
            .run_id
            .split("-attempt-")
            .next()
            .unwrap_or(&request.run_id)
            .to_string(),
        run_id: request.run_id.clone(),
        instance_id: request.instance_id.clone(),
        repo: request.repo.clone(),
        attempt: 1,
        study_mode: "codex_research_bench".to_string(),
        task_class: Some(request.task_class.clone()),
        artifact_root: attempt_dir.clone(),
    };

    let thread_start: ThreadStartResponse = client
        .request_typed(ClientRequest::ThreadStart {
            request_id: RequestId::Integer(1),
            params: ThreadStartParams {
                model: Some(request.model.clone()),
                model_provider: Some(request.provider.clone()),
                personality: request
                    .personality_mode
                    .as_deref()
                    .and_then(parse_personality),
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
                    text: request.prompt.clone(),
                    text_elements: Vec::new(),
                }],
                cwd: Some(worktree_dir.clone()),
                model: Some(request.model.clone()),
                personality: request
                    .personality_mode
                    .as_deref()
                    .and_then(parse_personality),
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

    let mut raw_agent_file =
        tokio::fs::File::create(attempt_dir.join("raw-agent-events.jsonl")).await?;
    let mut raw_diag_file =
        tokio::fs::File::create(attempt_dir.join("raw-diagnostics.jsonl")).await?;
    let mut probe_file =
        tokio::fs::File::create(attempt_dir.join("codex-probe-events.jsonl")).await?;

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
                    if matches!(decoded.msg, EventMsg::WebSearchBegin(_)) {
                        bail!(
                            "benchmark run emitted web_search_begin even though web_search was forced disabled"
                        );
                    }
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
                            message:
                                "codex-bench does not answer interactive server requests during study runs"
                                    .to_string(),
                        },
                    )
                    .await?;
            }
        }
    }
    client.shutdown().await?;

    Ok(CodexRuntimeCapture {
        decoded_events,
        probe_events,
        raw_diagnostics,
    })
}

fn parse_personality(value: &str) -> Option<Personality> {
    match value {
        "friendly" => Some(Personality::Friendly),
        "pragmatic" => Some(Personality::Pragmatic),
        "none" => Some(Personality::None),
        _ => None,
    }
}

pub fn decode_legacy_notification(notification: JSONRPCNotification) -> Result<Option<Event>> {
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
