use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;

use crate::types::{
    BenchmarkResearchProfile, CampaignManifest, ClaimCatalogEntry, DatasetRecord, ProbeEventRow,
    ProbeSummary, RunSummary, StudyArchitectureSubsystem,
};

#[async_trait]
pub trait RuntimeAdapter {
    type Request;
    type Capture;

    async fn run_task(&self, request: Self::Request) -> Result<Self::Capture>;
}

#[async_trait]
pub trait BenchmarkAdapter {
    async fn prepare_campaign(&self, campaign_root: &Path) -> Result<PathBuf>;
    async fn run_campaign(&self, campaign_dir: &Path) -> Result<()>;
    async fn grade_campaign(&self, campaign_dir: &Path) -> Result<()>;

    fn benchmark_research_profile(&self) -> BenchmarkResearchProfile {
        BenchmarkResearchProfile::default()
    }

    fn task_classification(&self, _record: &DatasetRecord) -> Option<String> {
        None
    }

    fn expected_verification_strength(&self, _task_class: &str) -> Option<String> {
        None
    }

    fn expected_context_pressure(&self, _task_class: &str) -> Option<String> {
        None
    }

    fn expected_tool_mix(&self, _task_class: &str) -> Vec<String> {
        Vec::new()
    }

    fn expected_bootstrap_risk(&self, _task_class: &str) -> Option<String> {
        None
    }

    fn expected_language_need(&self, _task_class: &str) -> Option<String> {
        None
    }

    fn language_profile_hint(&self, _task_class: &str) -> Option<String> {
        None
    }

    fn tool_profile_hint(&self, _task_class: &str) -> Option<String> {
        None
    }

    fn interaction_style_hint(&self, _task_class: &str) -> Option<String> {
        None
    }

    fn default_analysis_overrides(&self, _task_class: &str) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

pub trait ProbeDeriver {
    fn derive_run_outputs(
        &self,
        attempt_dir: &Path,
        run_id: &str,
        task_class: &str,
        record: &DatasetRecord,
        patch_text: &[u8],
    ) -> Result<RunSummary>;
}

pub trait ReportRenderer {
    fn render_campaign_report(&self, campaign_dir: &Path) -> Result<PathBuf>;
    fn render_run_evidence(&self, attempt_dir: &Path) -> Result<PathBuf>;
}

pub trait TaskClassifier {
    fn classify_task(&self, record: &DatasetRecord) -> String;
}

pub trait ClaimCatalog {
    fn grounding_claims(&self) -> Vec<ClaimCatalogEntry>;
    fn codex_unique_claims(&self) -> Vec<ClaimCatalogEntry>;
    fn architecture_map(&self) -> Vec<StudyArchitectureSubsystem>;
    fn summarize_probes(
        &self,
        _manifest: &CampaignManifest,
        _summary: &RunSummary,
        _probe_rows: &[ProbeEventRow],
    ) -> ProbeSummary {
        ProbeSummary::default()
    }
}
