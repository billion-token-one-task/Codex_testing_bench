export type CampaignListItem = {
  campaign_id: string;
  experiment_name: string;
  benchmark_name: string;
  benchmark_adapter: string;
  stage_name?: string | null;
  created_at: string;
  status: string;
  sample_size: number;
  cohort_count: number;
  max_parallel_runs: number;
  report_paths: string[];
  dataset_paths: string[];
  selected_instances: number;
  active_run_count: number;
  completed_run_count: number;
  failed_run_count: number;
  report_count: number;
  dataset_count: number;
  total_tokens: number;
  total_visible_output_tokens_est: number;
  total_tool_calls: number;
  total_commands: number;
  path: string;
};

export type ArtifactDescriptor = {
  name: string;
  path: string;
  kind: string;
  exists: boolean;
  role?: string | null;
  scope?: string | null;
  format?: string | null;
  size_bytes?: number | null;
  updated_at?: string | null;
  line_count?: number | null;
  row_count?: number | null;
  previewable?: boolean;
};

export type LiveRunProgress = {
  current_phase: string;
  turn_count: number;
  message_count: number;
  command_count: number;
  tool_count: number;
  patch_event_count: number;
  verification_event_count: number;
  raw_event_count: number;
  artifact_row_count: number;
  stalled: boolean;
};

export type LiveRunMechanismSnapshot = {
  personality_requested?: string | null;
  personality_effective?: string | null;
  personality_fallback_count: number;
  personality_model_messages_preserved?: boolean | null;
  instruction_layers_active: string[];
  compaction_count: number;
  harness_friction_count: number;
  skill_inferred_count: number;
  active_skill_names: string[];
  last_message_category?: string | null;
  top_tool_route?: string | null;
  latest_mechanism_event?: string | null;
};

export type LiveRunTelemetry = {
  total_tokens?: number | null;
  visible_output_total_tokens_est: number;
  tokens_per_minute: number;
  messages_per_minute: number;
  commands_per_minute: number;
  tool_bursts_per_minute: number;
  visible_tokens_per_tool_call: number;
  visible_tokens_per_message: number;
  tool_calls_per_message: number;
};

export type LiveRunSnapshot = {
  run_id: string;
  campaign_id: string;
  cohort_id: string;
  model: string;
  provider: string;
  personality_mode?: string | null;
  task_class: string;
  instance_id: string;
  repo: string;
  run_status: string;
  grading_status: string;
  started_at?: string | null;
  last_event_at?: string | null;
  elapsed_ms?: number | null;
  activity_heat: string;
  current_focus?: string | null;
  warnings: string[];
  progress: LiveRunProgress;
  telemetry: LiveRunTelemetry;
  latest_message_preview?: string | null;
  latest_tool?: string | null;
  latest_patch?: string | null;
  latest_command?: string | null;
  mechanism: LiveRunMechanismSnapshot;
};

export type AttemptIndex = {
  attempt: number;
  directory: string;
  artifacts: ArtifactDescriptor[];
};

export type RunIndexEntry = {
  campaign_id: string;
  run_id: string;
  manifest_run_id: string;
  instance_id: string;
  repo: string;
  task_class: string;
  cohort_id: string;
  model: string;
  provider: string;
  personality_mode?: string | null;
  prompt_style?: string | null;
  status: string;
  grading_status: string;
  run_dir: string;
  manifest_path: string;
  latest_updated_at?: string | null;
  command_count: number;
  tool_count: number;
  patch_file_count: number;
  message_metric_count: number;
  visible_output_total_tokens_est: number;
  total_tokens?: number | null;
  anomaly_count: number;
  tool_kind_counts: Record<string, number>;
  tool_name_counts: Record<string, number>;
  tool_route_counts: Record<string, number>;
  message_category_counts: Record<string, number>;
  ignition_shell_search_count: number;
  verification_closure_count: number;
  personality_fallback_count: number;
  harness_friction_count: number;
  latest_attempt?: AttemptIndex | null;
};

export type CampaignDetail = {
  manifest: Record<string, unknown>;
  reports: ArtifactDescriptor[];
  datasets: ArtifactDescriptor[];
  runs: RunIndexEntry[];
};

