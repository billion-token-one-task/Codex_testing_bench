use std::env;
use std::path::PathBuf;

pub fn preferred_python() -> PathBuf {
    if let Some(conda_prefix) = env::var_os("CONDA_PREFIX") {
        let candidate = PathBuf::from(conda_prefix).join("bin").join("python");
        if candidate.exists() {
            return candidate;
        }
    }
    if let Some(python) = env::var_os("PYTHON") {
        let candidate = PathBuf::from(python);
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from("python3")
}
