use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn attempt_artifact_paths(attempt_dir: &Path) -> BTreeMap<String, PathBuf> {
    BTreeMap::from([
        ("prompt".to_string(), attempt_dir.join("prompt.txt")),
        (
            "environmentPlan".to_string(),
            attempt_dir.join("environment-plan.json"),
        ),
        (
            "rawAgentEvents".to_string(),
            attempt_dir.join("raw-agent-events.jsonl"),
        ),
        (
            "rawDiagnostics".to_string(),
            attempt_dir.join("raw-diagnostics.jsonl"),
        ),
        (
            "codexProbeEvents".to_string(),
            attempt_dir.join("codex-probe-events.jsonl"),
        ),
        (
            "lifecycleEvents".to_string(),
            attempt_dir.join("lifecycle-events.jsonl"),
        ),
        (
            "tokenSnapshots".to_string(),
            attempt_dir.join("token-snapshots.jsonl"),
        ),
        (
            "turnMetrics".to_string(),
            attempt_dir.join("turn-metrics.jsonl"),
        ),
        (
            "messageMetrics".to_string(),
            attempt_dir.join("message-metrics.jsonl"),
        ),
        (
            "personalityEvents".to_string(),
            attempt_dir.join("personality-events.jsonl"),
        ),
        (
            "commandEvents".to_string(),
            attempt_dir.join("command-events.jsonl"),
        ),
        (
            "toolEvents".to_string(),
            attempt_dir.join("tool-events.jsonl"),
        ),
        (
            "skillEvents".to_string(),
            attempt_dir.join("skill-events.jsonl"),
        ),
        (
            "skillMechanism".to_string(),
            attempt_dir.join("skill-mechanism.jsonl"),
        ),
        (
            "patchEvents".to_string(),
            attempt_dir.join("patch-events.jsonl"),
        ),
        (
            "patchChain".to_string(),
            attempt_dir.join("patch-chain.jsonl"),
        ),
        (
            "gradeEvents".to_string(),
            attempt_dir.join("grade-events.jsonl"),
        ),
        ("anomalies".to_string(), attempt_dir.join("anomalies.jsonl")),
        (
            "verbosityToolCoupling".to_string(),
            attempt_dir.join("verbosity-tool-coupling.jsonl"),
        ),
        (
            "probeEvents".to_string(),
            attempt_dir.join("probe-events.jsonl"),
        ),
        (
            "probeSummary".to_string(),
            attempt_dir.join("probe-summary.json"),
        ),
        (
            "claimEvidence".to_string(),
            attempt_dir.join("claim-evidence.json"),
        ),
        ("patch".to_string(), attempt_dir.join("patch.diff")),
        (
            "runSummary".to_string(),
            attempt_dir.join("run-summary.json"),
        ),
        (
            "runEvidence".to_string(),
            attempt_dir.join("run-evidence.txt"),
        ),
        (
            "attemptLog".to_string(),
            attempt_dir.join("attempt-log.txt"),
        ),
        ("replay".to_string(), attempt_dir.join("replay.json")),
    ])
}

pub fn artifact_role_map_for_attempt() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("prompt".to_string(), "derived_input".to_string()),
        ("environmentPlan".to_string(), "derived_input".to_string()),
        ("rawAgentEvents".to_string(), "raw_truth".to_string()),
        ("rawDiagnostics".to_string(), "raw_truth".to_string()),
        ("codexProbeEvents".to_string(), "raw_truth".to_string()),
        (
            "lifecycleEvents".to_string(),
            "derived_evidence".to_string(),
        ),
        ("tokenSnapshots".to_string(), "derived_evidence".to_string()),
        ("turnMetrics".to_string(), "derived_evidence".to_string()),
        ("messageMetrics".to_string(), "derived_evidence".to_string()),
        (
            "personalityEvents".to_string(),
            "derived_evidence".to_string(),
        ),
        ("commandEvents".to_string(), "derived_evidence".to_string()),
        ("toolEvents".to_string(), "derived_evidence".to_string()),
        ("skillEvents".to_string(), "derived_evidence".to_string()),
        ("skillMechanism".to_string(), "derived_evidence".to_string()),
        ("patchEvents".to_string(), "derived_evidence".to_string()),
        ("patchChain".to_string(), "derived_evidence".to_string()),
        ("gradeEvents".to_string(), "derived_evidence".to_string()),
        ("anomalies".to_string(), "derived_evidence".to_string()),
        (
            "verbosityToolCoupling".to_string(),
            "derived_evidence".to_string(),
        ),
        ("probeEvents".to_string(), "derived_evidence".to_string()),
        ("probeSummary".to_string(), "derived_summary".to_string()),
        ("claimEvidence".to_string(), "derived_summary".to_string()),
        ("patch".to_string(), "raw_truth".to_string()),
        ("runSummary".to_string(), "derived_summary".to_string()),
        (
            "runEvidence".to_string(),
            "human_readable_dossier".to_string(),
        ),
        (
            "attemptLog".to_string(),
            "human_readable_dossier".to_string(),
        ),
        ("replay".to_string(), "human_readable_dossier".to_string()),
    ])
}

pub fn artifact_map_for_attempt(attempt_dir: &Path) -> BTreeMap<String, PathBuf> {
    attempt_artifact_paths(attempt_dir)
}

pub fn artifact_inventory_for_attempt(attempt_dir: &Path) -> BTreeMap<String, bool> {
    attempt_artifact_paths(attempt_dir)
        .into_iter()
        .map(|(name, path)| (name, path.exists()))
        .collect()
}

pub fn patch_file_count(patch_text: &[u8]) -> usize {
    String::from_utf8_lossy(patch_text)
        .lines()
        .filter(|line| line.starts_with("diff --git "))
        .count()
}
