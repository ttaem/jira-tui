# Copilot Instructions

## Build & Run

```bash
cargo build                 # debug build
cargo build --release       # release build
cargo run                   # run (loads .env automatically)
./run.sh                    # run release binary (requires env vars pre-set)

cargo fmt                   # format
cargo clippy                # lint
```

No tests exist in this project yet.

## Architecture

Four source files with a clear layered flow:

```
main.rs → Config::load() → JiraClient → App (UI event loop)
```

- **`config.rs`** – `Config::load()` reads `~/.config/jira-tui/config.toml` (TOML format). If the file doesn't exist, it auto-creates the directory and a template file, then returns an error prompting the user to edit it. `base_url` has a serde `default` pointing to `https://newracom.atlassian.net`.

- **`jira.rs`** – `JiraClient` wraps `reqwest::Client` and exposes async methods against JIRA REST API v3. Auth is HTTP Basic with base64-encoded `email:api_token`. Contains two layers of structs: private `JiraApi*` types for deserialization from the wire format, and the public `JiraIssue` / `ChangelogEntry` / `Comment` types consumed by the UI. `is_watching`, `changelog`, and `comments` on `JiraIssue` are `None` by default and loaded lazily when a detail popup is shown.

- **`ui.rs`** – `App` struct owns all UI state. `run_app_with_config` is the async entry point. The render path is `ui(frame, app)` → `render_kanban_board` or `render_recent_issues` → optional detail popup. The **Assigned** tab renders a 3-column kanban board (`StatusColumn::Todo / InProgress / Done`) each with its own `ListState`. The **Recent** tab is a flat list.

- **`main.rs`** – Thin: loads config, prints setup help on error, then calls `ui::run_app_with_config(config)`.

## Key Conventions

**Config file format** – TOML at `~/.config/jira-tui/config.toml`. Fields: `email`, `api_token`, `base_url` (optional with serde default). The file is auto-created as a template on first run.

**JIRA API** – Uses `/rest/api/3/search/jql` for listing (paginated via `nextPageToken`). Detail data (changelog, comments) is fetched per-issue on demand via separate endpoints. The JIRA description field is Atlassian Document Format (ADF) JSON — see `extract_text_from_adf()` in `jira.rs` for the recursive text-extraction pattern.

**Status string matching** – The `current_issues()` filter in `ui.rs` matches status strings in both **Korean and English** (e.g., `"해야 할 일" | "To Do"`). When adding new statuses or columns, add both language variants.

**Lazy-loaded fields** – `JiraIssue.is_watching`, `.changelog`, `.comments` start as `None`. `App::load_watch_status()` and `App::load_issue_updates()` populate them after a detail view is opened. Guard with `if issue.changelog.is_none()` before fetching to avoid redundant API calls.

**Kanban list state** – The `Assigned` tab maintains three separate `ListState`s (`todo_list_state`, `inprogress_list_state`, `done_list_state`). `current_list_state()` dispatches based on `current_tab` + `current_column`. When adding a new column, add a corresponding `ListState` field and update both `current_list_state()` and `current_issues()`.

## Configuration

Edit `~/.config/jira-tui/config.toml` (auto-created on first run):

```toml
email = "your.email@newracom.com"
api_token = "your-api-token-here"
# base_url = "https://newracom.atlassian.net"  # optional
```

| Field | Required | Default |
|---|---|---|
| `email` | ✅ | — |
| `api_token` | ✅ | — |
| `base_url` | ❌ | `https://newracom.atlassian.net` |
