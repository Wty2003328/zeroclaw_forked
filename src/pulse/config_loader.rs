use crate::pulse::config::AppConfig;
use anyhow::{Context, Result};
use std::path::Path;

pub fn load_config(path: &Path) -> Result<AppConfig> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let expanded =
        shellexpand::env_with_context_no_errors(&raw, |var| std::env::var(var).ok()).to_string();
    let config: AppConfig =
        serde_yaml::from_str(&expanded).with_context(|| "Failed to parse config YAML")?;
    Ok(config)
}
