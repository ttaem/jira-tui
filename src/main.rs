mod jira;
mod ui;
mod config;

use anyhow::Result;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = match Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    ui::run_app_with_config(config).await
}
