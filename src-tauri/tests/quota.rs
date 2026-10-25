use agent_orbit::remaining_percent;
#[test]
fn zero_usage_is_full_remaining() {
    assert_eq!(remaining_percent(Some(0.0)), Some(100.0));
}
#[test]
fn missing_usage_is_unknown() {
    assert_eq!(remaining_percent(None), None);
}
#[test]
fn invalid_usage_is_unknown() {
    assert_eq!(remaining_percent(Some(-1.0)), None);
    assert_eq!(remaining_percent(Some(f64::NAN)), None);
}
#[test]
fn exceeded_quota_has_no_remaining() {
    assert_eq!(remaining_percent(Some(112.0)), Some(0.0));
}
use agent_orbit::quota::normalize_quota;
use serde_json::json;
#[test]
fn codex_preserves_distinct_buckets_and_zero() {
    let rows = normalize_quota(
        "codex",
        &json!({"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":0,"windowDurationMins":300,"resetsAt":1000},"secondary":{"usedPercent":100,"windowDurationMins":10080,"resetsAt":2000}}}}),
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].remaining_percent, Some(100.0));
    assert_eq!(rows[1].remaining_percent, Some(0.0));
}
#[test]
fn claude_missing_window_is_not_zero() {
    let rows = normalize_quota(
        "claude",
        &json!({"five_hour":{"utilization":25,"resets_at":"2026-10-24T16:00:00Z"},"seven_day":null}),
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].remaining_percent, Some(75.0));
    assert!(rows[0].resets_at.is_some());
}
#[test]
fn google_fraction_does_not_require_remaining_amount() {
    let rows = normalize_quota(
        "google",
        &json!({"buckets":[{"modelId":"pro","remainingFraction":1.0,"resetTime":"2026-10-25T00:00:00Z"},{"modelId":"flash","remainingFraction":0.0}]}),
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].remaining_percent, Some(100.0));
    assert_eq!(rows[1].remaining_percent, Some(0.0));
    assert_eq!(rows[0].limit, None);
}
#[test]
fn custom_collector_rejects_invalid_percentages() {
    let rows = normalize_quota(
        "collector",
        &json!({"windows":[{"label":"daily","remainingPercent":-5},{"label":"weekly","remainingPercent":null}]}),
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].remaining_percent, None);
    assert_eq!(rows[1].remaining_percent, None);
}
#[test]
fn failed_direct_collection_preserves_a_source_bound_bridge_snapshot_as_stale() {
    use agent_orbit::model::QuotaSnapshot;
    let mut old = QuotaSnapshot::state("claude", "ready", "");
    old.source = "Claude Code status-line bridge".into();
    old.windows = normalize_quota("claude", &json!({"five_hour":{"utilization":20}}));
    old.fetched_at = 123;
    let failed = QuotaSnapshot::state("claude", "notLoggedIn", "Direct OAuth unavailable");
    let merged = agent_orbit::quota::retain_bridge_snapshot(Some(old), failed);
    assert_eq!(merged.status, "stale");
    assert_eq!(merged.fetched_at, 123);
    assert_eq!(merged.windows[0].remaining_percent, Some(80.0));
}
