//! Read-only quota probe. Does not start an inference turn or print credentials.
use agent_orbit::{config, model::Profile, providers};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = config::load()?;
    let id = std::env::args().nth(1).unwrap_or_else(|| "codex".into());
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider"))?;
    let profile = Profile {
        id: id.clone(),
        provider_id: id,
        label: provider.name.clone(),
        executable: String::new(),
        args: vec![],
        config_dir: String::new(),
        quota_command: String::new(),
        quota_args: vec![],
        quota_file: String::new(),
        enabled: true,
    };
    let snapshot = providers::refresh(&profile, provider, &cfg, &cfg.settings).await;
    println!(
        "{}",
        serde_json::json!({"provider":provider.name,"status":snapshot.status,"source":snapshot.source,"windows":snapshot.windows.len(),"message":snapshot.message})
    );
    Ok(())
}
