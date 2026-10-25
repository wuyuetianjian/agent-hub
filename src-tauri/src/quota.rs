use crate::model::QuotaWindow;
use serde_json::Value;
pub fn remaining_percent(used: Option<f64>) -> Option<f64> {
    used.filter(|x| x.is_finite() && *x >= 0.0)
        .map(|x| (100.0 - x).max(0.0))
}
fn percent(v: &Value) -> Option<f64> {
    v.as_f64()
        .filter(|x| x.is_finite() && (0.0..=100.0).contains(x))
}
fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .filter(|n| n.is_finite() && *n >= 0.0)
}
fn reset(v: &Value) -> Option<i64> {
    v.as_i64().filter(|t| *t > 0).or_else(|| {
        chrono::DateTime::parse_from_rfc3339(v.as_str()?)
            .ok()
            .map(|d| d.timestamp())
    })
}
fn window(label: String, remaining: Option<f64>, resets_at: Option<i64>) -> QuotaWindow {
    QuotaWindow {
        label,
        remaining_percent: remaining,
        used: None,
        limit: None,
        unit: "percent".into(),
        resets_at,
        window_minutes: None,
    }
}
pub fn normalize_quota(provider: &str, value: &Value) -> Vec<QuotaWindow> {
    let mut rows = vec![];
    match provider {
        "codex" => {
            let single = value.get("rateLimits").unwrap_or(value);
            let buckets: Vec<(String, &Value)> = if let Some(map) = value
                .get("rateLimitsByLimitId")
                .and_then(Value::as_object)
                .filter(|m| !m.is_empty())
            {
                map.iter().map(|(k, v)| (k.clone(), v)).collect()
            } else {
                vec![("Codex".into(), single)]
            };
            for (bucket, v) in buckets {
                for key in ["primary", "secondary"] {
                    if let Some(w) = v.get(key).filter(|x| x.is_object()) {
                        let name = w["windowDurationMins"]
                            .as_u64()
                            .map(|m| {
                                if m >= 1440 {
                                    format!("{}d", m / 1440)
                                } else {
                                    format!("{}h", m / 60)
                                }
                            })
                            .unwrap_or_else(|| key.into());
                        let mut row = window(
                            format!("{bucket} · {name}"),
                            remaining_percent(w["usedPercent"].as_f64()),
                            reset(&w["resetsAt"]),
                        );
                        row.window_minutes = w["windowDurationMins"].as_u64();
                        rows.push(row);
                    }
                }
            }
        }
        "claude" => {
            let data = value.get("rate_limits").unwrap_or(value);
            for (key, label) in [
                ("five_hour", "5h"),
                ("seven_day", "7d"),
                ("seven_day_sonnet", "Sonnet · 7d"),
                ("seven_day_opus", "Opus · 7d"),
                ("spend_limit", "Spend"),
            ] {
                if let Some(w) = data.get(key).filter(|x| x.is_object()) {
                    let usage = w
                        .get("utilization")
                        .or_else(|| w.get("used_percentage"))
                        .and_then(Value::as_f64);
                    let mut row = window(
                        label.into(),
                        remaining_percent(usage),
                        reset(&w["resets_at"]),
                    );
                    row.window_minutes = match key {
                        "five_hour" => Some(300),
                        "seven_day" | "seven_day_sonnet" | "seven_day_opus" => Some(10080),
                        _ => None,
                    };
                    rows.push(row);
                }
            }
        }
        "google" => {
            if let Some(buckets) = value["buckets"].as_array() {
                for b in buckets {
                    let f = b["remainingFraction"]
                        .as_f64()
                        .filter(|v| (0.0..=1.0).contains(v));
                    let label = format!(
                        "{} · {}",
                        b["modelId"].as_str().unwrap_or("Model"),
                        b["tokenType"].as_str().unwrap_or("quota")
                    );
                    rows.push(window(label, f.map(|x| x * 100.0), reset(&b["resetTime"])));
                }
            }
        }
        "copilot" => {
            if let Some(buckets) = value["quota_snapshots"].as_object() {
                for (name, b) in buckets {
                    if b["unlimited"].as_bool() == Some(true) {
                        continue;
                    }
                    let mut w = window(
                        name.clone(),
                        percent(&b["percent_remaining"]),
                        reset(&value["quota_reset_date"]),
                    );
                    w.limit = number(&b["entitlement"]);
                    let rem = number(&b["remaining"]);
                    w.used = w
                        .limit
                        .zip(rem)
                        .map(|(limit, left)| (limit - left).max(0.0));
                    w.unit = "requests".into();
                    rows.push(w);
                }
            }
        }
        _ => {
            if let Some(windows) = value["windows"].as_array() {
                for w in windows {
                    let mut row = window(
                        w["label"].as_str().unwrap_or("Quota").into(),
                        percent(&w["remainingPercent"]),
                        reset(&w["resetsAt"]),
                    );
                    row.window_minutes = w["windowMinutes"].as_u64();
                    row.used = number(&w["used"]);
                    row.limit = number(&w["limit"]);
                    row.unit = w["unit"].as_str().unwrap_or("percent").into();
                    rows.push(row);
                }
            }
        }
    }
    rows
}
pub fn retain_bridge_snapshot(
    previous: Option<crate::model::QuotaSnapshot>,
    incoming: crate::model::QuotaSnapshot,
) -> crate::model::QuotaSnapshot {
    if incoming.status != "ready" && incoming.windows.is_empty() {
        if let Some(mut old) = previous.filter(|old| {
            old.profile_id == incoming.profile_id
                && old.source == "Claude Code status-line bridge"
                && !old.windows.is_empty()
        }) {
            old.status = "stale".into();
            old.message = incoming.message;
            return old;
        }
    }
    incoming
}
