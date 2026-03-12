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
