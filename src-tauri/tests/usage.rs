use agent_orbit::{
    config,
    model::Profile,
    store::Store,
    usage::{self, parse_record, Cursor, Interval},
};
use serde_json::json;
use std::collections::HashMap;
fn profile() -> Profile {
    Profile {
        id: "codex".into(),
        provider_id: "codex".into(),
        label: "Work".into(),
        executable: String::new(),
        args: vec![],
        config_dir: String::new(),
        quota_command: String::new(),
        quota_args: vec![],
        quota_file: String::new(),
        enabled: true,
    }
}
fn codex(total: u64, input: u64, output: u64) -> serde_json::Value {
    json!({"timestamp":"2026-10-25T02:00:00Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":total,"input_tokens":input,"output_tokens":output,"cached_input_tokens":input/2,"reasoning_output_tokens":output/2}}}})
}
#[test]
fn cumulative_codex_counts_become_deltas_without_double_counting_cached_or_reasoning() {
    let mut state = Cursor::default();
    let first = parse_record("codex", &codex(120, 100, 20), &mut state);
    assert_eq!(first[0].tokens.total, 120);
    assert_eq!(first[0].tokens.cache_read, 50);
    assert!(parse_record("codex", &codex(120, 100, 20), &mut state).is_empty());
    let next = parse_record("codex", &codex(180, 150, 30), &mut state);
    assert_eq!(next[0].tokens.total, 60);
    assert_eq!(next[0].tokens.input, 50);
    let reset = parse_record("codex", &codex(12, 10, 2), &mut state);
    assert_eq!(reset[0].tokens.total, 12);
}
#[test]
fn claude_input_includes_cache_and_gemini_output_includes_thoughts_once() {
    let mut state = Cursor::default();
    let event = parse_record(
        "claude",
        &json!({"type":"assistant","timestamp":"2026-10-25T02:00:00Z","message":{"id":"message","usage":{"input_tokens":20,"cache_read_input_tokens":100,"cache_creation_input_tokens":30,"output_tokens":10}}}),
        &mut state,
    );
    assert_eq!(event[0].tokens.input, 150);
    assert_eq!(event[0].tokens.total, 160);
    let event = parse_record(
        "gemini",
        &json!({"id":"g","timestamp":"2026-10-25T02:00:00Z","tokens":{"input":100,"output":20,"thoughts":5,"cached":40,"total":125}}),
        &mut state,
    );
    assert_eq!(event[0].tokens.total, 125);
    assert_eq!(event[0].tokens.output, 25);
}
#[test]
fn repeated_scans_archived_copies_and_restart_do_not_duplicate_usage() {
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let p = profile();
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == "codex")
        .unwrap()
        .clone();
    let root = t.path().join(".codex/sessions/2026/10/25");
    std::fs::create_dir_all(&root).unwrap();
    let now = chrono::Utc::now();
    let timestamp = now.to_rfc3339();
    let mut event = codex(120, 100, 20);
    event["timestamp"] = timestamp.clone().into();
    let text = format!(
        "{}\n{}\n",
        json!({"type":"session_meta","payload":{"id":"s"}}),
        event
    );
    let path = root.join("rollout.jsonl");
    std::fs::write(&path, &text).unwrap();
    let intervals = [Interval {
        start: now.timestamp() - 3600,
        end: now.timestamp() + 3600,
    }];
    let db = t.path().join("history.db");
    {
        let store = Store::open(&db, &cfg).unwrap();
        let first = usage::collect(
            &store,
            &[p.clone()],
            &[provider.clone()],
            &cfg.settings,
            t.path(),
            &HashMap::new(),
            &intervals,
        )
        .unwrap();
        assert_eq!(first.total.total, 120);
        let second = usage::collect(
            &store,
            &[p.clone()],
            &[provider.clone()],
            &cfg.settings,
            t.path(),
            &HashMap::new(),
            &intervals,
        )
        .unwrap();
        assert_eq!(second.total.total, 120);
        std::fs::create_dir_all(t.path().join(".codex/archived_sessions")).unwrap();
        std::fs::write(
            t.path().join(".codex/archived_sessions/rollout.jsonl"),
            text,
        )
        .unwrap();
    }
    let store = Store::open(&db, &cfg).unwrap();
    let report = usage::collect(
        &store,
        &[p],
        &[provider],
        &cfg.settings,
        t.path(),
        &HashMap::new(),
        &intervals,
    )
    .unwrap();
    assert_eq!(report.total.total, 120);
}
#[test]
fn partial_trailing_jsonl_line_is_read_when_completed() {
    use std::io::Write;
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let p = profile();
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == "codex")
        .unwrap()
        .clone();
    let root = t.path().join(".codex/sessions/2026/10/25");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("p.jsonl");
    let mut value = codex(12, 10, 2);
    value["timestamp"] = chrono::Utc::now().to_rfc3339().into();
    let event = value.to_string();
    std::fs::write(&path, &event[..event.len() / 2]).unwrap();
    let store = Store::open(&t.path().join("h.db"), &cfg).unwrap();
    let now = chrono::Utc::now().timestamp();
    let ranges = [Interval {
        start: now - 60,
        end: now + 60,
    }];
    let report = usage::collect(
        &store,
        &[p.clone()],
        &[provider.clone()],
        &cfg.settings,
        t.path(),
        &HashMap::new(),
        &ranges,
    )
    .unwrap();
    assert_eq!(report.total.total, 0);
    assert!(report.partial);
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(file, "{}", &event[event.len() / 2..]).unwrap();
    let report = usage::collect(
        &store,
        &[p],
        &[provider],
        &cfg.settings,
        t.path(),
        &HashMap::new(),
        &ranges,
    )
    .unwrap();
    assert_eq!(report.total.total, 12);
}
#[test]
fn overlapping_buckets_are_rejected() {
    let s = config::load().unwrap().settings;
    let now = chrono::Utc::now().timestamp();
    assert!(usage::validate_intervals(
        &[
            Interval {
                start: now - 100,
                end: now
            },
            Interval {
                start: now - 20,
                end: now + 50
            }
        ],
        &s,
        now
    )
    .is_err());
}
#[test]
fn increasing_retention_replays_unchanged_history() {
    let temp = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let root = temp.path().join(".codex/sessions/2026/10/25");
    std::fs::create_dir_all(&root).unwrap();
    let now = chrono::Utc::now().timestamp();
    let mut old = codex(40, 30, 10);
    old["timestamp"] = chrono::DateTime::from_timestamp(now - 3 * 86400, 0)
        .unwrap()
        .to_rfc3339()
        .into();
    std::fs::write(root.join("old.jsonl"), format!("{old}\n")).unwrap();
    let store = Store::open(&temp.path().join("h.db"), &cfg).unwrap();
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == "codex")
        .unwrap()
        .clone();
    let mut settings = cfg.settings.clone();
    settings.usage_history_days = 1;
    usage::collect(
        &store,
        &[profile()],
        &[provider.clone()],
        &settings,
        temp.path(),
        &HashMap::new(),
        &[Interval {
            start: now - 60,
            end: now + 60,
        }],
    )
    .unwrap();
    settings.usage_history_days = 7;
    let report = usage::collect(
        &store,
        &[profile()],
        &[provider],
        &settings,
        temp.path(),
        &HashMap::new(),
        &[Interval {
            start: now - 4 * 86400,
            end: now + 60,
        }],
    )
    .unwrap();
    assert_eq!(report.total.total, 40);
}
#[test]
fn budget_boundary_does_not_discard_a_valid_following_record() {
    let temp = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let mut settings = cfg.settings.clone();
    settings.usage_max_bytes_per_scan = 4096;
    settings.usage_max_line_bytes = 4096;
    let root = temp.path().join(".codex/sessions/2026/10/25");
    std::fs::create_dir_all(&root).unwrap();
    let mut event = codex(120, 100, 20);
    event["timestamp"] = chrono::Utc::now().to_rfc3339().into();
    event["padding"] = "y".repeat(1000).into();
    std::fs::write(
        root.join("budget.jsonl"),
        format!("{}\n{event}\n", json!({"padding":"x".repeat(3300)})),
    )
    .unwrap();
    let store = Store::open(&temp.path().join("h.db"), &cfg).unwrap();
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == "codex")
        .unwrap()
        .clone();
    let now = chrono::Utc::now().timestamp();
    let ranges = [Interval {
        start: now - 60,
        end: now + 60,
    }];
    let first = usage::collect(
        &store,
        &[profile()],
        &[provider.clone()],
        &settings,
        temp.path(),
        &HashMap::new(),
        &ranges,
    )
    .unwrap();
    assert!(first.partial);
    assert_eq!(first.total.total, 0);
    let second = usage::collect(
        &store,
        &[profile()],
        &[provider],
        &settings,
        temp.path(),
        &HashMap::new(),
        &ranges,
    )
    .unwrap();
    assert_eq!(second.total.total, 120);
    assert!(!second.partial);
}
#[test]
fn changing_account_root_excludes_previous_account_history() {
    let temp = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let root = temp.path().join(".codex/sessions/2026/10/25");
    std::fs::create_dir_all(&root).unwrap();
    let mut event = codex(12, 10, 2);
    event["timestamp"] = chrono::Utc::now().to_rfc3339().into();
    std::fs::write(root.join("account.jsonl"), format!("{event}\n")).unwrap();
    let store = Store::open(&temp.path().join("h.db"), &cfg).unwrap();
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == "codex")
        .unwrap()
        .clone();
    let now = chrono::Utc::now().timestamp();
    let ranges = [Interval {
        start: now - 60,
        end: now + 60,
    }];
    let mut account = profile();
    assert_eq!(
        usage::collect(
            &store,
            &[account.clone()],
            &[provider.clone()],
            &cfg.settings,
            temp.path(),
            &HashMap::new(),
            &ranges
        )
        .unwrap()
        .total
        .total,
        12
    );
    let other = temp.path().join("different-account");
    std::fs::create_dir_all(&other).unwrap();
    account.config_dir = other.to_string_lossy().into_owned();
    let report = usage::collect(
        &store,
        &[account],
        &[provider],
        &cfg.settings,
        temp.path(),
        &HashMap::new(),
        &ranges,
    )
    .unwrap();
    assert_eq!(report.total.total, 0);
    assert!(!report.sources[0].has_data);
}
#[test]
fn claude_streamed_snapshots_count_one_message_at_its_maximum() {
    use std::collections::HashSet;
    let temp = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let store = Store::open(&temp.path().join("h.db"), &cfg).unwrap();
    store.init_usage().unwrap();
    let provider = cfg.providers.iter().find(|p| p.id == "claude").unwrap();
    let mut profile = profile();
    profile.provider_id = "claude".into();
    let mut state = Cursor::default();
    let now = chrono::Utc::now();
    let mut record = json!({"type":"assistant","timestamp":now.to_rfc3339(),"message":{"id":"same-message","usage":{"input_tokens":10,"cache_read_input_tokens":20,"output_tokens":1}}});
    let first = parse_record("claude", &record, &mut state);
    record["message"]["usage"]["output_tokens"] = 5.into();
    let second = parse_record("claude", &record, &mut state);
    store
        .save_usage_batch("scope", &profile, provider, "file", &state, &first)
        .unwrap();
    store
        .save_usage_batch("scope", &profile, provider, "file", &state, &second)
        .unwrap();
    let report = store
        .usage_report(
            &[Interval {
                start: now.timestamp() - 60,
                end: now.timestamp() + 60,
            }],
            &HashSet::from(["scope".into()]),
            vec![],
            false,
        )
        .unwrap();
    assert_eq!(report.total.total, 35);
    assert_eq!(report.total.cache_read, 20);
}
#[test]
fn copied_claude_api_messages_keep_the_same_usage_identity_after_forking() {
    let mut state = Cursor::default();
    let mut record = json!({"type":"assistant","sessionId":"original","timestamp":"2026-10-25T02:00:00Z","message":{"id":"msg_unique_api_call","usage":{"input_tokens":10,"output_tokens":2}}});
    let first = parse_record("claude", &record, &mut state);
    record["sessionId"] = "forked".into();
    let second = parse_record("claude", &record, &mut state);
    assert_eq!(first[0].id, second[0].id);
}
#[test]
fn resumed_codex_log_uses_last_call_instead_of_recounting_prior_cumulative_usage() {
    let mut state = Cursor::default();
    let mut record = codex(1000, 900, 100);
    record["payload"]["info"]["last_token_usage"] =
        json!({"total_tokens":40,"input_tokens":30,"output_tokens":10});
    let first = parse_record("codex", &record, &mut state);
    assert_eq!(first[0].tokens.total, 40);
    let next = parse_record("codex", &codex(1060, 950, 110), &mut state);
    assert_eq!(next[0].tokens.total, 60);
}

