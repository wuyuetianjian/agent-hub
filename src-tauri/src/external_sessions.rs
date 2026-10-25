use crate::model::{Profile, Provider, SessionInfo};
use rusqlite::{Connection, OpenFlags};
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, Read},
    path::{Path, PathBuf},
};

pub fn scan(profile: &Profile, provider: &Provider, limit: usize) -> Vec<SessionInfo> {
    let env = provider
        .config_env
        .as_ref()
        .and_then(|key| std::env::var(key).ok().map(|value| (key.clone(), value)))
        .into_iter()
        .collect();
    scan_with_environment(
        profile,
        provider,
        limit,
        &dirs::home_dir().unwrap_or_default(),
        &env,
    )
}
pub fn scan_in(
    profile: &Profile,
    provider: &Provider,
    limit: usize,
    home: &Path,
) -> Vec<SessionInfo> {
    scan_with_environment(profile, provider, limit, home, &HashMap::new())
}
pub fn scan_with_environment(
    profile: &Profile,
    provider: &Provider,
    limit: usize,
    home: &Path,
    env: &HashMap<String, String>,
) -> Vec<SessionInfo> {
    if limit == 0 {
        return vec![];
    }
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
    let roots: Vec<PathBuf> = if let Some(path) = configured {
        crate::discovery::expand_pattern(path, home, env)
            .into_iter()
            .collect()
    } else {
        provider
            .detection_paths
            .iter()
            .map(|p| {
                let path = PathBuf::from(p);
                if path.is_absolute() {
                    path
                } else {
                    home.join(path)
                }
            })
            .collect()
    };
    let mut sessions: HashMap<String, SessionInfo> = HashMap::new();
    for root in roots {
        let mut usable_index = false;
        for pattern in &provider.session_index_paths {
            let literal_root = glob::Pattern::escape(&root.to_string_lossy());
            let query = PathBuf::from(literal_root).join(pattern);
            let mut paths: Vec<_> = glob::glob(&query.to_string_lossy())
                .map(|paths| paths.flatten().take(limit).collect())
                .unwrap_or_default();
            paths.sort_by_key(|p: &PathBuf| {
                std::cmp::Reverse(
                    p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .split(|c: char| !c.is_ascii_digit())
                        .filter_map(|s| s.parse::<u64>().ok())
                        .collect::<Vec<_>>(),
                )
            });
            for path in paths {
                if let Some(rows) = from_index(&path, profile, limit) {
                    usable_index = true;
                    for row in rows {
                        sessions.insert(row.id.clone(), row);
                    }
                    break;
                }
            }
            if usable_index {
                break;
            }
        }
        if !usable_index {
            let mut budget = limit.saturating_mul(20);
            let mut files = vec![];
            for pattern in &provider.session_log_paths {
                let parts: Vec<_> = pattern.split(['/', '\\']).collect();
                bounded_paths(&root, &parts, &mut budget, &mut files);
                if budget == 0 {
                    break;
                }
            }
            for path in files {
                if let Some(row) = from_header(&path, profile) {
                    sessions.entry(row.id.clone()).or_insert(row);
                }
                if sessions.len() >= limit {
                    break;
                }
            }
        }
    }
    let mut rows: Vec<_> = sessions.into_values().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.updated_at));
    rows.truncate(limit);
    rows
}
/// Bound directory enumeration before sorting or opening any history files.
fn bounded_paths(root: &Path, parts: &[&str], budget: &mut usize, files: &mut Vec<PathBuf>) {
    let Some((first, rest)) = parts.split_first() else {
        return;
    };
    if *budget == 0 {
        return;
    }
    if !first.contains(['*', '?', '[']) {
        let next = root.join(first);
        if rest.is_empty() {
            if next.is_file() {
                files.push(next);
                *budget -= 1;
            }
        } else {
            bounded_paths(&next, rest, budget, files);
        }
        return;
    }
    let Ok(pattern) = glob::Pattern::new(first) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut entries: Vec<_> = entries.take(*budget).flatten().collect();
    *budget = budget.saturating_sub(entries.len());
    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    for entry in entries {
        if !pattern.matches(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let path = entry.path();
        if rest.is_empty() {
            if path.is_file() {
                files.push(path);
            }
        } else if *budget > 0 && path.is_dir() {
            bounded_paths(&path, rest, budget, files);
        }
    }
}
fn session(
    profile: &Profile,
    id: String,
    cwd: String,
    title: String,
    source: String,
    created: i64,
    updated: i64,
) -> SessionInfo {
    SessionInfo {
        id: format!("external:{}:{}", profile.id, id),
        profile_id: profile.id.clone(),
        provider_id: profile.provider_id.clone(),
        cwd,
        mode: "external".into(),
        status: "external".into(),
        started_at: created,
        updated_at: updated,
        read_only: true,
        title: title.chars().take(256).collect(),
        source: source.chars().take(128).collect(),
    }
}
fn from_index(path: &Path, profile: &Profile, limit: usize) -> Option<Vec<SessionInfo>> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    let mut schema = connection.prepare("PRAGMA table_info(threads)").ok()?;
    let columns: HashSet<String> = schema
        .query_map([], |r| r.get::<_, String>(1))
        .ok()?
        .flatten()
        .collect();
    if !columns.contains("id") {
        return None;
    }
    let text = |key: &str| {
        if columns.contains(key) {
            format!("COALESCE({key},'')")
        } else {
            "''".into()
        }
    };
    let number = |key: &str| {
        if columns.contains(key) {
            format!("COALESCE({key},0)")
        } else {
            "0".into()
        }
    };
    let filter = if columns.contains("archived") {
        "WHERE COALESCE(archived,0)=0"
    } else {
        ""
    };
    let order = if columns.contains("updated_at") {
        "updated_at DESC"
    } else {
        "rowid DESC"
    };
    let sql = format!(
        "SELECT id,{},{},{},{},{} FROM threads {filter} ORDER BY {order} LIMIT ?1",
        text("cwd"),
        text("title"),
        text("source"),
        number("created_at"),
        number("updated_at")
    );
    let mut query = connection.prepare(&sql).ok()?;
    let rows = query
        .query_map([limit as i64], |r| {
            Ok(session(
                profile,
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .ok()?;
    Some(rows.flatten().collect())
}
fn from_header(path: &Path, profile: &Profile) -> Option<SessionInfo> {
    let file = std::fs::File::open(path).ok()?;
    let modified = file
        .metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let mut line = String::new();
    std::io::BufReader::new(file)
        .take(65537)
        .read_line(&mut line)
        .ok()?;
    if line.len() > 65536 {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&line).ok()?;
    if value["type"].as_str() != Some("session_meta") {
        return None;
    }
    let meta = &value["payload"];
    let id = meta["id"].as_str()?.to_owned();
    let created = value["timestamp"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|v| v.timestamp())
        .unwrap_or(modified);
    Some(session(
        profile,
        id,
        meta["cwd"].as_str().unwrap_or("").into(),
        meta["session_name"].as_str().unwrap_or("").into(),
        meta["source"]
            .as_str()
            .or_else(|| meta["originator"].as_str())
            .unwrap_or("Codex")
            .into(),
        created,
        modified,
    ))
}
