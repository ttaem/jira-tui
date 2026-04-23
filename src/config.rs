use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub email: String,
    pub api_token: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_base_url() -> String {
    "https://newracom.atlassian.net".to_string()
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .context("Cannot determine home directory ($HOME is not set)")?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("jira-tui")
            .join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let template = "email = \"your.email@newracom.com\"\n\
                            api_token = \"your-api-token-here\"\n\
                            # base_url = \"https://newracom.atlassian.net\"\n";
            fs::write(&path, template)?;
            return Err(anyhow::anyhow!(
                "Config file created at {}\nEdit it with your credentials and run again.\nGet an API token at: https://id.atlassian.com/manage-profile/security/api-tokens",
                path.display()
            ));
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))
    }
}
