use super::*;
use crate::discovery::credential_paths;
pub async fn collect(
    p: &Profile,
    provider: &Provider,
    cfg: &Config,
    s: &Settings,
) -> Result<QuotaSnapshot> {
    let credentials = credential_paths(p, provider)
        .iter()
        .find_map(|path| read_json(path).ok());
    let token = credentials
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|map| map.values().find_map(|v| v["oauth_token"].as_str()));
    let Some(token) = token else {
        return Ok(QuotaSnapshot::state(&p.id,if discover(p,provider).installed{"unsupported"}else{"notInstalled"},"No compatible Copilot credential file found; configure a collector or view account usage"));
    };
    let endpoint = endpoint(cfg, "copilotUser")?;
    let data = response(
        http(s)?
            .get(endpoint)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Cannot reach Copilot quota service"))?,
    )
    .await?;
    let mut out = normalized(p, "copilot", &data, "GitHub Copilot account quota");
    out.account = data["login"].as_str().map(str::to_owned);
    out.plan = data["copilot_plan"].as_str().map(str::to_owned);
    Ok(out)
}
