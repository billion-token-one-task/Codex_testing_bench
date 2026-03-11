use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::fs;

use codex_bench_codex::write_architecture_map;
use codex_bench_core::{CampaignManifest, PrepareCampaignArgs, default_swebench_preset_path, load_study_preset, read_json};
use codex_bench_newtonbench::{
    grade_campaign as grade_newtonbench_campaign, prepare_campaign as prepare_newtonbench_campaign,
    run_campaign as run_newtonbench_campaign,
};
use codex_bench_nl2repo::{
    grade_campaign as grade_nl2repo_campaign, prepare_campaign as prepare_nl2repo_campaign,
    run_campaign as run_nl2repo_campaign,
};
use codex_bench_report::{render_campaign_report, render_single_run_replay};
use codex_bench_swebench::{
    grade_campaign as grade_swebench_campaign, prepare_campaign as prepare_swebench_campaign,
    run_campaign as run_swebench_campaign,
};

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
        #[arg(long)]
        sample_size: Option<usize>,
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
        #[arg(long)]
        preset_path: Option<PathBuf>,
        #[arg(long)]
        stage: Option<String>,
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
    ListPresets {
        #[arg(long)]
        presets_dir: Option<PathBuf>,
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
            preset_path,
            stage,
        } => {
            let args = PrepareCampaignArgs {
                campaign_root,
                sample_size,
                seed,
                dataset_jsonl,
                model,
                provider,
                repo_cache_root,
                preset_path,
                stage,
            };
            let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
            let resolved_preset_path = args
                .preset_path
                .clone()
                .unwrap_or_else(|| default_swebench_preset_path(&repo_root));
            let preset = load_study_preset(&resolved_preset_path)?;
            let campaign_dir = match preset.benchmark_adapter.as_str() {
                "swebench" | "repo-patch-jsonl" => prepare_swebench_campaign(args).await?,
                "nl2repo" => prepare_nl2repo_campaign(args).await?,
                "newtonbench" => prepare_newtonbench_campaign(args).await?,
                other => anyhow::bail!("unsupported benchmark adapter `{other}`"),
            };
            println!("{}", campaign_dir.display());
        }
        Command::Run {
            campaign_dir,
            refresh_repo_cache,
        } => {
            let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
            match manifest.benchmark_adapter.as_str() {
                "swebench" | "repo-patch-jsonl" => {
                    run_swebench_campaign(&campaign_dir, refresh_repo_cache).await?
                }
                "nl2repo" => run_nl2repo_campaign(&campaign_dir).await?,
                "newtonbench" => run_newtonbench_campaign(&campaign_dir).await?,
                other => anyhow::bail!("unsupported benchmark adapter `{other}`"),
            }
            println!("{}", campaign_dir.display());
        }
        Command::Grade {
            campaign_dir,
            command,
        } => {
            let manifest: CampaignManifest = read_json(&campaign_dir.join("campaign-manifest.json"))?;
            match manifest.benchmark_adapter.as_str() {
                "swebench" | "repo-patch-jsonl" => grade_swebench_campaign(&campaign_dir, command).await?,
                "nl2repo" => grade_nl2repo_campaign(&campaign_dir).await?,
                "newtonbench" => grade_newtonbench_campaign(&campaign_dir).await?,
                other => anyhow::bail!("unsupported benchmark adapter `{other}`"),
            }
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
        Command::ListPresets { presets_dir } => {
            let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
            let presets_dir = presets_dir.unwrap_or_else(|| {
                default_swebench_preset_path(&repo_root)
                    .parent()
                    .expect("default preset path has parent")
                    .to_path_buf()
            });
            for entry in fs::read_dir(&presets_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    continue;
                }
                let preset = load_study_preset(&path)?;
                println!(
                    "{} | benchmark={} | adapter={} | stages={} | required={}",
                    path.display(),
                    preset.benchmark,
                    preset.benchmark_adapter,
                    preset
                        .stages
                        .iter()
                        .map(|stage| format!("{}:{}", stage.name, stage.sample_size))
                        .collect::<Vec<_>>()
                        .join(","),
                    if preset.required_task_classes.is_empty() {
                        "-".to_string()
                    } else {
                        preset.required_task_classes.join(",")
                    }
                );
            }
        }
    }
    Ok(())
}
