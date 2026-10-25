use crate::model::Config;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::PathBuf;

pub fn merge(base: &mut Value, overlay: Value) {
    if let (Some(a), Some(b)) = (base.as_object_mut(), overlay.as_object()) {
        for (k, v) in b {
            merge(a.entry(k).or_insert(Value::Null), v.clone());
        }
    } else {
        *base = overlay;
    }
}
pub fn load() -> Result<Config> {
    let mut value: Value = serde_json::from_str(include_str!("../../config/defaults.json"))?;
    if let Some(path) = std::env::var_os("AGENT_ORBIT_CONFIG") {
        merge(
            &mut value,
            serde_json::from_slice(
                &std::fs::read(path).context("Cannot read external configuration")?,
            )?,
        );
    }
    if let Ok(seconds) = std::env::var("AGENT_ORBIT_REFRESH_SECONDS") {
        value["settings"]["refreshSeconds"] = seconds
            .parse::<u64>()
            .context("Invalid refresh interval")?
            .into();
    }
    let config: Config = serde_json::from_value(value)?;
    validate_settings(&config.settings)?;
    Ok(config)
}
pub fn data_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("AGENT_ORBIT_DATA_DIR") {
        return Ok(PathBuf::from(path));
    }
    Ok(dirs::data_local_dir()
        .context("No application data directory")?
        .join(env!("CARGO_PKG_NAME")))
}
pub fn validate_settings(s: &crate::model::Settings) -> Result<()> {
    if !(15..=86400).contains(&s.refresh_seconds) {
        bail!("Refresh interval must be between 15 and 86400 seconds");
    }
    if !(3..=120).contains(&s.request_timeout_seconds)
        || !(30..=3600).contains(&s.interaction_timeout_seconds)
    {
        bail!("Invalid request timeout");
    }
    if s.warning_percent > 100 || s.stale_seconds < s.refresh_seconds {
        bail!("Invalid warning or stale threshold");
    }
    if !["zh", "en"].contains(&s.locale.as_str())
        || !["light", "dark", "system"].contains(&s.theme.as_str())
    {
        bail!("Invalid language or theme");
    }
    if s.max_output_bytes < 4096 || s.max_output_bytes > 8 * 1024 * 1024 {
        bail!("Invalid terminal output limit");
    }
    if !["floating", "island"].contains(&s.window_mode.as_str())
        || !["shortest", "lowest"].contains(&s.summary_policy.as_str())
    {
        bail!("Invalid display mode or summary policy");
    }
    for (width, height) in [
        (s.window_width, s.window_height),
        (s.expanded_width, s.expanded_height),
        (s.island_width, s.island_height),
    ] {
        if !width.is_finite()
            || !height.is_finite()
            || !(320.0..=4096.0).contains(&width)
            || !(80.0..=2160.0).contains(&height)
        {
            bail!("Invalid window dimensions");
        }
    }
    Ok(())
}
