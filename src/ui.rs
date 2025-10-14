use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use std::io;

use crate::config::Config;
use crate::jira::{JiraClient, JiraIssue};

// Helper function to check if a datetime string is within the last 7 days
fn is_within_last_week(datetime_str: &str) -> bool {
    use std::time::{SystemTime, Duration};
    
    // Parse JIRA datetime format: "2025-10-12T17:25:10.216-0700"
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(datetime_str) {
        let now = SystemTime::now();
        let seven_days_ago = now - Duration::from_secs(7 * 24 * 60 * 60);
        
        // Convert chrono timestamp to SystemTime
        let issue_timestamp = dt.timestamp();
        if issue_timestamp > 0 {
            let duration_since_epoch = Duration::from_secs(issue_timestamp as u64);
            if let Some(issue_time) = std::time::UNIX_EPOCH.checked_add(duration_since_epoch) {
                return issue_time >= seven_days_ago;
            }
        }
    }
    
    // If parsing fails, include the item (safer to show more than less)
    true
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Assigned,
    Recent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum StatusColumn {
    Todo,
    InProgress,
    Done,
}

pub struct App {
    jira_client: JiraClient,
    jira_base_url: String,
    current_tab: Tab,
    current_column: StatusColumn,
    assigned_issues: Vec<JiraIssue>,
    recent_issues: Vec<JiraIssue>,
    assigned_list_state: ListState,
    recent_list_state: ListState,
    todo_list_state: ListState,
    inprogress_list_state: ListState,
    done_list_state: ListState,
    selected_issue: Option<JiraIssue>,
    show_details: bool,
    loading: bool,
    error_message: Option<String>,
}

impl App {
    pub fn new(jira_client: JiraClient, jira_base_url: String) -> Self {
        let mut assigned_list_state = ListState::default();
        assigned_list_state.select(Some(0));
        
        let mut recent_list_state = ListState::default();
        recent_list_state.select(Some(0));

        let mut todo_list_state = ListState::default();
        todo_list_state.select(Some(0));
        
        let mut inprogress_list_state = ListState::default();
        inprogress_list_state.select(Some(0));
        
        let mut done_list_state = ListState::default();
        done_list_state.select(Some(0));

        Self {
            jira_client,
            jira_base_url,
            current_tab: Tab::Assigned,
            current_column: StatusColumn::Todo,
            assigned_issues: Vec::new(),
            recent_issues: Vec::new(),
            assigned_list_state,
            recent_list_state,
            todo_list_state,
            inprogress_list_state,
            done_list_state,
            selected_issue: None,
            show_details: false,
            loading: false,
            error_message: None,
        }
    }

    pub async fn load_issues(&mut self) -> Result<()> {
        self.loading = true;
        self.error_message = None;

        match self.jira_client.get_assigned_issues().await {
            Ok(issues) => {
                self.assigned_issues = issues;
                if !self.assigned_issues.is_empty() && self.current_tab == Tab::Assigned {
                    // Reset column states
                    self.todo_list_state.select(Some(0));
                    self.inprogress_list_state.select(Some(0));
                    self.done_list_state.select(Some(0));
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load assigned issues: {}", e));
            }
        }

        match self.jira_client.get_recent_issues().await {
            Ok(issues) => {
                self.recent_issues = issues;
                if !self.recent_issues.is_empty() && self.current_tab == Tab::Recent {
                    self.recent_list_state.select(Some(0));
                }
            }
            Err(e) => {
                if self.error_message.is_none() {
                    self.error_message = Some(format!("Failed to load recent issues: {}", e));
                }
            }
        }

        self.loading = false;
        Ok(())
    }


    fn current_list_state(&mut self) -> &mut ListState {
        match self.current_tab {
            Tab::Assigned => match self.current_column {
                StatusColumn::Todo => &mut self.todo_list_state,
                StatusColumn::InProgress => &mut self.inprogress_list_state,
                StatusColumn::Done => &mut self.done_list_state,
            },
            Tab::Recent => &mut self.recent_list_state,
        }
    }

    fn current_issues(&self) -> Vec<&JiraIssue> {
        let source_issues = match self.current_tab {
            Tab::Assigned => &self.assigned_issues,
            Tab::Recent => &self.recent_issues,
        };

        if self.current_tab == Tab::Recent {
            return source_issues.iter().collect();
        }

        // Filter by status for assigned tab
        source_issues
            .iter()
            .filter(|issue| {
                match self.current_column {
                    StatusColumn::Todo => matches!(
                        issue.status.as_str(),
                        "해야 할 일" | "To Do" | "Open" | "PENDING"
                    ),
                    StatusColumn::InProgress => matches!(
                        issue.status.as_str(),
                        "진행 중" | "In Progress" | "In Review"
                    ),
                    StatusColumn::Done => matches!(
                        issue.status.as_str(),
                        "해결됨" | "완료" | "Done" | "Closed" | "Resolved"
                    ),
                }
            })
            .collect()
    }

    fn next_issue(&mut self) {
        let issues = self.current_issues();
        let issues_len = issues.len();
        
        if issues_len == 0 {
            return;
        }

        let list_state = self.current_list_state();
        let i = match list_state.selected() {
            Some(i) => {
                if i >= issues_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        list_state.select(Some(i));
    }

    fn previous_issue(&mut self) {
        let issues = self.current_issues();
        let issues_len = issues.len();
        
        if issues_len == 0 {
            return;
        }

        let list_state = self.current_list_state();
        let i = match list_state.selected() {
            Some(i) => {
                if i == 0 {
                    issues_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        list_state.select(Some(i));
    }

    fn next_column(&mut self) {
        if self.current_tab == Tab::Assigned {
            self.current_column = match self.current_column {
                StatusColumn::Todo => StatusColumn::InProgress,
                StatusColumn::InProgress => StatusColumn::Done,
                StatusColumn::Done => StatusColumn::Todo,
            };
        }
    }

    fn previous_column(&mut self) {
        if self.current_tab == Tab::Assigned {
            self.current_column = match self.current_column {
                StatusColumn::Todo => StatusColumn::Done,
                StatusColumn::InProgress => StatusColumn::Todo,
                StatusColumn::Done => StatusColumn::InProgress,
            };
        }
    }

    fn show_issue_details(&mut self) {
        let selected_index = match self.current_tab {
            Tab::Assigned => match self.current_column {
                StatusColumn::Todo => self.todo_list_state.selected(),
                StatusColumn::InProgress => self.inprogress_list_state.selected(),
                StatusColumn::Done => self.done_list_state.selected(),
            },
            Tab::Recent => self.recent_list_state.selected(),
        };
        
        if let Some(i) = selected_index {
            let issues = self.current_issues();
            if let Some(issue) = issues.get(i) {
                self.selected_issue = Some((*issue).clone());
                self.show_details = true;
            }
        }
    }

    fn switch_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Assigned => Tab::Recent,
            Tab::Recent => Tab::Assigned,
        };
        self.show_details = false;
        self.selected_issue = None;
        self.current_column = StatusColumn::Todo;
    }

    pub fn handle_key_event(&mut self, key: KeyCode) -> bool {
        if self.show_details {
            match key {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.show_details = false;
                    self.selected_issue = None;
                }
                KeyCode::Char('w') => {
                    // Toggle watch status - will be handled in main loop
                    return false;
                }
                _ => {}
            }
            return false;
        }

        match key {
            KeyCode::Char('q') => return true,
            KeyCode::Down | KeyCode::Char('j') => self.next_issue(),
            KeyCode::Up | KeyCode::Char('k') => self.previous_issue(),
            KeyCode::Left | KeyCode::Char('h') => self.previous_column(),
            KeyCode::Right | KeyCode::Char('l') => self.next_column(),
            KeyCode::Enter | KeyCode::Char(' ') => self.show_issue_details(),
            KeyCode::Tab => self.switch_tab(),
            KeyCode::Char('r') => {
                // Refresh issues - this would need to be handled in the main loop
            }
            _ => {}
        }
        false
    }

    pub async fn load_watch_status(&mut self, issue_key: &str) -> Result<()> {
        if let Some(ref mut issue) = self.selected_issue {
            if issue.key == issue_key {
                match self.jira_client.get_watch_status(issue_key).await {
                    Ok(is_watching) => {
                        issue.is_watching = Some(is_watching);
                    }
                    Err(_) => {
                        // If we can't get watch status, assume not watching
                        issue.is_watching = Some(false);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_watch(&mut self) -> Result<String> {
        if let Some(ref issue) = self.selected_issue.clone() {
            let issue_key = issue.key.clone();
            
            // Get current watch status if not loaded
            if issue.is_watching.is_none() {
                self.load_watch_status(&issue_key).await?;
            }

            let current_watching = self.selected_issue.as_ref()
                .and_then(|i| i.is_watching)
                .unwrap_or(false);
            
            let result = if current_watching {
                match self.jira_client.unwatch_issue(&issue_key).await {
                    Ok(_) => {
                        if let Some(ref mut issue) = self.selected_issue {
                            issue.is_watching = Some(false);
                        }
                        format!("Stopped watching {}", issue_key)
                    }
                    Err(e) => format!("Failed to unwatch {}: {}", issue_key, e),
                }
            } else {
                match self.jira_client.watch_issue(&issue_key).await {
                    Ok(_) => {
                        if let Some(ref mut issue) = self.selected_issue {
                            issue.is_watching = Some(true);
                        }
                        format!("Now watching {}", issue_key)
                    }
                    Err(e) => format!("Failed to watch {}: {}", issue_key, e),
                }
            };

            Ok(result)
        } else {
            Err(anyhow::anyhow!("No issue selected"))
        }
    }

    pub async fn load_issue_updates(&mut self, issue_key: &str) -> Result<()> {
        if let Some(ref mut issue) = self.selected_issue {
            if issue.key == issue_key && issue.changelog.is_none() {
                match self.jira_client.get_issue_updates(issue_key).await {
                    Ok((changelog, comments)) => {
                        issue.changelog = Some(changelog);
                        issue.comments = Some(comments);
                    }
                    Err(_) => {
                        // If we can't get updates, set empty lists
                        issue.changelog = Some(Vec::new());
                        issue.comments = Some(Vec::new());
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());

    // Render tabs
    let tab_text = format!(
        "{}{}{}",
        if app.current_tab == Tab::Assigned { "[Assigned]" } else { " Assigned " },
        " | ",
        if app.current_tab == Tab::Recent { "[Recent]" } else { " Recent " }
    );
    
    let tabs = Paragraph::new(tab_text)
        .block(Block::default().borders(Borders::ALL).title("JIRA Issues"))
        .wrap(Wrap { trim: true });
    f.render_widget(tabs, chunks[0]);

    // Show loading or error message
    if app.loading {
        let loading = Paragraph::new("Loading issues...")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(loading, chunks[1]);
        return;
    }

    if let Some(ref error) = app.error_message {
        let error_widget = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title("Error"))
            .wrap(Wrap { trim: true });
        f.render_widget(error_widget, chunks[1]);
        return;
    }

    match app.current_tab {
        Tab::Assigned => render_kanban_board(f, app, chunks[1]),
        Tab::Recent => render_recent_issues(f, app, chunks[1]),
    }

    // Show issue details if selected
    if app.show_details {
        if let Some(ref issue) = app.selected_issue {
            let popup_area = centered_rect(80, 80, f.size());
            f.render_widget(Clear, popup_area);

            let description = issue.description.as_deref().unwrap_or("No description available");
            let issue_url = issue.get_url(&app.jira_base_url);
            let watch_status = match issue.is_watching {
                Some(true) => "Watching ✓",
                Some(false) => "Not watching",
                None => "Loading watch status...",
            };
            
            // Build updates section - only show updates from last 7 days
            let updates_text = if let Some(ref changelog) = issue.changelog {
                let recent_changes: Vec<_> = changelog
                    .iter()
                    .filter(|entry| is_within_last_week(&entry.created))
                    .take(5)
                    .collect();
                
                if recent_changes.is_empty() {
                    "No recent changes (last 7 days)".to_string()
                } else {
                    let mut updates = Vec::new();
                    for entry in recent_changes {
                        let mut changes = Vec::new();
                        for item in &entry.items {
                            let change = if let (Some(from), Some(to)) = (&item.from_string, &item.to_string) {
                                format!("{}: {} → {}", item.field, from, to)
                            } else if let Some(to) = &item.to_string {
                                format!("{}: {}", item.field, to)
                            } else {
                                format!("{}: updated", item.field)
                            };
                            changes.push(change);
                        }
                        updates.push(format!("{} by {} - {}", entry.created[..19].replace('T', " "), entry.author, changes.join(", ")));
                    }
                    updates.join("\n")
                }
            } else {
                "Loading updates...".to_string()
            };

            let comments_text = if let Some(ref comments) = issue.comments {
                let recent_comments: Vec<_> = comments
                    .iter()
                    .filter(|comment| is_within_last_week(&comment.created))
                    .take(3)
                    .collect();
                
                if recent_comments.is_empty() {
                    "No recent comments (last 7 days)".to_string()
                } else {
                    let mut comment_list = Vec::new();
                    for comment in recent_comments {
                        comment_list.push(format!("{} by {} - {}", 
                            comment.created[..19].replace('T', " "), 
                            comment.author, 
                            comment.body.chars().take(100).collect::<String>()
                        ));
                    }
                    comment_list.join("\n")
                }
            } else {
                "Loading comments...".to_string()
            };

            let details_text = format!(
                "Key: {}\n\nSummary: {}\n\nURL: {}\n\nStatus: {}\nPriority: {}\nAssignee: {}\nReporter: {}\nWatching: {}\nCreated: {}\nUpdated: {}\n\nDescription:\n{}\n\nRecent Updates (Last 7 Days):\n{}\n\nRecent Comments (Last 7 Days):\n{}",
                issue.key,
                issue.summary,
                issue_url,
                issue.status,
                issue.priority,
                issue.assignee.as_deref().unwrap_or("Unassigned"),
                issue.reporter,
                watch_status,
                issue.created,
                issue.updated,
                description,
                updates_text,
                comments_text
            );

            let details = Paragraph::new(details_text)
                .block(Block::default().borders(Borders::ALL).title("Issue Details"))
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White).bg(Color::Black));

            f.render_widget(details, popup_area);
        }
    }

    // Show help
    let help_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.size());

    let help_text = if app.show_details {
        "Press 'Esc' or 'q' to close details | 'w' to toggle watch | Copy the URL to open in your browser"
    } else {
        match app.current_tab {
            Tab::Assigned => "↑/k: Up | ↓/j: Down | ←/h: Left | →/l: Right | Enter/Space: Details | Tab: Switch tabs | r: Refresh | q: Quit",
            Tab::Recent => "↑/k: Up | ↓/j: Down | Enter/Space: Details | Tab: Switch tabs | r: Refresh | q: Quit",
        }
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, help_chunks[1]);
}

fn render_kanban_board(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Create 3 columns for kanban board
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)].as_ref())
        .split(area);

    // Get issues for each column
    let todo_issues: Vec<&JiraIssue> = app.assigned_issues
        .iter()
        .filter(|issue| matches!(
            issue.status.as_str(),
            "해야 할 일" | "To Do" | "Open" | "PENDING"
        ))
        .collect();

    let inprogress_issues: Vec<&JiraIssue> = app.assigned_issues
        .iter()
        .filter(|issue| matches!(
            issue.status.as_str(),
            "진행 중" | "In Progress" | "In Review"
        ))
        .collect();

    let done_issues: Vec<&JiraIssue> = app.assigned_issues
        .iter()
        .filter(|issue| matches!(
            issue.status.as_str(),
            "해결됨" | "완료" | "Done" | "Closed" | "Resolved"
        ))
        .collect();

    // Render each column
    render_status_column(f, "To Do", &todo_issues, columns[0], 
                        app.current_column == StatusColumn::Todo, &mut app.todo_list_state);
    render_status_column(f, "In Progress", &inprogress_issues, columns[1], 
                        app.current_column == StatusColumn::InProgress, &mut app.inprogress_list_state);
    render_status_column(f, "Done", &done_issues, columns[2], 
                        app.current_column == StatusColumn::Done, &mut app.done_list_state);
}

fn render_status_column(
    f: &mut Frame, 
    title: &str, 
    issues: &[&JiraIssue], 
    area: ratatui::layout::Rect, 
    is_selected: bool,
    list_state: &mut ListState
) {
    let items: Vec<ListItem> = issues
        .iter()
        .map(|issue| {
            let priority_color = match issue.priority.as_str() {
                "Highest" | "High" => Color::Red,
                "Medium" => Color::Yellow,
                "Low" | "Lowest" => Color::Green,
                _ => Color::White,
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(&issue.key, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw(&issue.summary),
                ]),
                Line::from(vec![
                    Span::styled("Priority: ", Style::default().fg(Color::Gray)),
                    Span::styled(&issue.priority, Style::default().fg(priority_color)),
                ]),
                Line::from(vec![
                    Span::styled("Assignee: ", Style::default().fg(Color::Gray)),
                    Span::raw(issue.assignee.as_deref().unwrap_or("Unassigned")),
                ]),
                Line::from(vec![
                    Span::styled("─".repeat(30), Style::default().fg(Color::DarkGray)),
                ]),
            ];
            ListItem::new(content)
        })
        .collect();

    let current_position = list_state.selected()
        .map(|i| i + 1)
        .unwrap_or(0);
    
    let total_count = issues.len();
    
    let column_title = if total_count > 0 && current_position > 0 && is_selected {
        format!("{} ({}) - {}/{}", title, total_count, current_position, total_count)
    } else {
        format!("{} ({})", title, total_count)
    };

    let block_style = if is_selected {
        Block::default()
            .borders(Borders::ALL)
            .title(column_title)
            .border_style(Style::default().fg(Color::Yellow))
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title(column_title)
    };

    let list = List::new(items)
        .block(block_style)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, list_state);
}

fn render_recent_issues(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app.recent_issues
        .iter()
        .map(|issue| {
            let priority_color = match issue.priority.as_str() {
                "Highest" | "High" => Color::Red,
                "Medium" => Color::Yellow,
                "Low" | "Lowest" => Color::Green,
                _ => Color::White,
            };

            let status_color = match issue.status.as_str() {
                "Done" | "Closed" | "Resolved" | "해결됨" | "완료" => Color::Green,
                "In Progress" | "In Review" | "진행 중" => Color::Yellow,
                "To Do" | "Open" | "해야 할 일" | "PENDING" => Color::Blue,
                _ => Color::White,
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(&issue.key, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(" - "),
                    Span::raw(&issue.summary),
                ]),
                Line::from(vec![
                    Span::styled("  Status: ", Style::default().fg(Color::Gray)),
                    Span::styled(&issue.status, Style::default().fg(status_color)),
                    Span::raw(" | "),
                    Span::styled("Priority: ", Style::default().fg(Color::Gray)),
                    Span::styled(&issue.priority, Style::default().fg(priority_color)),
                    Span::raw(" | "),
                    Span::styled("Assignee: ", Style::default().fg(Color::Gray)),
                    Span::raw(issue.assignee.as_deref().unwrap_or("Unassigned")),
                ]),
                Line::from(vec![
                    Span::styled("─".repeat(80), Style::default().fg(Color::DarkGray)),
                ]),
            ];
            ListItem::new(content)
        })
        .collect();

    let current_position = app.recent_list_state.selected()
        .map(|i| i + 1)
        .unwrap_or(0);
    
    let total_count = app.recent_issues.len();
    
    let title = if total_count > 0 && current_position > 0 {
        format!("Recent Issues ({}) - {}/{}", total_count, current_position, total_count)
    } else {
        format!("Recent Issues ({})", total_count)
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.recent_list_state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub async fn run_app_with_config(config: Config) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let jira_client = JiraClient::new(
        config.jira_base_url.clone(),
        config.jira_email.clone(),
        config.jira_api_token.clone(),
    );
    let mut app = App::new(jira_client, config.jira_base_url.clone());
    
    // Load initial data
    app.load_issues().await?;

    let res = run_app_loop(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}


async fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if app.handle_key_event(key.code) {
                    return Ok(());
                }
                
                // Handle refresh
                if key.code == KeyCode::Char('r') && !app.show_details {
                    app.load_issues().await?;
                }

                // Handle watch toggle
                if key.code == KeyCode::Char('w') && app.show_details {
                    if let Ok(_message) = app.toggle_watch().await {
                        // You could store this message to show to user
                        // For now, we'll just continue
                    }
                }

                // Load watch status and updates when issue details are shown
                if app.show_details && app.selected_issue.is_some() {
                    if let Some(ref issue) = app.selected_issue.clone() {
                        if issue.is_watching.is_none() {
                            let _ = app.load_watch_status(&issue.key).await;
                        }
                        if issue.changelog.is_none() {
                            let _ = app.load_issue_updates(&issue.key).await;
                        }
                    }
                }
            }
        }
    }
}