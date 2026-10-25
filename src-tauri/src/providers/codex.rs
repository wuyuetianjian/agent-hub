use super::*;
use crate::{discovery::expand, protocol::RpcClient};
use serde_json::json;
pub async fn collect(
    p: &Profile,
    provider: &Provider,
    _cfg: &Config,
    s: &Settings,
) -> Result<QuotaSnapshot> {
    let found = discover(p, provider);
    let Some(exe) = found.executable else {
        return Ok(QuotaSnapshot::state(
            &p.id,
            if found.installed {
                "unavailable"
            } else {
                "notInstalled"
            },
            if found.installed {
                "Codex installation or account data was detected, but its CLI executable could not be located; configure the executable path"
            } else {
                "Codex CLI was not found"
            },
        ));
    };
    let env = if p.config_dir.is_empty() {
        vec![]
    } else {
        vec![(
            "CODEX_HOME".into(),
            expand(&p.config_dir).to_string_lossy().into_owned(),
        )]
    };
    let mut rpc_args = p.args.clone();
    rpc_args.push("app-server".into());
    let rpc = RpcClient::spawn(&exe, &rpc_args, None, &env, s.request_timeout_seconds).await?;
    let _guard = crate::protocol::RpcCloseGuard::new(rpc.clone());
    let result=async {
  rpc.call("initialize",json!({"clientInfo":{"name":"agent_orbit","title":"Agent Orbit","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}})).await?;
  rpc.send(json!({"method":"initialized"})).await?;
  let account=rpc.call("account/read",json!({"refreshToken":false})).await?;
  if account["account"].is_null(){return Ok(QuotaSnapshot::state(&p.id,"notLoggedIn","Sign in using the Codex CLI"));}
  if account["account"]["type"].as_str()==Some("apiKey"){return Ok(QuotaSnapshot::state(&p.id,"unsupported","API key login does not expose ChatGPT subscription quota"));}
  let limits=rpc.call("account/rateLimits/read",json!({})).await?;
  let mut out=normalized(p,"codex",&limits,"Codex app-server");out.account=account["account"]["email"].as_str().map(str::to_owned);out.plan=account["account"]["planType"].as_str().map(str::to_owned);Ok(out)
 }.await;
    rpc.close();
    result
}
