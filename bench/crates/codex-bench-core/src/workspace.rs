use std::fs;
use std::path::Path;

use anyhow::Result;
use tokio::process::Command;

use crate::commands::run_command;

pub fn reset_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

pub async fn init_git_workspace(workspace_dir: &Path) -> Result<()> {
    run_command(Command::new("git").arg("init").arg(workspace_dir)).await?;
    run_command(
        Command::new("git")
            .arg("-C")
            .arg(workspace_dir)
            .arg("config")
            .arg("user.name")
            .arg("codex-bench"),
    )
    .await?;
    run_command(
        Command::new("git")
            .arg("-C")
            .arg(workspace_dir)
            .arg("config")
            .arg("user.email")
            .arg("codex-bench@example.invalid"),
    )
    .await?;
    Ok(())
}

pub async fn git_commit_all(workspace_dir: &Path, message: &str) -> Result<()> {
    run_command(
        Command::new("git")
            .arg("-C")
            .arg(workspace_dir)
            .arg("add")
            .arg("."),
    )
    .await?;
    run_command(
        Command::new("git")
            .arg("-C")
            .arg(workspace_dir)
            .arg("commit")
            .arg("-m")
            .arg(message),
    )
    .await?;
    Ok(())
}
