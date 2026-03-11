use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareCampaignArgs {
    pub campaign_root: PathBuf,
    pub sample_size: Option<usize>,
    pub seed: String,
    pub dataset_jsonl: Option<PathBuf>,
    pub model: String,
    pub provider: String,
    pub repo_cache_root: Option<PathBuf>,
    pub preset_path: Option<PathBuf>,
    pub stage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecord {
    pub instance_id: String,
    pub repo: String,
    pub base_commit: String,
    #[serde(default)]
    pub patch: Option<String>,
    #[serde(default)]
    pub test_patch: Option<String>,
    #[serde(default)]
    pub problem_statement: String,
    #[serde(default)]
    pub hints_text: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub environment_setup_commit: Option<String>,
    #[serde(default)]
    pub fail_to_pass: Vec<String>,
    #[serde(default)]
    pub pass_to_pass: Vec<String>,
    #[serde(default)]
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignManifest {
    pub schema_version: String,
    pub campaign_id: String,
    pub created_at: String,
    pub campaign_root: PathBuf,
    pub repo_cache_root: PathBuf,
    pub benchmark_name: String,
    pub benchmark_adapter: String,
    pub preset_name: String,
    pub preset_path: PathBuf,
    #[serde(default)]
    pub stage_name: Option<String>,
    pub probe_profile: String,
    pub report_profile: String,
    pub model: String,
    pub provider: String,
    pub seed: String,
    pub sample_size: usize,
    pub study_mode: String,
    #[serde(default)]
    pub required_task_classes: Vec<String>,
    #[serde(default)]
    pub preferred_task_classes: Vec<String>,
    #[serde(default)]
    pub future_benchmarks: Vec<String>,
    pub grounding_documents: Vec<String>,
    pub reference_documents: Vec<String>,
    pub selected_instances: Vec<SelectedInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedInstance {
    pub instance_id: String,
    pub repo: String,
    pub task_class: String,
    pub run_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: String,
    pub campaign_id: String,
    pub run_id: String,
    pub instance_id: String,
    pub repo: String,
    pub task_class: String,
    pub base_commit: String,
    pub worktree_dir: PathBuf,
    pub attempt: u32,
    pub status: String,
    pub grading_status: String,
    pub artifact_paths: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyArchitectureSubsystem {
    pub id: String,
    pub purpose: String,
    pub files: Vec<String>,
    pub reference_docs: Vec<String>,
    pub visible_events: Vec<String>,
    pub hidden_state: Vec<String>,
    pub probes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimCatalogEntry {
    pub id: String,
    pub source: String,
    pub text: String,
    pub operationalization: String,
    pub required_evidence: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeEventRow {
    pub run_id: String,
    pub instance_id: String,
    pub repo: String,
    pub attempt: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub subsystem: String,
    pub evidence_code: String,
    pub classification: String,
    pub summary: String,
    pub source_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProbeSummary {
    pub exact_probe_count: usize,
    pub inferred_probe_count: usize,
    pub estimated_probe_count: usize,
    pub first_meaningful_edit_tokens: Option<i64>,
    pub first_controlled_change_tokens: Option<i64>,
    pub first_verification_tokens: Option<i64>,
    pub first_patch_tokens: Option<i64>,
    pub final_patch_tokens: Option<i64>,
    pub ignition_shell_search_count: usize,
    pub ignition_patch_apply_count: usize,
    pub ignition_tool_mediated_count: usize,
    pub repeated_read_count: usize,
    pub repeated_verification_count: usize,
    pub repeated_git_inspection_count: usize,
    pub post_submit_activity_count: usize,
    pub cleanup_only_activity_count: usize,
    pub compaction_count: usize,
    pub compaction_rediscovery_count: usize,
    pub config_freeze_drift_count: usize,
    pub instruction_shift_count: usize,
    pub instruction_stratification_count: usize,
    pub harness_friction_count: usize,
    pub tool_mediation_tax_count: usize,
    pub control_rod_compaction_count: usize,
    pub control_rod_config_freeze_count: usize,
    pub control_rod_persistence_count: usize,
    pub chain_reaction_cycle_count: usize,
    pub containment_breach_count: usize,
    pub containment_heat_leak_count: usize,
    pub verification_closure_count: usize,
    pub persistence_continuity_count: usize,
    pub persistence_staleness_risk_count: usize,
    pub externalized_coordination_count: usize,
    pub event_discontinuity_count: usize,
    pub useful_step_proxy_num: usize,
    pub useful_step_proxy_den: usize,
    pub useful_token_proxy_num: i64,
    pub useful_token_proxy_den: i64,
    pub friction_token_proxy_num: i64,
    pub friction_token_proxy_den: i64,
    pub retained_edit_ratio_num: usize,
    pub retained_edit_ratio_den: usize,
    pub reverted_work_ratio_num: usize,
    pub reverted_work_ratio_den: usize,
    pub cache_read_ratio_num: i64,
    pub cache_read_ratio_den: i64,
    pub context_window: Option<i64>,
    pub peak_context_utilization_bps: Option<i64>,
    pub useful_token_proxy_bps: Option<i64>,
    pub friction_token_proxy_bps: Option<i64>,
    pub harness_overhead_proxy_bps: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEvidence {
    pub claim_id: String,
    pub label: String,
    pub supporting_evidence: Vec<String>,
    pub conflicting_evidence: Vec<String>,
    pub relevant_runs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunSummary {
    pub instance_id: String,
    pub repo: String,
    pub task_class: String,
    pub status: String,
    pub grading_status: String,
    pub raw_event_count: usize,
    pub raw_probe_count: usize,
    pub raw_diagnostic_count: usize,
    pub token_snapshot_count: usize,
    pub command_count: usize,
    pub tool_count: usize,
    pub patch_event_count: usize,
    pub patch_file_count: usize,
    pub patch_sha256: Option<String>,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub total_cache_read_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub model_context_window: Option<i64>,
    pub anomaly_count: usize,
    pub event_type_counts: BTreeMap<String, usize>,
    pub probe_code_counts: BTreeMap<String, usize>,
    pub probe_subsystem_counts: BTreeMap<String, usize>,
    pub diagnostic_type_counts: BTreeMap<String, usize>,
    pub artifact_inventory: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexRunRequest {
    pub model: String,
    pub provider: String,
    pub run_id: String,
    pub repo: String,
    pub instance_id: String,
    pub task_class: String,
    pub prompt: String,
    pub worktree_dir: PathBuf,
    pub attempt_dir: PathBuf,
    pub approval_never: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexRunCapture {
    pub raw_event_count: usize,
    pub raw_probe_count: usize,
    pub raw_diagnostic_count: usize,
}
