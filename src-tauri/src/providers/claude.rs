use super::*;
use crate::discovery::credential_paths;
pub async fn collect(
    p: &Profile,
    provider: &Provider,
    cfg: &Config,
    s: &Settings,
) -> Result<QuotaSnapshot> {
    let mut credential = credential_paths(p, provider)
        .iter()
        .find_map(|path| read_json(path).ok());
    #[cfg(target_os = "macos")]
    if credential.is_none() && p.config_dir.is_empty() {
        if let Some(service) = cfg.headers.get("claudeKeychainService") {
            let output = Command::new("security")
                .args(["find-generic-password", "-s", service, "-w"])
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .output()
                .await;
            if let Ok(out) = output {
                if out.status.success() {
                    credential = serde_json::from_slice(&out.stdout).ok();
                }
            }
        }
    }
    let Some(cred) = credential else {
        return Ok(QuotaSnapshot::state(
            &p.id,
            if discover(p, provider).installed {
                "notLoggedIn"
            } else {
                "notInstalled"
            },
            "Start Claude Code to sign in, or use the status-line bridge",
        ));
    };
    let Some(token) = cred["claudeAiOauth"]["accessToken"].as_str() else {
        return Ok(QuotaSnapshot::state(
            &p.id,
            "unsupported",
            "This login does not expose a Claude subscription token; use the status-line bridge",
        ));
    };
    if cred["claudeAiOauth"]["expiresAt"]
        .as_i64()
        .is_some_and(|t| t < chrono::Utc::now().timestamp_millis())
    {
        return Ok(QuotaSnapshot::state(
            &p.id,
            "notLoggedIn",
            "Claude login expired; launch Claude Code to renew it",
        ));
    }
    let endpoint = endpoint(cfg, "claudeUsage")?;
    let res = http(s)?
        .get(endpoint)
        .bearer_auth(token)
        .header(
            "anthropic-beta",
            cfg.headers.get("claudeBeta").cloned().unwrap_or_default(),
        )
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("Cannot reach Claude quota service"))?;
    let data = response(res).await?;
    let mut result = normalized(p, "claude", &data, "Claude account usage (OAuth)");
    result.plan = cred["claudeAiOauth"]["subscriptionType"]
        .as_str()
        .map(str::to_owned);
    Ok(result)
}
