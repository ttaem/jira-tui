use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub jira_email: String,
    pub jira_api_token: String,
    pub jira_base_url: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try to load from .env file first
        if Path::new(".env").exists() {
            if let Ok(content) = fs::read_to_string(".env") {
                for line in content.lines() {
                    if line.trim().is_empty() || line.trim().starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim().trim_start_matches("export ");
                        let value = value.trim().trim_matches('"');
                        env::set_var(key, value);
                    }
                }
            }
        }

        // Get from environment variables
        let jira_email = env::var("JIRA_EMAIL")
            .map_err(|_| anyhow::anyhow!("JIRA_EMAIL environment variable is required"))?;
        
        let jira_api_token = env::var("JIRA_API_TOKEN")
            .map_err(|_| anyhow::anyhow!("JIRA_API_TOKEN environment variable is required"))?;
        
        let jira_base_url = env::var("JIRA_BASE_URL")
            .unwrap_or_else(|_| "https://newracom.atlassian.net".to_string());

        Ok(Config {
            jira_email,
            jira_api_token,
            jira_base_url,
        })
    }
}