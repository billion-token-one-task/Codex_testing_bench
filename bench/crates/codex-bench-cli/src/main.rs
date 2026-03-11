use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use codex_bench_codex::write_architecture_map;
use codex_bench_report::{render_campaign_report, render_single_run_replay};
use codex_bench_swebench::{PrepareArgs, grade_campaign, prepare_campaign, run_campaign};

#[derive(Parser, Debug)]
#[command(name = "codex-bench")]
#[command(about = "Codex-only local research bench")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Prepare {
        #[arg(long)]
        campaign_root: PathBuf,
        #[arg(long, default_value_t = 15)]
        sample_size: usize,
        #[arg(long, default_value = "codex-study")]
        seed: String,
        #[arg(long)]
        dataset_jsonl: Option<PathBuf>,
        #[arg(long, default_value = "gpt-5.4")]
        model: String,
        #[arg(long, default_value = "openai")]
        provider: String,
        #[arg(long)]
        repo_cache_root: Option<PathBuf>,
    },
    Run {
        campaign_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        refresh_repo_cache: bool,
    },
    Grade {
        campaign_dir: PathBuf,
        #[arg(long)]
        command: Option<String>,
    },
    Report {
        campaign_dir: PathBuf,
    },
    Replay {
        run_dir: PathBuf,
    },
    InspectCodex {
        campaign_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Prepare {
            campaign_root,
            sample_size,
            seed,
            dataset_jsonl,
            model,
            provider,
            repo_cache_root,
        } => {
            let campaign_dir = prepare_campaign(PrepareArgs {
                campaign_root,
                sample_size,
                seed,
                dataset_jsonl,
                model,
                provider,
                repo_cache_root,
            })
            .await?;
            println!("{}", campaign_dir.display());
        }
        Command::Run {
            campaign_dir,
            refresh_repo_cache,
        } => {
            run_campaign(&campaign_dir, refresh_repo_cache).await?;
            println!("{}", campaign_dir.display());
        }
        Command::Grade {
            campaign_dir,
            command,
        } => {
            grade_campaign(&campaign_dir, command).await?;
            println!("{}", campaign_dir.display());
        }
        Command::Report { campaign_dir } => {
            let report_path = render_campaign_report(&campaign_dir)?;
            println!("{}", report_path.display());
        }
        Command::Replay { run_dir } => {
            let replay_path = render_single_run_replay(&run_dir)?;
            println!("{}", replay_path.display());
        }
        Command::InspectCodex { campaign_dir } => {
            let path = write_architecture_map(&campaign_dir)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
