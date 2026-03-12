use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::api::{AppState, poll_workspace, router as api_router};
use crate::processes::{ProcessRegistry, UiEvent};

#[derive(Debug, Clone, Parser)]
pub struct ControlPlaneConfig {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, default_value_t = 4173)]
    pub port: u16,
    #[arg(long, default_value = "../../..")]
    pub repo_root: PathBuf,
}

pub async fn run_control_plane(config: ControlPlaneConfig) -> Result<()> {
    let repo_root = repo_root_from(&config.repo_root)?;
    let (events, _) = broadcast::channel::<UiEvent>(2048);
    let state = AppState {
        repo_root: repo_root.clone(),
        processes: ProcessRegistry::new(events.clone()),
        events: events.clone(),
    };
    tokio::spawn(poll_workspace(state.clone()));

    let mut app = api_router(state);
    if let Some((dist, index)) = frontend_paths(&repo_root) {
        app = app.fallback_service(ServeDir::new(dist).fallback(ServeFile::new(index)));
    }
    let app = app.layer(TraceLayer::new_for_http());

    let address: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = TcpListener::bind(address).await?;
    println!("codex-bench-control-plane listening on http://{}", address);
    axum::serve(listener, app).await?;
    Ok(())
}

fn repo_root_from(path: &Path) -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
    .canonicalize()?)
}

fn frontend_paths(repo_root: &Path) -> Option<(PathBuf, PathBuf)> {
    let dist = repo_root.join("apps/research-console/dist");
    if !dist.exists() {
        return None;
    }
    let index = dist.join("index.html");
    Some((dist, index))
}