export type CampaignOperationalSummary = {
  campaign: CampaignListItem;
  active_live_runs: LiveRunSnapshot[];
  latest_reports: ArtifactDescriptor[];
  latest_datasets: ArtifactDescriptor[];
  active_process_count: number;
  latest_activity_at?: string | null;
  live_visible_output_total_tokens_est: number;
  live_total_tokens: number;
  live_message_count: number;
  live_command_count: number;
  live_tool_count: number;
  live_patch_event_count: number;
  solver_status_counts: Record<string, number>;
  grading_status_counts: Record<string, number>;
  cohort_counts: Record<string, number>;
  task_class_counts: Record<string, number>;
  model_counts: Record<string, number>;
  personality_counts: Record<string, number>;
  tool_route_counts: Record<string, number>;
  tool_name_counts: Record<string, number>;
  active_cohorts: string[];
  active_instances: string[];
  unresolved_infra_failure_count: number;
  active_warning_count: number;
  stalled_live_run_count: number;
  personality_fallback_live_count: number;
  heat_counts: Record<string, number>;
  focus_samples: string[];
  latest_message_previews: string[];
  operational_warnings: string[];
};

export type LiveProcessDossier = {
  snapshot: ProcessDetail;
  kind_group: string;
};

export type LiveOverviewResponse = {
  workspace: WorkspaceIndex;
  active_campaign?: CampaignListItem | null;
  active_campaign_summary?: CampaignOperationalSummary | null;
  active_live_runs: LiveRunSnapshot[];
  current_campaign_live_runs: LiveRunSnapshot[];
  other_live_runs: LiveRunSnapshot[];
  hottest_live_runs: LiveRunSnapshot[];
  stalled_live_runs: LiveRunSnapshot[];
  running_processes: ManagedProcessSnapshot[];
  process_dossiers: LiveProcessDossier[];
  active_process_count: number;
  latest_process_output_at?: string | null;
  latest_global_focus_samples: string[];
  latest_global_message_previews: string[];
  latest_global_warnings: string[];
  operator_notices: string[];
};

export type WorkspaceIndex = {
  repo_root: string;
  generated_at: string;
  campaigns: CampaignListItem[];
  runs: RunIndexEntry[];
  summary: WorkspaceSummary;
};

export type WorkspaceSummary = {
  campaign_count: number;
  run_count: number;
  active_run_count: number;
  completed_run_count: number;
  failed_run_count: number;
  total_tokens: number;
  total_visible_output_tokens_est: number;
  total_tool_calls: number;
  total_commands: number;
};

export type ManagedProcessSnapshot = {
  id: string;
  kind: string;
  command: string[];
  cwd: string;
  status: string;
  started_at: string;
  exited_at?: string | null;
  exit_code?: number | null;
  stdout_line_count: number;
  stderr_line_count: number;
  total_output_line_count: number;
  last_output_at?: string | null;
  latest_line_preview?: string | null;
};

export type ManagedProcessOutputLine = {
  stream: "stdout" | "stderr" | string;
  line: string;
  timestamp: string;
};

export type ProcessDetail = {
  snapshot: ManagedProcessSnapshot;
  recent_output: ManagedProcessOutputLine[];
};

export type ProcessOutputEvent = {
  processId: string;
  stream: "stdout" | "stderr";
  line: string;
  timestamp: string;
};

export type UiEvent = {
  type: string;
  payload: unknown;
};

export type LiveEventBucket = {
  type: string;
  timestamp?: string | null;
  payload: Record<string, unknown>;
};

export type TimelineRow = {
  lane: string;
  kind: string;
  timestamp?: string | null;
  title: string;
  summary: string;
  payload: Record<string, unknown>;
};

export type RunDetailResponse = {
  run: RunIndexEntry;
  campaign?: CampaignListItem | null;
  reports: ArtifactDescriptor[];
  datasets: ArtifactDescriptor[];
  attempt_artifacts: ArtifactDescriptor[];
  run_summary?: Record<string, unknown> | null;
  probe_summary?: Record<string, unknown> | null;
  timeline: TimelineRow[];
  tables: Record<string, Array<Record<string, unknown>>>;
  previews: Record<string, string>;
  live_snapshot?: LiveRunSnapshot | null;
};

export type RunOperationalSummary = {
  run: RunIndexEntry;
  live_snapshot?: LiveRunSnapshot | null;
  latest_reports: ArtifactDescriptor[];
  latest_datasets: ArtifactDescriptor[];
  attempt_artifact_count: number;
  artifact_type_counts: Record<string, number>;
  event_table_counts: Record<string, number>;
  current_phase?: string | null;
  latest_focus?: string | null;
  latest_message_preview?: string | null;
  latest_tool?: string | null;
  latest_patch?: string | null;
  latest_command?: string | null;
  live_warning_count: number;
  operational_warnings: string[];
};

export type ArtifactFileResponse = {
  path: string;
  format: string;
  payload:
    | { kind: "text"; content: string }
    | { kind: "csv"; rows: Array<Record<string, string>> }
    | { kind: "jsonl"; rows: Array<Record<string, unknown>> };
};

export type ArtifactTailResponse = {
  path: string;
  lines: string[];
  truncated: boolean;
};
