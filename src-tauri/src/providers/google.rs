use super::*;
use crate::discovery::credential_paths;
use serde_json::json;
pub async fn collect(
    p: &Profile,
    provider: &Provider,
    cfg: &Config,
    s: &Settings,
) -> Result<QuotaSnapshot> {
    let Some(cred) = credential_paths(p, provider)
        .iter()
        .find_map(|path| read_json(path).ok())
    else {
        return Ok(QuotaSnapshot::state(&p.id,if discover(p,provider).installed{"notLoggedIn"}else{"notInstalled"},"Sign in with an eligible Gemini Code Assist account; personal accounts may require Antigravity"));
    };
    let Some(token) = cred["access_token"].as_str() else {
        return Ok(QuotaSnapshot::state(
            &p.id,
            "unsupported",
            "Quota requires a supported Google OAuth account",
        ));
    };
    if cred["expiry_date"]
        .as_i64()
        .is_some_and(|t| t < chrono::Utc::now().timestamp_millis())
    {
        return Ok(QuotaSnapshot::state(
            &p.id,
            "notLoggedIn",
            "Google login expired; open the original CLI to renew it",
        ));
    }
    let base = endpoint(cfg, "googleBase")?;
    let client = http(s)?;
    let data=response(client.post(format!("{base}:loadCodeAssist")).bearer_auth(token).json(&json!({"metadata":{"ideType":"IDE_UNSPECIFIED","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}})).send().await.map_err(|_|anyhow::anyhow!("Cannot reach Google Code Assist"))?).await?;
    let project = data["cloudaicompanionProject"]
        .as_str()
        .or_else(|| data["cloudaicompanionProject"]["id"].as_str())
        .ok_or_else(|| {
            anyhow::anyhow!("This Google account has no eligible Code Assist project")
        })?;
    let quota = response(
        client
            .post(format!("{base}:retrieveUserQuota"))
            .bearer_auth(token)
            .json(&json!({"project":project}))
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Cannot retrieve Google quota"))?,
    )
    .await?;
    let mut out = normalized(
        p,
        "google",
        &quota,
        "Google Code Assist (internal quota API)",
    );
    out.plan = data["currentTier"]["name"].as_str().map(str::to_owned);
    Ok(out)
}
