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
            "commandEvents".to_string(),
            attempt_dir.join("command-events.jsonl"),
        ),
        ("toolEvents".to_string(), attempt_dir.join("tool-events.jsonl")),
        ("patchEvents".to_string(), attempt_dir.join("patch-events.jsonl")),
        ("gradeEvents".to_string(), attempt_dir.join("grade-events.jsonl")),
        ("anomalies".to_string(), attempt_dir.join("anomalies.jsonl")),
        ("probeEvents".to_string(), attempt_dir.join("probe-events.jsonl")),
        ("probeSummary".to_string(), attempt_dir.join("probe-summary.json")),
        ("claimEvidence".to_string(), attempt_dir.join("claim-evidence.json")),
        ("patch".to_string(), attempt_dir.join("patch.diff")),
        ("runSummary".to_string(), attempt_dir.join("run-summary.json")),
        ("runEvidence".to_string(), attempt_dir.join("run-evidence.txt")),
        ("replay".to_string(), attempt_dir.join("replay.json")),
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
