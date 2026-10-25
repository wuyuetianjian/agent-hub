use crate::{
    model::{Profile, Provider, Settings},
    store::Store,
};
use anyhow::{ensure, Result};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    hash::{Hash, Hasher},
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub reasoning: u64,
}
impl Tokens {
    pub fn add(&mut self, other: &Self) {
        self.total = self.total.saturating_add(other.total);
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.cache_read = self.cache_read.saturating_add(other.cache_read);
        self.cache_write = self.cache_write.saturating_add(other.cache_write);
        self.reasoning = self.reasoning.saturating_add(other.reasoning);
    }
    fn delta(&self, prev: &Self) -> Self {
        Self {
            total: self.total.saturating_sub(prev.total),
            input: self.input.saturating_sub(prev.input),
            output: self.output.saturating_sub(prev.output),
            cache_read: self.cache_read.saturating_sub(prev.cache_read),
            cache_write: self.cache_write.saturating_sub(prev.cache_write),
            reasoning: self.reasoning.saturating_sub(prev.reasoning),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interval {
    pub start: i64,
    pub end: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Bucket {
    pub start: i64,
    pub end: i64,
    pub tokens: Tokens,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUsage {
    pub profile_id: String,
    pub provider_id: String,
    pub tokens: Tokens,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSource {
    pub profile_id: String,
    pub provider_id: String,
    pub status: String,
    pub files: usize,
    pub warnings: u64,
    pub has_data: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReport {
    pub total: Tokens,
    pub buckets: Vec<Bucket>,
    pub agents: Vec<AgentUsage>,
    pub sources: Vec<UsageSource>,
    pub scanned_at: i64,
    pub partial: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Cursor {
    pub offset: u64,
    pub length: u64,
    pub modified: u64,
    pub fingerprint: u64,
    pub session: String,
    pub previous: Option<Tokens>,
    pub skipping: bool,
    pub warnings: u64,
}
#[derive(Debug, Clone)]
pub struct UsageEvent {
    pub id: String,
    pub at: i64,
    pub tokens: Tokens,
}
fn number(value: &Value, key: &str) -> u64 {
    value[key]
        .as_u64()
        .filter(|n| *n <= i64::MAX as u64 / 1024)
        .unwrap_or(0)
}
fn timestamp(value: &Value) -> Option<i64> {
    value
        .as_str()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.timestamp())
        .or_else(|| value.as_i64())
}
fn codex_tokens(v: &Value) -> Tokens {
    let input = number(v, "input_tokens");
    let output = number(v, "output_tokens");
    Tokens {
        total: v["total_tokens"]
            .as_u64()
            .filter(|n| *n <= i64::MAX as u64 / 1024)
            .unwrap_or(input + output),
        input,
        output,
        cache_read: number(v, "cached_input_tokens"),
        cache_write: number(v, "cache_write_input_tokens"),
        reasoning: number(v, "reasoning_output_tokens"),
    }
}
/// Extract only usage metadata; never retain message text or tool arguments.
pub fn parse_record(format: &str, value: &Value, state: &mut Cursor) -> Vec<UsageEvent> {
    if format == "codex" && value["type"] == "session_meta" {
        if let Some(id) = value["payload"]["id"].as_str() {
            if state.session != id {
                state.previous = None;
            }
            state.session = id.to_owned();
        }
        return vec![];
    }
    if format == "gemini" && value["sessionId"].is_string() {
        state.session = value["sessionId"].as_str().unwrap().into();
    }
    if format == "gemini" && value["messages"].is_array() {
        return value["messages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|record| parse_record(format, record, state))
            .collect();
    }
    if format == "gemini" && value["$set"]["messages"].is_array() {
        return value["$set"]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|record| parse_record(format, record, state))
            .collect();
    }
    let Some(at) = timestamp(&value["timestamp"]) else {
        return vec![];
    };
    if format == "codex" {
        if value["payload"]["type"] != "token_count" {
            return vec![];
        }
        let info = &value["payload"]["info"];
        let cumulative = &info["total_token_usage"];
        if !cumulative.is_object() {
            return vec![];
        }
        if cumulative["input_tokens"].as_u64().is_none()
            || cumulative["output_tokens"].as_u64().is_none()
        {
            return vec![];
        }
        let total = codex_tokens(cumulative);
        let counts = match &state.previous {
            Some(previous) if &total == previous => return vec![],
            Some(previous)
                if total.total >= previous.total
                    && total.input >= previous.input
                    && total.output >= previous.output =>
            {
                total.delta(previous)
            }
            None if info["last_token_usage"]["input_tokens"].as_u64().is_some()
                && info["last_token_usage"]["output_tokens"].as_u64().is_some() =>
            {
                codex_tokens(&info["last_token_usage"])
            }
            _ => total.clone(),
        };
        state.previous = Some(total.clone());
        return vec![UsageEvent {
            id: format!(
                "{}:{at}:{}:{}:{}",
                state.session, total.total, total.input, total.output
            ),
            at,
            tokens: counts,
        }];
    }
    let (id, tokens) = if format == "claude"
        && value["type"] == "assistant"
        && value["message"]["usage"].is_object()
    {
        let usage = &value["message"]["usage"];
        if usage["input_tokens"].as_u64().is_none() || usage["output_tokens"].as_u64().is_none() {
            return vec![];
        }
        let cache_read = number(usage, "cache_read_input_tokens");
        let cache_write = number(usage, "cache_creation_input_tokens");
        let input = number(usage, "input_tokens") + cache_read + cache_write;
        let output = number(usage, "output_tokens");
        let Some(id) = value["message"]["id"].as_str().or(value["uuid"].as_str()) else {
            return vec![];
        };
        let session = value["sessionId"].as_str().unwrap_or(&state.session);
        (
            if id.starts_with("msg_") || uuid::Uuid::parse_str(id).is_ok() {
                format!("claude:{id}")
            } else {
                format!("{session}:{id}")
            },
            Tokens {
                total: input + output,
                input,
                output,
                cache_read,
                cache_write,
                reasoning: 0,
            },
        )
    } else if format == "gemini" && value["tokens"].is_object() {
        let usage = &value["tokens"];
        if usage["input"].as_u64().is_none() || usage["output"].as_u64().is_none() {
            return vec![];
        }
        let input = number(usage, "input");
        let reasoning = number(usage, "thoughts");
        let output = number(usage, "output") + reasoning;
        let Some(id) = value["id"].as_str() else {
            return vec![];
        };
        (
            format!("{}:{id}", state.session),
            Tokens {
                total: usage["total"]
                    .as_u64()
                    .filter(|n| *n <= i64::MAX as u64 / 1024)
                    .unwrap_or(input + output),
                input,
                output,
                cache_read: number(usage, "cached"),
                cache_write: 0,
                reasoning,
            },
        )
    } else {
        return vec![];
    };
    vec![UsageEvent { id, at, tokens }]
}
impl Store {
    pub fn init_usage(&self) -> Result<()> {
        self.connection.lock().unwrap().execute_batch("CREATE TABLE IF NOT EXISTS usage_events(scope TEXT NOT NULL,event_id TEXT NOT NULL,profile_id TEXT NOT NULL,provider_id TEXT NOT NULL,at INTEGER NOT NULL,total INTEGER NOT NULL,input INTEGER NOT NULL,output INTEGER NOT NULL,cache_read INTEGER NOT NULL,cache_write INTEGER NOT NULL,reasoning INTEGER NOT NULL,PRIMARY KEY(scope,event_id));CREATE INDEX IF NOT EXISTS usage_events_time ON usage_events(at);CREATE TABLE IF NOT EXISTS usage_cursors(path TEXT PRIMARY KEY,value TEXT NOT NULL);CREATE TABLE IF NOT EXISTS usage_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);")?;
        Ok(())
    }
    fn usage_cursor(&self, path: &str) -> Result<Cursor> {
        let value: Option<String> = self
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM usage_cursors WHERE path=?1",
                [path],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_default())
    }
    pub fn save_usage_batch(
        &self,
        scope: &str,
        profile: &Profile,
        provider: &Provider,
        path: &str,
        cursor: &Cursor,
        events: &[UsageEvent],
    ) -> Result<()> {
        let mut connection = self.connection.lock().unwrap();
        let tx = connection.transaction()?;
        for event in events {
            let t = &event.tokens;
            tx.execute("INSERT INTO usage_events(scope,event_id,profile_id,provider_id,at,total,input,output,cache_read,cache_write,reasoning) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(scope,event_id) DO UPDATE SET at=MIN(at,excluded.at),total=MAX(total,excluded.total),input=MAX(input,excluded.input),output=MAX(output,excluded.output),cache_read=MAX(cache_read,excluded.cache_read),cache_write=MAX(cache_write,excluded.cache_write),reasoning=MAX(reasoning,excluded.reasoning),profile_id=excluded.profile_id",params![scope,event.id,profile.id,provider.id,event.at,t.total,t.input,t.output,t.cache_read,t.cache_write,t.reasoning])?;
        }
        tx.execute("INSERT INTO usage_cursors(path,value) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET value=excluded.value",params![path,serde_json::to_string(cursor)?])?;
        tx.commit()?;
        Ok(())
    }
    pub fn usage_report(
        &self,
        intervals: &[Interval],
        scopes: &HashSet<String>,
        sources: Vec<UsageSource>,
        partial: bool,
    ) -> Result<UsageReport> {
        let mut report = UsageReport {
            total: Tokens::default(),
            buckets: intervals
                .iter()
                .map(|b| Bucket {
                    start: b.start,
                    end: b.end,
                    tokens: Tokens::default(),
                })
                .collect(),
            agents: vec![],
            sources,
            scanned_at: chrono::Utc::now().timestamp(),
            partial,
        };
        let connection = self.connection.lock().unwrap();
        let mut statement=connection.prepare("SELECT scope,profile_id,provider_id,at,total,input,output,cache_read,cache_write,reasoning FROM usage_events WHERE at>=?1 AND at<?2")?;
        let mut rows = statement.query(params![
            intervals.first().unwrap().start,
            intervals.last().unwrap().end
        ])?;
        let mut agents: HashMap<String, AgentUsage> = HashMap::new();
        while let Some(row) = rows.next()? {
            let scope: String = row.get(0)?;
            if !scopes.contains(&scope) {
                continue;
            }
            let profile_id: String = row.get(1)?;
            let provider_id: String = row.get(2)?;
            let at: i64 = row.get(3)?;
            let tokens = Tokens {
                total: row.get(4)?,
                input: row.get(5)?,
                output: row.get(6)?,
                cache_read: row.get(7)?,
                cache_write: row.get(8)?,
                reasoning: row.get(9)?,
            };
            let index = report.buckets.partition_point(|b| b.end <= at);
            if let Some(bucket) = report
                .buckets
                .get_mut(index)
                .filter(|b| at >= b.start && at < b.end)
            {
                bucket.tokens.add(&tokens);
                report.total.add(&tokens);
                agents
                    .entry(profile_id.clone())
                    .or_insert(AgentUsage {
                        profile_id,
                        provider_id,
                        tokens: Tokens::default(),
                    })
                    .tokens
                    .add(&tokens);
            }
        }
        report.agents = agents.into_values().collect();
        report
            .agents
            .sort_by_key(|a| std::cmp::Reverse(a.tokens.total));
        Ok(report)
    }
}
pub fn validate_intervals(intervals: &[Interval], s: &Settings, now: i64) -> Result<()> {
    ensure!(
        !intervals.is_empty() && intervals.len() <= s.usage_max_buckets,
        "Invalid token time buckets"
    );
    ensure!(
        intervals.iter().all(|b| b.start < b.end),
        "Invalid token time range"
    );
    ensure!(
        intervals
            .windows(2)
            .all(|pair| pair[0].end == pair[1].start),
        "Token time buckets must be contiguous"
    );
    ensure!(
        intervals[0].start >= now - (s.usage_history_days as i64 + 1) * 86400
            && intervals.last().unwrap().end - intervals[0].start
                <= (s.usage_history_days as i64 + 1) * 86400,
        "Requested token history exceeds configured retention"
    );
    ensure!(
        intervals.last().unwrap().end <= now + 86400,
        "Token time range is in the future"
    );
    Ok(())
}
fn ingest_file(
    store: &Store,
    profile: &Profile,
    provider: &Provider,
    scope: &str,
    path: &Path,
    budget: &mut u64,
    s: &Settings,
    cutoff: i64,
) -> Result<(bool, u64)> {
    let metadata = path.metadata()?;
    let length = metadata.len();
    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .min(u64::MAX as u128) as u64;
    let path_key = format!("{scope}\0{}", path.to_string_lossy());
    let mut state = store.usage_cursor(&path_key)?;
    if state.length == length && state.modified == modified {
        return Ok((false, state.warnings));
    }
    let mut file = File::open(path)?;
    let mut prefix = vec![0; length.min(256) as usize];
    file.read_exact(&mut prefix)?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    prefix.hash(&mut hasher);
    let fingerprint = hasher.finish();
    if state.offset > length
        || (state.fingerprint != 0 && state.fingerprint != fingerprint)
        || (state.length == length && state.modified != modified)
    {
        state = Cursor::default();
    }
    if state.session.is_empty() {
        state.session = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
    }
    let mut events = vec![];
    let json = path.extension().is_some_and(|e| e == "json");
    if json {
        if length > *budget {
            return Ok((true, state.warnings + 1));
        }
        file.seek(SeekFrom::Start(0))?;
        *budget -= length;
        let value: Value = serde_json::from_reader((&mut file).take(length))?;
        events.extend(parse_record(&provider.usage_format, &value, &mut state));
        state.offset = length;
    } else {
        file.seek(SeekFrom::Start(state.offset))?;
        let mut reader = BufReader::new(file);
        while *budget > 0 {
            let mut line = vec![];
            let cap = (*budget).min(s.usage_max_line_bytes);
            let n = (&mut reader).take(cap).read_until(b'\n', &mut line)? as u64;
            if n == 0 {
                break;
            }
            *budget -= n;
            let complete = line.last() == Some(&b'\n');
            if !complete && !state.skipping && (n < cap || cap < s.usage_max_line_bytes) {
                break;
            }
            state.offset += n;
            if state.skipping {
                if complete {
                    state.skipping = false;
                }
                continue;
            }
            if !complete {
                state.skipping = true;
                state.warnings += 1;
                continue;
            }
            if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                events.extend(parse_record(&provider.usage_format, &value, &mut state));
            } else {
                state.warnings += 1;
            }
        }
    }
    events.retain(|event| event.at >= cutoff);
    state.length = if state.offset == length { length } else { 0 };
    state.modified = modified;
    state.fingerprint = fingerprint;
    store.save_usage_batch(scope, profile, provider, &path_key, &state, &events)?;
    Ok((state.offset < length || state.skipping, state.warnings))
}
/// Scan configured local metadata, using a shared per-request byte budget.
pub fn collect(
    store: &Store,
    profiles: &[Profile],
    providers: &[Provider],
    s: &Settings,
    home: &Path,
    env: &HashMap<String, String>,
    intervals: &[Interval],
) -> Result<UsageReport> {
    let now = chrono::Utc::now().timestamp();
    validate_intervals(intervals, s, now)?;
    store.init_usage()?;
    {
        let mut connection = store.connection.lock().unwrap();
        let retention = s.usage_history_days.to_string();
        let previous: Option<String> = connection
            .query_row(
                "SELECT value FROM usage_meta WHERE key='retentionDays'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if previous.as_ref() != Some(&retention) {
            let tx = connection.transaction()?;
            tx.execute("DELETE FROM usage_cursors", [])?;
            tx.execute("INSERT INTO usage_meta(key,value) VALUES('retentionDays',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[retention])?;
            tx.commit()?;
        }
    }
    // One-day boundary padding covers local midnight and daylight-saving transitions.
    let cutoff = now - (s.usage_history_days as i64 + 1) * 86400;
    store
        .connection
        .lock()
        .unwrap()
        .execute("DELETE FROM usage_events WHERE at<?1", [cutoff])?;
    let mut sources = vec![];
    let mut scopes = HashSet::new();
    let mut seen = HashSet::new();
    let mut bytes = s.usage_max_bytes_per_scan;
    let mut partial = false;
    for profile in profiles.iter().filter(|p| p.enabled) {
        let Some(provider) = providers.iter().find(|p| p.id == profile.provider_id) else {
            continue;
        };
        let mut source = UsageSource {
            profile_id: profile.id.clone(),
            provider_id: provider.id.clone(),
            status: if provider.usage_log_paths.is_empty() {
                "unsupported"
            } else {
                "noLogs"
            }
            .into(),
            files: 0,
            warnings: 0,
            has_data: false,
        };
        for root in crate::external_sessions::profile_roots(profile, provider, home, env) {
            let root = root.canonicalize().unwrap_or(root);
            let source_key = format!("{}\0{}", provider.id, root.to_string_lossy());
            let scope = format!("{}\0{source_key}", profile.id);
            if !seen.insert(source_key) {
                source.status = "sharedSource".into();
                continue;
            }
            scopes.insert(scope.clone());
            let mut files = vec![];
            let mut discovery_budget = s.usage_max_files.saturating_mul(20);
            for pattern in &provider.usage_log_paths {
                let parts: Vec<_> = pattern.split(['/', '\\']).collect();
                crate::external_sessions::bounded_paths(
                    &root,
                    &parts,
                    &mut discovery_budget,
                    &mut files,
                );
            }
            files.sort();
            files.dedup();
            files.sort_by_key(|p| std::cmp::Reverse(p.metadata().and_then(|m| m.modified()).ok()));
            if files.len() > s.usage_max_files || discovery_budget == 0 {
                partial = true;
                source.warnings += 1;
                source.status = "partial".into();
            }
            source.files += files.len().min(s.usage_max_files);
            for path in files.into_iter().take(s.usage_max_files) {
                if bytes == 0 {
                    partial = true;
                    source.status = "partial".into();
                    break;
                }
                match ingest_file(
                    store, profile, provider, &scope, &path, &mut bytes, s, cutoff,
                ) {
                    Ok((incomplete, warnings)) => {
                        partial |= incomplete || warnings > 0;
                        source.warnings += warnings;
                        if source.status != "partial" {
                            source.status = if incomplete || warnings > 0 {
                                "partial"
                            } else {
                                "ready"
                            }
                            .into();
                        }
                    }
                    Err(_) => {
                        source.warnings += 1;
                        source.status = "partial".into();
                        partial = true;
                    }
                }
            }
            source.has_data |= store.connection.lock().unwrap().query_row(
                "SELECT EXISTS(SELECT 1 FROM usage_events WHERE scope=?1)",
                [&scope],
                |row| row.get::<_, bool>(0),
            )?;
        }
        if source.status == "noLogs" && source.has_data {
            source.status = "retained".into();
        }
        if source.status == "ready" && !source.has_data {
            source.status = "noUsage".into();
        }
        sources.push(source);
    }
    store.usage_report(intervals, &scopes, sources, partial)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_json_consumes_the_shared_byte_budget() {
        let temp = tempfile::tempdir().unwrap();
        let cfg = crate::config::load().unwrap();
        let provider = cfg.providers.iter().find(|p| p.id == "gemini").unwrap();
        let store = Store::open(&temp.path().join("s.db"), &cfg).unwrap();
        store.init_usage().unwrap();
        let profile = store
            .profiles()
            .unwrap()
            .into_iter()
            .find(|p| p.id == "gemini")
            .unwrap();
        let path = temp.path().join("broken.json");
        std::fs::write(&path, "x".repeat(1000)).unwrap();
        let mut budget = 4096;
        assert!(ingest_file(
            &store,
            &profile,
            provider,
            "scope",
            &path,
            &mut budget,
            &cfg.settings,
            0
        )
        .is_err());
        assert_eq!(budget, 3096);
    }
}
