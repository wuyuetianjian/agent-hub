use crate::model::{Discovery, Profile, Provider};
use std::path::PathBuf;
pub fn expand(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_default();
    }
    if let Some(tail) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        return dirs::home_dir().unwrap_or_default().join(tail);
    }
    PathBuf::from(path)
}
pub fn discover(profile: &Profile, provider: &Provider) -> Discovery {
    let executable = if !profile.executable.trim().is_empty() {
        which::which(expand(&profile.executable)).ok()
    } else {
        provider
            .executables
            .iter()
            .find_map(|name| which::which(name).ok())
    };
    let config_found = if !profile.config_dir.is_empty() {
        expand(&profile.config_dir).exists()
    } else {
        provider
            .detection_paths
            .iter()
            .any(|p| dirs::home_dir().unwrap_or_default().join(p).exists())
    };
    Discovery {
        installed: executable.is_some() || config_found,
        executable: executable.map(|p| p.to_string_lossy().into_owned()),
        config_found,
    }
}
pub fn credential_paths(profile: &Profile, provider: &Provider) -> Vec<PathBuf> {
    provider
        .credential_paths
        .iter()
        .map(|p| {
            if profile.config_dir.is_empty() {
                dirs::home_dir().unwrap_or_default().join(p)
            } else {
                expand(&profile.config_dir).join(PathBuf::from(p).file_name().unwrap_or_default())
            }
        })
        .collect()
}
pub fn read_json(path: &std::path::Path) -> anyhow::Result<serde_json::Value> {
    use anyhow::ensure;
    ensure!(
        std::fs::metadata(path)?.len() <= 2 * 1024 * 1024,
        "JSON file exceeds size limit"
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
