use crate::{
    discovery::{discover, expand, read_json},
    model::{Config, Profile, Provider, QuotaSnapshot, Settings},
    quota::normalize_quota,
};
use anyhow::{bail, Result};
use serde_json::Value;
use std::process::Stdio;
use tokio::{io::AsyncReadExt, process::Command};
mod claude;
mod codex;
mod copilot;
mod google;

pub async fn refresh(
    profile: &Profile,
    provider: &Provider,
    config: &Config,
    settings: &Settings,
) -> QuotaSnapshot {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(settings.request_timeout_seconds),
        collect(profile, provider, config, settings),
    )
    .await;
    match result {
        Ok(Ok(snapshot)) => snapshot,
        Ok(Err(e)) => QuotaSnapshot::state(&profile.id, "error", &e.to_string()),
        Err(_) => QuotaSnapshot::state(
            &profile.id,
            "error",
            "Request timed out; the provider or CLI did not respond",
        ),
    }
}
async fn collect(
    p: &Profile,
    provider: &Provider,
    cfg: &Config,
    s: &Settings,
) -> Result<QuotaSnapshot> {
    if !p.quota_file.is_empty() {
        return custom(p, read_json(&expand(&p.quota_file))?, "Local quota file");
    }
    if !p.quota_command.is_empty() {
        let mut cmd = Command::new(expand(&p.quota_command));
        cmd.args(&p.quota_args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        cmd.env("AGENT_ORBIT_PROFILE_ID", &p.id)
            .env("AGENT_ORBIT_PROVIDER", &p.provider_id);
        if !p.config_dir.is_empty() {
            cmd.env("AGENT_ORBIT_PROFILE_DIR", expand(&p.config_dir));
        }
        let mut child = cmd
            .spawn()
            .map_err(|_| anyhow::anyhow!("Cannot launch quota collector"))?;
        let mut out = vec![];
        child
            .stdout
            .take()
            .unwrap()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut out)
            .await?;
        if out.len() > 2 * 1024 * 1024 {
            bail!("Collector output exceeds size limit");
        }
        if !child.wait().await?.success() {
            bail!("Quota collector exited unsuccessfully");
        }
        return custom(
            p,
            serde_json::from_slice(&out)
                .map_err(|_| anyhow::anyhow!("Collector must return one JSON object"))?,
            "Custom collector",
        );
    }
    match provider.adapter.as_str() {
        "codex" => codex::collect(p, provider, cfg, s).await,
        "claude" => claude::collect(p, provider, cfg, s).await,
        "google" => google::collect(p, provider, cfg, s).await,
        "copilot" => copilot::collect(p, provider, cfg, s).await,
        _ => {
            let d = discover(p, provider);
            Ok(QuotaSnapshot::state(
                &p.id,
                if d.installed {
                    "unsupported"
                } else {
                    "notInstalled"
                },
                if d.installed {
                    "Configure a JSON quota collector or open the provider usage page"
                } else {
                    "Agent installation was not detected; configure its executable or profile directory"
                },
            ))
        }
    }
}
pub fn custom(p: &Profile, value: Value, source: &str) -> Result<QuotaSnapshot> {
    if !value["windows"].is_array() {
        bail!("Quota JSON must contain a windows array");
    }
    let mut snapshot = QuotaSnapshot::state(&p.id, "ready", "");
    snapshot.account = value["account"].as_str().map(str::to_owned);
    snapshot.plan = value["plan"].as_str().map(str::to_owned);
    snapshot.source = source.into();
    snapshot.windows = normalize_quota("collector", &value);
    if let Some(time) = value["fetchedAt"].as_i64() {
        snapshot.fetched_at = time;
    }
    if snapshot.windows.is_empty() {
        snapshot.status = "unavailable".into();
        snapshot.message = "No quota windows were returned".into();
    }
    Ok(snapshot)
}
pub fn normalized(p: &Profile, adapter: &str, value: &Value, source: &str) -> QuotaSnapshot {
    let mut result = QuotaSnapshot::state(&p.id, "ready", "");
    result.source = source.into();
    result.windows = normalize_quota(adapter, value);
    if result.windows.is_empty() {
        result.status = "unavailable".into();
        result.message = "The provider returned no account quota windows".into();
    }
    result
}
pub fn http(s: &Settings) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(s.request_timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("agent-orbit/", env!("CARGO_PKG_VERSION")))
        .build()?)
}
pub async fn response(mut res: reqwest::Response) -> Result<Value> {
    let status = res.status();
    if !status.is_success() {
        bail!(match status.as_u16() {
            401 => "Login expired; sign in again through the original CLI",
            403 => "This account is not authorized to read quota",
            429 => "Provider rate limited quota requests; try again later",
            _ => "Provider quota service returned an error",
        });
    }
    if res.content_length().unwrap_or(0) > 2 * 1024 * 1024 {
        bail!("Quota response exceeds size limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = res.chunk().await? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            bail!("Quota response exceeds size limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("Invalid quota response"))?)
}

pub fn endpoint(cfg: &Config, key: &str) -> Result<String> {
    let supplied = cfg
        .endpoints
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("Quota endpoint is missing"))?;
    let defaults: Config = serde_json::from_str(include_str!("../../../config/defaults.json"))?;
    let original = defaults
        .endpoints
        .get(key)
        .ok_or_else(|| anyhow::anyhow!("Unknown credential destination"))?;
    let url = reqwest::Url::parse(supplied)?;
    let trusted = reqwest::Url::parse(original)?;
    if url.scheme() != "https"
        || url.host_str() != trusted.host_str()
        || url.port_or_known_default() != trusted.port_or_known_default()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        bail!("Account credential is not bound to this quota endpoint; use a custom collector for a different service");
    }
    Ok(supplied.clone())
}
