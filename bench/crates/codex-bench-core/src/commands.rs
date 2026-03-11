use anyhow::{Result, bail};
use tokio::process::Command;

pub async fn run_command(cmd: &mut Command) -> Result<()> {
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

pub async fn command_capture(cmd: &mut Command) -> Result<std::process::Output> {
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

pub fn render_command(cmd: &Command) -> String {
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
