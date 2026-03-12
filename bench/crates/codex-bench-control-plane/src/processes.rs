use std::collections::BTreeMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{RwLock, broadcast};

#[derive(Debug, Clone, Serialize)]
pub struct ManagedProcessSnapshot {
    pub id: String,
    pub kind: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub status: String,
    pub started_at: String,
    pub exited_at: Option<String>,
    pub exit_code: Option<i32>,
}

pub struct ManagedProcess {
    pub snapshot: ManagedProcessSnapshot,
    pub child: Arc<RwLock<Child>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UiEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[derive(Clone)]
pub struct ProcessRegistry {
    inner: Arc<RwLock<BTreeMap<String, ManagedProcess>>>,
    events: broadcast::Sender<UiEvent>,
}

impl ProcessRegistry {
    pub fn new(events: broadcast::Sender<UiEvent>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
            events,
        }
    }

    pub async fn list(&self) -> Vec<ManagedProcessSnapshot> {
        self.inner
            .read()
            .await
            .values()
            .map(|process| process.snapshot.clone())
            .collect()
    }

    pub async fn stop(&self, process_id: &str) -> Result<()> {
        let child = {
            let registry = self.inner.read().await;
            registry.get(process_id).map(|process| process.child.clone())
        }
        .context("managed process not found")?;
        child.write().await.kill().await?;
        Ok(())
    }

    pub async fn spawn_cli_process(
        &self,
        repo_root: &Path,
        kind: &str,
        args: &[String],
    ) -> Result<ManagedProcessSnapshot> {
        let process_id = format!("proc-{}-{}", kind, Utc::now().timestamp_millis());
        let command = bench_cli_command(repo_root, args);
        let cwd = repo_root.join("bench");
        let mut child = Command::new(&command.0);
        child.args(&command.1);
        child.current_dir(&cwd);
        child.stdout(Stdio::piped());
        child.stderr(Stdio::piped());
        let mut child = child.spawn().with_context(|| format!("failed to spawn `{kind}`"))?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let snapshot = ManagedProcessSnapshot {
            id: process_id.clone(),
            kind: kind.to_string(),
            command: std::iter::once(command.0.clone())
                .chain(command.1.clone().into_iter())
                .collect(),
            cwd: cwd.display().to_string(),
            status: "running".to_string(),
            started_at: Utc::now().to_rfc3339(),
            exited_at: None,
            exit_code: None,
        };
        let child = Arc::new(RwLock::new(child));
        self.inner.write().await.insert(
            process_id.clone(),
            ManagedProcess {
                snapshot: snapshot.clone(),
                child: child.clone(),
            },
        );
        let _ = self.events.send(UiEvent {
            event_type: "process.updated".to_string(),
            payload: serde_json::to_value(&snapshot)?,
        });

        if let Some(stdout) = stdout {
            let events = self.events.clone();
            let process_id = process_id.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = events.send(UiEvent {
                        event_type: "process.output".to_string(),
                        payload: serde_json::json!({
                            "processId": process_id,
                            "stream": "stdout",
                            "line": line,
                            "timestamp": Utc::now().to_rfc3339(),
                        }),
                    });
                }
            });
        }

        if let Some(stderr) = stderr {
            let events = self.events.clone();
            let process_id = process_id.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let _ = events.send(UiEvent {
                        event_type: "process.output".to_string(),
                        payload: serde_json::json!({
                            "processId": process_id,
                            "stream": "stderr",
                            "line": line,
                            "timestamp": Utc::now().to_rfc3339(),
                        }),
                    });
                }
            });
        }

        let registry = self.inner.clone();
        let events = self.events.clone();
        let process_id_for_task = process_id.clone();
        tokio::spawn(async move {
            let exit = child.write().await.wait().await.ok();
            let mut registry = registry.write().await;
            if let Some(process) = registry.get_mut(&process_id_for_task) {
                process.snapshot.status = "exited".to_string();
                process.snapshot.exited_at = Some(Utc::now().to_rfc3339());
                process.snapshot.exit_code = exit.and_then(|status| status.code());
                let _ = events.send(UiEvent {
                    event_type: "process.updated".to_string(),
                    payload: serde_json::to_value(&process.snapshot).unwrap_or_else(|_| serde_json::json!({})),
                });
            }
        });

        Ok(snapshot)
    }
}

fn bench_cli_command(repo_root: &Path, args: &[String]) -> (String, Vec<String>) {
    let binary = repo_root.join("bench/target/debug/codex-bench-cli");
    if binary.exists() {
        return (
            binary.display().to_string(),
            args.iter().map(|value| value.to_string()).collect(),
        );
    }
    (
        "cargo".to_string(),
        std::iter::once("run".to_string())
            .chain(std::iter::once("-p".to_string()))
            .chain(std::iter::once("codex-bench-cli".to_string()))
            .chain(std::iter::once("--".to_string()))
            .chain(args.iter().cloned())
            .collect(),
    )
}
