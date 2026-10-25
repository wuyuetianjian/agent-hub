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
    let home = dirs::home_dir().unwrap_or_default();
    let env = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect();
    discover_in(
        profile,
        provider,
        &home,
        std::env::var_os("PATH").as_deref(),
        &env,
    )
}
pub fn expand_pattern(
    template: &str,
    home: &std::path::Path,
    env: &std::collections::HashMap<String, String>,
) -> Option<PathBuf> {
    let mut value = template.to_owned();
    for _ in 0..16 {
        let Some(start) = value.find("${") else { break };
        let end = value[start + 2..].find('}')? + start + 2;
        let replacement = env.get(&value[start + 2..end])?;
        value.replace_range(start..=end, replacement);
    }
    if value.contains("${") {
        return None;
    }
    if value == "~" {
        Some(home.to_path_buf())
    } else if let Some(tail) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
    {
        Some(home.join(tail))
    } else {
        Some(PathBuf::from(value))
    }
}
fn matches(
    template: &str,
    home: &std::path::Path,
    env: &std::collections::HashMap<String, String>,
) -> Vec<PathBuf> {
    let escaped_home = PathBuf::from(glob::Pattern::escape(&home.to_string_lossy()));
    let escaped_env = env
        .iter()
        .map(|(k, v)| (k.clone(), glob::Pattern::escape(v)))
        .collect();
    let Some(pattern) = expand_pattern(template, &escaped_home, &escaped_env) else {
        return vec![];
    };
    let Ok(paths) = glob::glob(&pattern.to_string_lossy()) else {
        return vec![];
    };
    let mut paths: Vec<_> = paths.flatten().collect();
    let version = |p: &PathBuf| {
        p.to_string_lossy()
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|part| part.parse::<u64>().ok())
            .collect::<Vec<_>>()
    };
    paths.sort_by_key(|p| std::cmp::Reverse(version(p)));
    paths
}
pub fn discover_in(
    profile: &Profile,
    provider: &Provider,
    home: &std::path::Path,
    path: Option<&std::ffi::OsStr>,
    env: &std::collections::HashMap<String, String>,
) -> Discovery {
    let executable = if !profile.executable.trim().is_empty() {
        expand_pattern(&profile.executable, home, env)
            .and_then(|p| which::which_in(p, path, home).ok())
    } else {
        provider
            .executables
            .iter()
            .find_map(|name| which::which_in(name, path, home).ok())
            .or_else(|| {
                provider
                    .executable_paths
                    .iter()
                    .flat_map(|p| matches(p, home, env))
                    .find_map(|p| which::which_in(p, path, home).ok())
            })
    };
    let configured = if !profile.config_dir.is_empty() {
        Some(profile.config_dir.as_str())
    } else {
        provider
            .config_env
            .as_ref()
            .and_then(|key| env.get(key))
            .filter(|v| !v.is_empty())
            .map(String::as_str)
    };
    let config_found = if let Some(path) = configured {
        expand_pattern(path, home, env).is_some_and(|p| p.exists())
    } else {
        provider.detection_paths.iter().any(|p| {
            expand_pattern(p, home, env).is_some_and(|p| {
                if p.is_absolute() {
                    p.exists()
                } else {
                    home.join(p).exists()
                }
            })
        })
    };
    let desktop_found = provider
        .desktop_paths
        .iter()
        .any(|p| matches(p, home, env).iter().any(|p| p.exists()));
    Discovery {
        installed: executable.is_some() || config_found || desktop_found,
        desktop_found,
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
