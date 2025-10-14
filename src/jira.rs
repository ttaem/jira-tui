use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::{Engine, engine::general_purpose};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub id: String,
    pub key: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub reporter: String,
    pub created: String,
    pub updated: String,
    pub is_watching: Option<bool>,
    pub changelog: Option<Vec<ChangelogEntry>>,
    pub comments: Option<Vec<Comment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub id: String,
    pub author: String,
    pub created: String,
    pub items: Vec<ChangelogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogItem {
    pub field: String,
    pub field_type: String,
    pub from_string: Option<String>,
    pub to_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub created: String,
    pub updated: String,
    pub body: String,
}

impl JiraIssue {
    pub fn get_url(&self, base_url: &str) -> String {
        format!("{}/browse/{}", base_url, self.key)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct JiraApiResponse {
    issues: Vec<JiraApiIssue>,
    #[serde(rename = "isLast")]
    is_last: bool,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JiraApiIssue {
    id: String,
    key: String,
    fields: JiraFields,
}

#[derive(Debug, Serialize, Deserialize)]
struct JiraFields {
    summary: String,
    description: Option<JiraDescription>,
    status: JiraStatus,
    priority: JiraPriority,
    assignee: Option<JiraUser>,
    reporter: JiraUser,
    created: String,
    updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JiraDescription {
    content: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JiraStatus {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JiraPriority {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JiraUser {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct WatchersResponse {
    #[serde(rename = "isWatching")]
    is_watching: bool,
    #[serde(rename = "watchCount")]
    watch_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct IssueWithChangelog {
    id: String,
    key: String,
    fields: JiraFields,
    changelog: Option<Changelog>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Changelog {
    histories: Vec<ChangelogHistory>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangelogHistory {
    id: String,
    author: JiraUser,
    created: String,
    items: Vec<ChangelogItemApi>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangelogItemApi {
    field: String,
    #[serde(rename = "fieldtype")]
    field_type: String,
    #[serde(rename = "fromString")]
    from_string: Option<String>,
    #[serde(rename = "toString")]
    to_string: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommentsResponse {
    comments: Vec<CommentApi>,
    total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommentApi {
    id: String,
    author: JiraUser,
    created: String,
    updated: String,
    body: serde_json::Value,
}

pub struct JiraClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl JiraClient {
    pub fn new(base_url: String, email: String, api_token: String) -> Self {
        let auth = general_purpose::STANDARD.encode(format!("{}:{}", email, api_token));
        let auth_header = format!("Basic {}", auth);
        
        Self {
            client: Client::new(),
            base_url,
            auth_header,
        }
    }

    pub async fn get_assigned_issues(&self) -> Result<Vec<JiraIssue>> {
        let url = format!("{}/rest/api/3/search/jql", self.base_url);
        
        let jql = "assignee = currentUser() AND resolution = Unresolved ORDER BY updated DESC";
        let mut params = HashMap::new();
        params.insert("jql", jql);
        params.insert("maxResults", "50");
        params.insert("fields", "summary,description,status,priority,assignee,reporter,created,updated");

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("API request failed: {}", response.status()));
        }

        let api_response: JiraApiResponse = response.json().await?;
        
        let issues = api_response
            .issues
            .into_iter()
            .map(|issue| {
                let description = issue.fields.description
                    .map(|desc| {
                        // Simplified description extraction
                        desc.content
                            .into_iter()
                            .filter_map(|content| {
                                content.get("content")
                                    .and_then(|c| c.as_array())
                                    .map(|paragraphs| {
                                        paragraphs
                                            .iter()
                                            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                                            .collect::<Vec<_>>()
                                            .join(" ")
                                    })
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .filter(|s| !s.is_empty());

                JiraIssue {
                    id: issue.id,
                    key: issue.key,
                    summary: issue.fields.summary,
                    description,
                    status: issue.fields.status.name,
                    priority: issue.fields.priority.name,
                    assignee: issue.fields.assignee.map(|u| u.display_name),
                    reporter: issue.fields.reporter.display_name,
                    created: issue.fields.created,
                    updated: issue.fields.updated,
                    is_watching: None, // Will be loaded separately if needed
                    changelog: None,   // Will be loaded separately if needed
                    comments: None,    // Will be loaded separately if needed
                }
            })
            .collect();

        Ok(issues)
    }

    pub async fn get_recent_issues(&self) -> Result<Vec<JiraIssue>> {
        let url = format!("{}/rest/api/3/search/jql", self.base_url);
        
        let jql = "updated >= -7d ORDER BY updated DESC";
        let mut params = HashMap::new();
        params.insert("jql", jql);
        params.insert("maxResults", "50");
        params.insert("fields", "summary,description,status,priority,assignee,reporter,created,updated");

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .query(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("API request failed: {}", response.status()));
        }

        let api_response: JiraApiResponse = response.json().await?;
        
        let issues = api_response
            .issues
            .into_iter()
            .map(|issue| {
                let description = issue.fields.description
                    .map(|desc| {
                        desc.content
                            .into_iter()
                            .filter_map(|content| {
                                content.get("content")
                                    .and_then(|c| c.as_array())
                                    .map(|paragraphs| {
                                        paragraphs
                                            .iter()
                                            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                                            .collect::<Vec<_>>()
                                            .join(" ")
                                    })
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .filter(|s| !s.is_empty());

                JiraIssue {
                    id: issue.id,
                    key: issue.key,
                    summary: issue.fields.summary,
                    description,
                    status: issue.fields.status.name,
                    priority: issue.fields.priority.name,
                    assignee: issue.fields.assignee.map(|u| u.display_name),
                    reporter: issue.fields.reporter.display_name,
                    created: issue.fields.created,
                    updated: issue.fields.updated,
                    is_watching: None, // Will be loaded separately if needed
                    changelog: None,   // Will be loaded separately if needed
                    comments: None,    // Will be loaded separately if needed
                }
            })
            .collect();

        Ok(issues)
    }

    pub async fn get_watch_status(&self, issue_key: &str) -> Result<bool> {
        let url = format!("{}/rest/api/3/issue/{}/watchers", self.base_url, issue_key);

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to get watch status: {}", response.status()));
        }

        let watchers_response: WatchersResponse = response.json().await?;
        Ok(watchers_response.is_watching)
    }

    pub async fn watch_issue(&self, issue_key: &str) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}/watchers", self.base_url, issue_key);

        let response = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to watch issue: {}", response.status()));
        }

        Ok(())
    }

    pub async fn unwatch_issue(&self, issue_key: &str) -> Result<()> {
        let url = format!("{}/rest/api/3/issue/{}/watchers", self.base_url, issue_key);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .query(&[("username", "")])  // Delete self as watcher
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to unwatch issue: {}", response.status()));
        }

        Ok(())
    }

    pub async fn get_issue_updates(&self, issue_key: &str) -> Result<(Vec<ChangelogEntry>, Vec<Comment>)> {
        // Get changelog
        let changelog_url = format!("{}/rest/api/3/issue/{}?expand=changelog", self.base_url, issue_key);
        
        let changelog_response = self
            .client
            .get(&changelog_url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await?;

        let changelog_entries = if changelog_response.status().is_success() {
            let issue_data: IssueWithChangelog = changelog_response.json().await?;
            if let Some(changelog) = issue_data.changelog {
                changelog.histories
                    .into_iter()
                    .map(|history| ChangelogEntry {
                        id: history.id,
                        author: history.author.display_name,
                        created: history.created,
                        items: history.items.into_iter().map(|item| ChangelogItem {
                            field: item.field,
                            field_type: item.field_type,
                            from_string: item.from_string,
                            to_string: item.to_string,
                        }).collect(),
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Get comments
        let comments_url = format!("{}/rest/api/3/issue/{}/comment", self.base_url, issue_key);
        
        let comments_response = self
            .client
            .get(&comments_url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await?;

        let comments = if comments_response.status().is_success() {
            let comments_data: CommentsResponse = comments_response.json().await?;
            comments_data.comments
                .into_iter()
                .map(|comment| Comment {
                    id: comment.id,
                    author: comment.author.display_name,
                    created: comment.created,
                    updated: comment.updated,
                    body: Self::extract_text_from_adf(&comment.body),
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok((changelog_entries, comments))
    }

    fn extract_text_from_adf(adf_content: &serde_json::Value) -> String {
        // Simple text extraction from Atlassian Document Format
        if let Some(content) = adf_content.get("content").and_then(|c| c.as_array()) {
            content
                .iter()
                .filter_map(|node| {
                    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
                        Some(
                            content
                                .iter()
                                .filter_map(|text_node| text_node.get("text").and_then(|t| t.as_str()))
                                .collect::<Vec<_>>()
                                .join("")
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "Unable to parse comment content".to_string()
        }
    }
}