use anyhow::Result;
use clap::Parser;

use codex_bench_control_plane::{ControlPlaneConfig, run_control_plane};

#[tokio::main]
async fn main() -> Result<()> {
    let config = ControlPlaneConfig::parse();
    run_control_plane(config).await
}
