use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyStagePreset {
    pub name: String,
    #[serde(rename = "sampleSize")]
    pub sample_size: usize,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyCohortPreset {
    pub id: String,
    pub label: String,
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub personality: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyPreset {
    pub name: String,
    pub benchmark: String,
    #[serde(default = "default_benchmark_adapter")]
    pub benchmark_adapter: String,
    #[serde(default = "default_probe_profile")]
    pub probe_profile: String,
    #[serde(default = "default_report_profile")]
    pub report_profile: String,
    #[serde(default)]
    pub stages: Vec<StudyStagePreset>,
    #[serde(default)]
    pub experiment_name: Option<String>,
    #[serde(default)]
    pub comparison_axes: Vec<String>,
    #[serde(default)]
    pub cohorts: Vec<StudyCohortPreset>,
    #[serde(default)]
    pub required_task_classes: Vec<String>,
    #[serde(default)]
    pub preferred_task_classes: Vec<String>,
    #[serde(default)]
    pub future_benchmarks: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl StudyPreset {
    pub fn resolve_stage(
        &self,
        requested_stage: Option<&str>,
        sample_size_override: Option<usize>,
    ) -> Result<(Option<String>, usize)> {
        if let Some(sample_size) = sample_size_override {
            return Ok((requested_stage.map(ToOwned::to_owned), sample_size));
        }

        if let Some(stage_name) = requested_stage {
            let stage = self
                .stages
                .iter()
                .find(|stage| stage.name == stage_name)
                .ok_or_else(|| anyhow!("stage `{stage_name}` was not found in preset `{}`", self.name))?;
            return Ok((Some(stage.name.clone()), stage.sample_size));
        }

        let stage = self.stages.first().ok_or_else(|| {
            anyhow!(
                "preset `{}` did not contain any stages and no explicit sample size was provided",
                self.name
            )
        })?;
        Ok((Some(stage.name.clone()), stage.sample_size))
    }
}

pub fn load_study_preset(path: &Path) -> Result<StudyPreset> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read study preset at {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse study preset at {}", path.display()))
}

pub fn default_swebench_preset_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join("studies")
        .join("task-presets")
        .join("swebench-v1.json")
}

fn default_benchmark_adapter() -> String {
    "swebench".to_string()
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_probe_profile() -> String {
    "codex-deep-probe.v1".to_string()
}

fn default_report_profile() -> String {
    "evidence-dossier.v1".to_string()
}
