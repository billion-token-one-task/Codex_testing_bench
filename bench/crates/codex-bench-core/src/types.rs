use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClassification {
    Observed,
    Inferred,
    Estimated,
    Unobservable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareCampaignArgs {
    pub campaign_root: PathBuf,
    pub sample_size: Option<usize>,
    pub seed: String,
    pub dataset_jsonl: Option<PathBuf>,
    pub model: String,
    pub provider: String,
    #[serde(default)]
    pub personality: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
    #[serde(default)]
    pub experiment_name: Option<String>,
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
pub struct ExperimentCohort {
    pub cohort_id: String,
    pub label: String,
    pub model: String,
    pub provider: String,
    #[serde(default)]
    pub personality_mode: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkTaskClassProfile {
    pub task_class: String,
    pub expected_verification_strength: String,
    pub expected_context_pressure: String,
    pub expected_tool_mix: Vec<String>,
    pub expected_bootstrap_risk: String,
    pub expected_language_need: String,
    #[serde(default)]
    pub language_profile_hint: Option<String>,
    #[serde(default)]
    pub tool_profile_hint: Option<String>,
    #[serde(default)]
    pub interaction_style_hint: Option<String>,
    #[serde(default)]
    pub default_analysis_overrides: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkResearchProfile {
    pub benchmark_name: String,
    pub benchmark_adapter: String,
    pub summary: String,
    #[serde(default)]
    pub benchmark_notes: Vec<String>,
    #[serde(default)]
    pub task_class_profiles: Vec<BenchmarkTaskClassProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignManifest {
    pub schema_version: String,
    pub campaign_id: String,
    #[serde(default)]
    pub campaign_status: String,
    #[serde(default)]
    pub experiment_id: String,
    #[serde(default)]
    pub experiment_name: String,
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
    #[serde(default)]
    pub personality_mode: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
    #[serde(default)]
    pub comparison_axes: Vec<String>,
    #[serde(default)]
    pub cohorts: Vec<ExperimentCohort>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_catalog_snapshot_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hypothesis_catalog_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_lock_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark_research_profile_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_report_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_report_generated_at: Option<String>,
    pub selected_instances: Vec<SelectedInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedInstance {
    pub instance_id: String,
    pub repo: String,
    pub task_class: String,
    pub run_dir: PathBuf,
    #[serde(default)]
    pub paired_instance_key: String,
    #[serde(default)]
    pub cohort_id: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub personality_mode: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: String,
    pub campaign_id: String,
    #[serde(default)]
    pub experiment_id: String,
    #[serde(default)]
    pub experiment_name: String,
    pub run_id: String,
    pub instance_id: String,
    pub repo: String,
    pub task_class: String,
    #[serde(default)]
    pub paired_instance_key: String,
    #[serde(default)]
    pub cohort_id: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub personality_mode: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
    pub base_commit: String,
    pub worktree_dir: PathBuf,
    pub attempt: u32,
    pub status: String,
    #[serde(default)]
    pub derivations_status: String,
    #[serde(default)]
    pub evidence_status: String,
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
    #[serde(default)]
    pub observability_contract_version: Option<String>,
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
    #[serde(default)]
    pub visible_output_total_chars: usize,
    #[serde(default)]
    pub visible_output_total_tokens_est: i64,
    #[serde(default)]
    pub visible_output_message_count: usize,
    #[serde(default)]
    pub actionable_commentary_ratio_bps: Option<i64>,
    #[serde(default)]
    pub tool_grounded_commentary_ratio_bps: Option<i64>,
    #[serde(default)]
    pub verification_grounded_commentary_ratio_bps: Option<i64>,
    #[serde(default)]
    pub restatement_ratio_bps: Option<i64>,
    #[serde(default)]
    pub redundant_commentary_ratio_bps: Option<i64>,
    #[serde(default)]
    pub speculation_ratio_bps: Option<i64>,
    #[serde(default)]
    pub social_tone_ratio_bps: Option<i64>,
    #[serde(default)]
    pub tool_burst_count: usize,
    #[serde(default)]
    pub silent_tool_burst_count: usize,
    #[serde(default)]
    pub micro_narrated_tool_burst_count: usize,
    #[serde(default)]
    pub tokens_before_first_tool: Option<i64>,
    #[serde(default)]
    pub visible_text_before_first_tool_chars: Option<usize>,
    #[serde(default)]
    pub field_classifications: BTreeMap<String, String>,
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
    #[serde(default)]
    pub observability_contract_version: Option<String>,
    pub instance_id: String,
    pub repo: String,
    pub task_class: String,
    #[serde(default)]
    pub paired_instance_key: Option<String>,
    #[serde(default)]
    pub cohort_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub personality_mode: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
    pub status: String,
    pub grading_status: String,
    #[serde(default)]
    pub raw_event_count: usize,
    #[serde(default)]
    pub raw_probe_count: usize,
    #[serde(default)]
    pub raw_diagnostic_count: usize,
    #[serde(default)]
    pub turn_count: usize,
    #[serde(default)]
    pub token_snapshot_count: usize,
    #[serde(default)]
    pub command_count: usize,
    #[serde(default)]
    pub tool_count: usize,
    #[serde(default)]
    pub skill_event_count: usize,
    #[serde(default)]
    pub message_metric_count: usize,
    #[serde(default)]
    pub patch_event_count: usize,
    #[serde(default)]
    pub patch_file_count: usize,
    #[serde(default)]
    pub patch_sha256: Option<String>,
    #[serde(default)]
    pub total_input_tokens: Option<i64>,
    #[serde(default)]
    pub total_output_tokens: Option<i64>,
    #[serde(default)]
    pub total_cache_read_tokens: Option<i64>,
    #[serde(default)]
    pub total_tokens: Option<i64>,
    #[serde(default)]
    pub model_context_window: Option<i64>,
    #[serde(default)]
    pub anomaly_count: usize,
    #[serde(default)]
    pub visible_output_total_chars: usize,
    #[serde(default)]
    pub visible_output_total_tokens_est: i64,
    #[serde(default)]
    pub visible_output_sentence_count: usize,
    #[serde(default)]
    pub visible_output_paragraph_count: usize,
    #[serde(default)]
    pub visible_output_bullet_count: usize,
    #[serde(default)]
    pub visible_output_codeblock_count: usize,
    #[serde(default)]
    pub visible_output_per_turn_tokens_est: Option<i64>,
    #[serde(default)]
    pub visible_output_per_tool_call_tokens_est: Option<i64>,
    #[serde(default)]
    pub visible_output_per_patch_event_tokens_est: Option<i64>,
    #[serde(default)]
    pub visible_output_per_verification_event_tokens_est: Option<i64>,
    #[serde(default)]
    pub event_type_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub probe_code_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub probe_subsystem_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub diagnostic_type_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub tool_kind_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub tool_name_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub skill_name_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub message_category_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub artifact_inventory: BTreeMap<String, bool>,
    #[serde(default)]
    pub artifact_roles: BTreeMap<String, String>,
    #[serde(default)]
    pub field_classifications: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexRunRequest {
    pub model: String,
    pub provider: String,
    #[serde(default)]
    pub personality_mode: Option<String>,
    #[serde(default)]
    pub prompt_style: Option<String>,
    #[serde(default)]
    pub cohort_id: Option<String>,
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
