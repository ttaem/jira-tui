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
            eprintln!("Configuration Error: {}", e);
            eprintln!();
            eprintln!("Setup Instructions:");
            eprintln!("1. Set environment variables:");
            eprintln!("   export JIRA_EMAIL=your.email@newracom.com");
            eprintln!("   export JIRA_API_TOKEN=your-api-token");
            eprintln!("   export JIRA_BASE_URL=https://newracom.atlassian.net  # optional");
            eprintln!();
            eprintln!("2. Or create a .env file:");
            eprintln!("   cp env.example .env");
            eprintln!("   # Edit .env with your credentials");
            eprintln!();
            eprintln!("3. Get API Token:");
            eprintln!("   https://id.atlassian.com/manage-profile/security/api-tokens");
            std::process::exit(1);
        }
    };

    ui::run_app_with_config(config).await
}