#[test]
fn account_timeline_excludes_other_profiles_from_every_time_bucket() {
    let t = tempfile::tempdir().unwrap();
    let cfg = config::load().unwrap();
    let store = Store::open(&t.path().join("usage.db"), &cfg).unwrap();
    let now = chrono::Utc::now();
    let mut accounts = vec![];
    for (id, total) in [("work", 120), ("personal", 600)] {
        let mut p = profile();
        p.id = id.into();
        let root = t.path().join(id);
        let logs = root
            .join("sessions")
            .join(now.format("%Y/%m/%d").to_string());
        std::fs::create_dir_all(&logs).unwrap();
        p.config_dir = root.to_string_lossy().into_owned();
        let mut event = codex(total, total - 20, 20);
        event["timestamp"] = now.to_rfc3339().into();
        std::fs::write(
            logs.join("rollout.jsonl"),
            format!(
                "{}\n{}\n",
                json!({"type":"session_meta","payload":{"id":id}}),
                event
            ),
        )
        .unwrap();
        accounts.push(p);
    }
    let intervals = [Interval {
        start: now.timestamp() - 3600,
        end: now.timestamp() + 3600,
    }];
    let all = usage::collect(
        &store,
        &accounts,
        &cfg.providers,
        &cfg.settings,
        t.path(),
        &HashMap::new(),
        &intervals,
    )
    .unwrap();
    assert_eq!(all.total.total, 720);
    let work = usage::collect(
        &store,
        &accounts[..1],
        &cfg.providers,
        &cfg.settings,
        t.path(),
        &HashMap::new(),
        &intervals,
    )
    .unwrap();
    assert_eq!(work.total.total, 120);
    assert_eq!(work.buckets[0].tokens.total, 120);
    assert_eq!(work.agents.len(), 1);
    assert_eq!(work.agents[0].profile_id, "work");
    assert_eq!(work.sources.len(), 1);
}
