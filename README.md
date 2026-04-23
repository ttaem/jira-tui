# JIRA TUI

A terminal-based JIRA client built with Rust and ratatui that provides a clean interface to view and manage your JIRA issues from https://newracom.atlassian.net.

## Features

- 📋 View issues assigned to you
- 🕒 View recently updated issues (last 7 days)
- 📖 Detailed issue information popup
- ⌨️ Intuitive keyboard navigation
- 🔄 Real-time issue loading and refresh
- 🎨 Clean, responsive terminal UI
- ⚙️ Flexible configuration (environment variables or .env file)

## Screenshots

```
┌─ JIRA Issues ─────────────────────────────────────────────────────────┐
│ [Assigned] | Recent                                                    │
└───────────────────────────────────────────────────────────────────────┘
┌─ Assigned Issues (5) ─────────────────────────────────────────────────┐
│ > PROJ-123 - Fix critical bug in authentication module                │
│   Status: In Progress | Priority: High | Assignee: John Doe           │
│   PROJ-124 - Implement new user dashboard                             │
│   Status: To Do | Priority: Medium | Assignee: John Doe               │
│   PROJ-125 - Update documentation                                     │
│   Status: In Review | Priority: Low | Assignee: John Doe              │
└───────────────────────────────────────────────────────────────────────┘
┌─ Help ────────────────────────────────────────────────────────────────┐
│ ↑/k: Up | ↓/j: Down | Enter/Space: Details | Tab: Switch | q: Quit    │
└───────────────────────────────────────────────────────────────────────┘
```

## Prerequisites

- Rust (latest stable version)
- JIRA account with API access to https://newracom.atlassian.net
- JIRA API token

## Quick Start

### 1. Clone and Build

```bash
git clone <repository-url>
cd jira-tui
cargo build --release
```

### 2. Setup Configuration

On first run, the app creates a template config file at `~/.config/jira-tui/config.toml`:

```toml
email = "your.email@newracom.com"
api_token = "your-api-token-here"
# base_url = "https://newracom.atlassian.net"
```

Edit the file with your credentials, then run again.

### 3. Create JIRA API Token

1. Go to [Atlassian Account Settings](https://id.atlassian.com/manage-profile/security/api-tokens)
2. Click "Create API token"
3. Give it a label (e.g., "JIRA TUI")
4. Copy the generated token and use it as `JIRA_API_TOKEN`

### 4. Run

```bash
cargo run
# or
./run.sh
# or
./target/release/jira-tui
```

## Usage

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up in the issue list |
| `↓` / `j` | Move down in the issue list |
| `Enter` / `Space` | View detailed issue information |
| `Tab` | Switch between "Assigned" and "Recent" tabs |
| `r` | Refresh issues from JIRA |
| `q` | Quit the application |
| `Esc` | Close issue details popup |

### Tabs

1. **Assigned**: Shows unresolved issues assigned to you
2. **Recent**: Shows all issues updated in the last 7 days

### Issue Details

Press `Enter` or `Space` on any issue to view:
- Issue key and summary
- Status and priority (color-coded)
- Assignee and reporter
- Creation and update timestamps
- Full description

### Status Colors

- 🟢 **Green**: Done, Closed, Resolved
- 🟡 **Yellow**: In Progress, In Review
- 🔵 **Blue**: To Do, Open
- ⚪ **White**: Other statuses

### Priority Colors

- 🔴 **Red**: Highest, High priority
- 🟡 **Yellow**: Medium priority
- 🟢 **Green**: Low, Lowest priority

## Configuration

Edit `~/.config/jira-tui/config.toml` (auto-created on first run):

```toml
email = "your.email@newracom.com"
api_token = "your-api-token-here"
# base_url = "https://newracom.atlassian.net"  # optional
```

### Config Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `email` | ✅ Yes | — | Your JIRA account email |
| `api_token` | ✅ Yes | — | Your JIRA API token |
| `base_url` | ❌ No | `https://newracom.atlassian.net` | JIRA instance URL |

## Troubleshooting

### Authentication Issues

- ✅ Verify your email and API token are correct
- ✅ Ensure your JIRA account has proper permissions
- ✅ Check that your JIRA instance URL is correct
- ✅ Try accessing JIRA through a web browser to confirm credentials

### Network Issues

- ✅ Verify you can access your JIRA instance from your network
- ✅ Check firewall settings if running in a corporate environment
- ✅ Ensure you have internet connectivity

### API Rate Limits

- The application makes minimal API calls, but if you encounter rate limits, wait a few minutes before retrying

### Common Error Messages

**"JIRA_EMAIL environment variable is required"**
- Edit `~/.config/jira-tui/config.toml` with your email and API token

**"JIRA_API_TOKEN environment variable is required"**
- Edit `~/.config/jira-tui/config.toml` with a valid API token

**"API request failed: 401"**
- Invalid credentials, check your email and API token

**"API request failed: 403"**
- Insufficient permissions, contact your JIRA administrator

## Building from Source

```bash
# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone <repository-url>
cd jira-tui

# Build in release mode
cargo build --release

# The binary will be available at target/release/jira-tui
```

## Dependencies

- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI library
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [tokio](https://github.com/tokio-rs/tokio) - Asynchronous runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [serde](https://github.com/serde-rs/serde) - Serialization framework
- [anyhow](https://github.com/dtolnay/anyhow) - Error handling
- [base64](https://github.com/marshallpierce/rust-base64) - Base64 encoding

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

## License

This project is licensed under the MIT License.

## Changelog

### v0.1.0
- Initial release
- Basic JIRA issue viewing
- Assigned and recent issues tabs
- Issue detail popup
- Environment variable and .env file configuration
- Terminal UI with keyboard navigation