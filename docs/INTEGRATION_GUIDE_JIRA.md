# Integration Guide: Jira Data Source

## Overview

This guide shows how to integrate Jira as an external data source for OmniDatum. This is useful for tracking repositories mentioned in Jira issues, epics, or project metadata.

**Use Case:** Sync repository information from Jira custom fields, issue descriptions, or project metadata where teams track their technology stack and dependencies.

---

## Prerequisites

### Dependencies

Add to [`Cargo.toml`](../Cargo.toml):

```toml
[dependencies]
# ... existing dependencies ...

# Jira API
jira_query = "2.3"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
```

### Jira Setup

1. Create API token at https://id.atlassian.com/manage-profile/security/api-tokens
2. Get your Jira site URL (e.g., `https://yourcompany.atlassian.net`)
3. Identify custom field IDs or issue labels for repository tracking

---

## DataSourceAdapter Trait Reference

Located in [`src/sync/adapters/mod.rs`](../src/sync/adapters/mod.rs):

```rust
use crate::models::Repository;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for data source adapters
#[async_trait]
pub trait DataSourceAdapter: Send + Sync {
    /// Fetch repository data from external source
    ///
    /// # Arguments
    /// * `identifier` - Source-specific identifier
    ///   - For Jira: Issue key (e.g., "PROJ-123") or JQL query
    ///
    /// # Returns
    /// Repository data extracted from the source
    ///
    /// # Errors
    /// Returns error if:
    /// - Authentication fails
    /// - Issue not found
    /// - Required fields missing
    /// - Network errors
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository>;

    /// Check if connection to source is working
    ///
    /// # Returns
    /// Ok(()) if authenticated and can query, Err otherwise
    async fn check_connection(&self) -> Result<()>;

    /// Get source name for logging
    fn source_name(&self) -> &str {
        "unknown"
    }
}
```

---

## Implementation

### Step 1: Create Jira Adapter

Create [`src/sync/adapters/jira.rs`](../src/sync/adapters/jira.rs):

```rust
//! Jira data source adapter
//!
//! Fetches repository information from Jira issues where repositories
//! are tracked in custom fields, labels, or issue descriptions.

use super::DataSourceAdapter;
use crate::config::OmnidatumConfig;
use crate::models::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository,
    RepositoryClassification, RepositoryMetadata, RepositorySource,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};

/// Jira issue response
#[derive(Debug, Deserialize)]
struct JiraIssue {
    key: String,
    fields: JiraFields,
}

/// Jira issue fields
#[derive(Debug, Deserialize)]
struct JiraFields {
    summary: String,
    description: Option<String>,
    labels: Vec<String>,
    
    #[serde(flatten)]
    custom_fields: serde_json::Value,
}

/// Jira adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    /// Jira site URL (e.g., https://company.atlassian.net)
    pub site_url: String,
    /// API token for authentication
    pub api_token: String,
    /// User email for basic auth
    pub email: String,
    /// Custom field ID for repository URL (e.g., "customfield_10050")
    pub repo_url_field: String,
    /// Custom field ID for repository language (optional)
    pub language_field: Option<String>,
    /// Label prefix for technology tags (e.g., "tech:")
    pub tech_label_prefix: Option<String>,
}

/// Jira API adapter
pub struct JiraAdapter {
    client: Client,
    config: JiraConfig,
}

impl JiraAdapter {
    /// Create new Jira adapter
    ///
    /// # Arguments
    /// * `omnidatum_config` - Application configuration
    /// * `jira_config` - Jira-specific configuration
    ///
    /// # Returns
    /// Authenticated Jira adapter ready to fetch data
    pub async fn new(
        omnidatum_config: &OmnidatumConfig,
        jira_config: JiraConfig,
    ) -> Result<Self> {
        // Create HTTP client with auth
        let mut headers = header::HeaderMap::new();
        
        // Basic auth: base64(email:api_token)
        let auth_string = format!("{}:{}", jira_config.email, jira_config.api_token);
        let auth_header = format!("Basic {}", base64::encode(auth_string));
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&auth_header)?,
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(
                omnidatum_config.sync.request_timeout_secs,
            ))
            .build()?;

        log::info!("Jira adapter initialized for site: {}", jira_config.site_url);

        Ok(Self {
            client,
            config: jira_config,
        })
    }

    /// Fetch issue by key
    async fn fetch_issue(&self, issue_key: &str) -> Result<JiraIssue> {
        let url = format!(
            "{}/rest/api/3/issue/{}",
            self.config.site_url, issue_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch Jira issue")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Jira API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let issue: JiraIssue = response
            .json()
            .await
            .context("Failed to parse Jira response")?;

        Ok(issue)
    }

    /// Search issues with JQL
    async fn search_issues(&self, jql: &str, max_results: u32) -> Result<Vec<JiraIssue>> {
        let url = format!("{}/rest/api/3/search", self.config.site_url);

        #[derive(Serialize)]
        struct SearchRequest {
            jql: String,
            #[serde(rename = "maxResults")]
            max_results: u32,
            fields: Vec<String>,
        }

        let request = SearchRequest {
            jql: jql.to_string(),
            max_results,
            fields: vec![
                "summary".to_string(),
                "description".to_string(),
                "labels".to_string(),
                self.config.repo_url_field.clone(),
            ],
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to search Jira issues")?;

        #[derive(Deserialize)]
        struct SearchResponse {
            issues: Vec<JiraIssue>,
        }

        let result: SearchResponse = response
            .json()
            .await
            .context("Failed to parse Jira search response")?;

        Ok(result.issues)
    }

    /// Convert Jira issue to Repository
    fn issue_to_repository(&self, issue: JiraIssue) -> Result<Repository> {
        // Extract repository URL from custom field
        let repo_url = issue
            .fields
            .custom_fields
            .get(&self.config.repo_url_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Issue {} missing repository URL field {}",
                    issue.key,
                    self.config.repo_url_field
                )
            })?;

        // Parse URL
        let (platform, owner, name) = self.parse_repository_url(repo_url)?;
        let full_name = format!("{}/{}", owner, name);

        // Extract language from custom field if configured
        let language = if let Some(lang_field) = &self.config.language_field {
            issue
                .fields
                .custom_fields
                .get(lang_field)
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string()
        } else {
            "Unknown".to_string()
        };

        // Extract technology tags from labels
        let tech_tags: Vec<String> = if let Some(prefix) = &self.config.tech_label_prefix {
            issue
                .fields
                .labels
                .iter()
                .filter(|l| l.starts_with(prefix))
                .map(|l| l.strip_prefix(prefix).unwrap_or(l).to_string())
                .collect()
        } else {
            issue.fields.labels.clone()
        };

        let id = format!("jira-{}-{}", issue.key, full_name.replace('/', "-"));

        Ok(Repository {
            id,
            platforms: vec![PlatformInfo {
                platform,
                url: repo_url.to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                notes: Some(format!("Tracked in Jira issue: {}", issue.key)),
            }],
            metadata: RepositoryMetadata {
                name: name.to_string(),
                owner: owner.to_string(),
                full_name: full_name.clone(),
                description: issue.fields.description.unwrap_or(issue.fields.summary),
                primary_language: language.clone(),
                license: None, // Not available from Jira
                license_spdx: None,
                stars: 0, // Not available from Jira
                topics: tech_tags,
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec!["jira-tracked".to_string()],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: language,
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: Some(format!("Tracked in Jira: {}", issue.key)),
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: None,
                last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                quality_score: 70,
            },
            source: RepositorySource::Manual, // Curated via Jira
            added_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            manually_curated: true,
            curator_notes: Some(format!("From Jira issue: {}", issue.key)),
        })
    }

    /// Parse repository URL to extract platform and identifiers
    fn parse_repository_url(&self, url: &str) -> Result<(Platform, String, String)> {
        if url.contains("github.com") {
            let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[parts.len() - 2];
                let name = parts[parts.len() - 1];
                return Ok((Platform::GitHub, owner.to_string(), name.to_string()));
            }
        } else if url.contains("gitlab.com") {
            let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[parts.len() - 2];
                let name = parts[parts.len() - 1];
                return Ok((Platform::GitLab, owner.to_string(), name.to_string()));
            }
        }

        Err(anyhow::anyhow!("Unsupported repository URL: {}", url))
    }
}

#[async_trait]
impl DataSourceAdapter for JiraAdapter {
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository> {
        log::debug!("Fetching repository from Jira issue: {}", identifier);

        let issue = self.fetch_issue(identifier).await?;
        self.issue_to_repository(issue)
    }

    async fn check_connection(&self) -> Result<()> {
        // Try to fetch current user to verify auth
        let url = format!("{}/rest/api/3/myself", self.config.site_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to Jira")?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Jira authentication failed: {}",
                response.status()
            ))
        }
    }

    fn source_name(&self) -> &str {
        "jira"
    }
}

/// Extended functionality for bulk operations
impl JiraAdapter {
    /// Fetch all repositories from Jira project
    ///
    /// # Arguments
    /// * `project_key` - Jira project key (e.g., "INFRA")
    /// * `label` - Label to filter issues (e.g., "repository")
    ///
    /// # Returns
    /// Vector of Repository objects from matching issues
    pub async fn fetch_from_project(
        &self,
        project_key: &str,
        label: Option<&str>,
    ) -> Result<Vec<Repository>> {
        // Build JQL query
        let jql = if let Some(lbl) = label {
            format!(
                "project = {} AND labels = {} AND {} IS NOT EMPTY",
                project_key, lbl, self.config.repo_url_field
            )
        } else {
            format!(
                "project = {} AND {} IS NOT EMPTY",
                project_key, self.config.repo_url_field
            )
        };

        log::info!("Searching Jira with JQL: {}", jql);

        // Search issues
        let issues = self.search_issues(&jql, 1000).await?;

        log::info!("Found {} Jira issues with repositories", issues.len());

        // Convert to repositories
        let mut repositories = Vec::new();
        for issue in issues {
            match self.issue_to_repository(issue) {
                Ok(repo) => repositories.push(repo),
                Err(e) => {
                    log::warn!("Failed to convert Jira issue to repository: {}", e);
                    continue;
                }
            }
        }

        Ok(repositories)
    }

    /// Fetch repositories from epic
    ///
    /// # Arguments
    /// * `epic_key` - Epic issue key (e.g., "PROJ-1")
    ///
    /// # Returns
    /// All repositories mentioned in epic and its child issues
    pub async fn fetch_from_epic(&self, epic_key: &str) -> Result<Vec<Repository>> {
        let jql = format!(
            "\"Epic Link\" = {} AND {} IS NOT EMPTY",
            epic_key, self.config.repo_url_field
        );

        let issues = self.search_issues(&jql, 1000).await?;

        let mut repositories = Vec::new();
        for issue in issues {
            if let Ok(repo) = self.issue_to_repository(issue) {
                repositories.push(repo);
            }
        }

        Ok(repositories)
    }
}
```

### Step 2: Add Jira Configuration

Update [`src/config/settings.rs`](../src/config/settings.rs):

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncConfig {
    // ... existing fields ...
    
    /// Jira integration settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jira: Option<JiraConfig>,
}

/// Jira-specific configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JiraConfig {
    pub enabled: bool,
    pub site_url: String,
    pub project_key: String,
    pub repository_label: Option<String>,
    pub repo_url_field: String,
    pub language_field: Option<String>,
    pub tech_label_prefix: Option<String>,
}
```

### Step 3: Configure Credentials

Update [`src/config/credentials.rs`](../src/config/credentials.rs):

```rust
impl CredentialManager {
    /// Get Jira API token
    pub fn get_jira_token(&self) -> Result<String> {
        match self.source {
            CredentialSource::Env => std::env::var("JIRA_API_TOKEN")
                .context("JIRA_API_TOKEN not found in environment"),
            CredentialSource::File => {
                let path = Self::jira_credentials_file_path();
                let content = std::fs::read_to_string(&path)
                    .context("Failed to read Jira credentials")?;
                Ok(content.trim().to_string())
            }
            CredentialSource::Keychain => self.from_keychain_jira(),
        }
    }

    /// Get Jira credentials file path
    fn jira_credentials_file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("omnidatum")
            .join("jira-credentials")
    }

    /// Get Jira email for basic auth
    pub fn get_jira_email(&self) -> Result<String> {
        std::env::var("JIRA_EMAIL")
            .context("JIRA_EMAIL not found in environment")
    }

    #[cfg(target_os = "macos")]
    fn from_keychain_jira(&self) -> Result<String> {
        let output = std::process::Command::new("security")
            .args(&["find-generic-password", "-a", "omnidatum", "-s", "jira-token", "-w"])
            .output()?;
        
        if output.status.success() {
            let token = String::from_utf8(output.stdout)?;
            return Ok(token.trim().to_string());
        }
        
        Err(anyhow::anyhow!("Jira token not found in keychain"))
    }

    #[cfg(not(target_os = "macos"))]
    fn from_keychain_jira(&self) -> Result<String> {
        Err(anyhow::anyhow!("Keychain access not implemented for this platform"))
    }
}
```

### Step 4: Register Adapter

Update [`src/sync/adapters/mod.rs`](../src/sync/adapters/mod.rs):

```rust
pub mod jira;
pub use jira::{JiraAdapter, JiraConfig};
```

### Step 5: Update SyncOrchestrator

Add to [`src/sync/mod.rs`](../src/sync/mod.rs):

```rust
impl SyncOrchestrator {
    /// Sync from Jira if configured
    pub async fn sync_from_jira(&mut self) -> Result<Vec<Repository>> {
        if let Some(jira_config) = &self.config.sync.jira {
            if jira_config.enabled {
                log::info!("Syncing from Jira project: {}", jira_config.project_key);
                
                // Get Jira credentials
                let cred_mgr = crate::config::CredentialManager::new(
                    self.config.credentials.source.clone()
                );
                let api_token = cred_mgr.get_jira_token()?;
                let email = cred_mgr.get_jira_email()?;
                
                // Build adapter config
                let adapter_config = crate::sync::adapters::JiraConfig {
                    enabled: true,
                    site_url: jira_config.site_url.clone(),
                    project_key: jira_config.project_key.clone(),
                    repository_label: jira_config.repository_label.clone(),
                    repo_url_field: jira_config.repo_url_field.clone(),
                    language_field: jira_config.language_field.clone(),
                    tech_label_prefix: jira_config.tech_label_prefix.clone(),
                };
                
                let mut full_config = adapter_config.clone();
                full_config.api_token = api_token;
                full_config.email = email;
                
                let adapter = JiraAdapter::new(&self.config, full_config).await?;
                
                // Check connection
                adapter.check_connection().await?;
                
                // Fetch from project
                let repos = adapter
                    .fetch_from_project(
                        &jira_config.project_key,
                        jira_config.repository_label.as_deref(),
                    )
                    .await?;
                
                log::info!("Fetched {} repos from Jira", repos.len());
                return Ok(repos);
            }
        }
        
        Ok(vec![])
    }
}
```

---

## Configuration

### Application Config

`~/.config/omnidatum/config.toml`:

```toml
[sync.jira]
enabled = true
site_url = "https://yourcompany.atlassian.net"
project_key = "INFRA"
repository_label = "repository"  # Issues with this label are synced
repo_url_field = "customfield_10050"  # Custom field containing repo URL
language_field = "customfield_10051"  # Optional: language field
tech_label_prefix = "tech:"  # Labels like "tech:rust", "tech:go"
```

### Environment Variables

```bash
# Jira credentials
export JIRA_API_TOKEN=your_api_token_here
export JIRA_EMAIL=your.email@company.com

# Or store in file
echo "your_api_token" > ~/.config/omnidatum/jira-credentials
chmod 600 ~/.config/omnidatum/jira-credentials
```

---

## Jira Issue Format

### Custom Fields Setup

Create custom fields in Jira:

1. **Repository URL** (Text field, single line)
   - Field ID: `customfield_10050`
   - Used to store full repository URL

2. **Language** (Select list, single choice)
   - Field ID: `customfield_10051`
   - Options: Rust, Go, Python, TypeScript, etc.

### Issue Labels

Use labels to categorize repositories:
- `repository` - Marks issue as tracking a repository
- `tech:rust` - Technology/language tags
- `priority:high` - Importance indicators

### Example Jira Issue

```
Issue: INFRA-123
Summary: rust-lang/rust - Rust Programming Language
Description: The Rust programming language compiler and standard library

Labels: repository, tech:rust, tech:compiler
Custom Fields:
  Repository URL: https://github.com/rust-lang/rust
  Language: Rust
```

---

## Usage

### CLI Integration

```bash
# Sync from Jira project
cargo run -- sync --from-jira

# Sync specific epic
cargo run -- sync --from-jira --epic PROJ-100

# Hybrid: Jira + GitHub
cargo run -- sync --from-jira  # Get list from Jira
cargo run -- sync              # Enrich with GitHub data
```

### CLI Command Updates

Update sync command in [`src/main.rs`](../src/main.rs):

```rust
Sync {
    // ... existing fields ...
    
    /// Sync from Jira instead of GitHub
    #[arg(long)]
    from_jira: bool,
    
    /// Jira epic key to sync (requires --from-jira)
    #[arg(long)]
    epic: Option<String>,
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url() {
        let config = create_test_config();
        let adapter = create_test_adapter(config);
        
        let result = adapter.parse_repository_url("https://github.com/rust-lang/rust");
        assert!(result.is_ok());
        
        let (platform, owner, name) = result.unwrap();
        assert_eq!(platform, Platform::GitHub);
        assert_eq!(owner, "rust-lang");
        assert_eq!(name, "rust");
    }

    #[test]
    fn test_issue_to_repository() {
        let adapter = create_test_adapter(create_test_config());
        let issue = create_test_issue();
        
        let result = adapter.issue_to_repository(issue);
        assert!(result.is_ok());
        
        let repo = result.unwrap();
        assert!(repo.id.starts_with("jira-"));
        assert!(repo.manually_curated);
    }

    fn create_test_config() -> JiraConfig {
        JiraConfig {
            enabled: true,
            site_url: "https://test.atlassian.net".to_string(),
            project_key: "TEST".to_string(),
            repository_label: Some("repository".to_string()),
            repo_url_field: "customfield_10050".to_string(),
            language_field: Some("customfield_10051".to_string()),
            tech_label_prefix: Some("tech:".to_string()),
        }
    }

    fn create_test_issue() -> JiraIssue {
        // Create mock issue for testing
        JiraIssue {
            key: "TEST-123".to_string(),
            fields: JiraFields {
                summary: "Test Repo".to_string(),
                description: Some("Test description".to_string()),
                labels: vec!["repository".to_string(), "tech:rust".to_string()],
                custom_fields: serde_json::json!({
                    "customfield_10050": "https://github.com/test/repo",
                    "customfield_10051": "Rust"
                }),
            },
        }
    }
}
```

---

## JQL Query Examples

### Find All Repository Issues

```jql
project = INFRA 
AND labels = repository 
AND customfield_10050 IS NOT EMPTY
ORDER BY created DESC
```

### Find Rust Repositories

```jql
project = INFRA 
AND labels IN (repository, tech:rust)
AND customfield_10050 IS NOT EMPTY
```

### Find High Priority Dependencies

```jql
project = INFRA 
AND labels IN (repository, priority:high)
AND customfield_10050 IS NOT EMPTY
ORDER BY priority DESC
```

---

## Error Handling

### Authentication Errors

```
Error: Jira authentication failed: 401

Solution:
1. Verify JIRA_API_TOKEN is correct
2. Verify JIRA_EMAIL matches token owner
3. Check token hasn't expired
4. Regenerate token if needed
```

### Custom Field Errors

```
Error: Issue PROJ-123 missing repository URL field customfield_10050

Solution:
1. Verify custom field ID is correct
2. Check issue has the custom field populated
3. Verify field is included in search results
```

### Rate Limiting

```
Error: Jira API rate limit exceeded

Solution:
1. Jira Cloud has rate limits (varies by plan)
2. Add delays between requests
3. Cache results to reduce API calls
4. Contact Jira admin to check limits
```

---

## Advanced Usage

### Bi-Directional Sync

Sync from Jira, enrich with GitHub, update Jira with stars:

```rust
// 1. Fetch from Jira
let jira_repos = orchestrator.sync_from_jira().await?;

// 2. Add to canonical
for repo in jira_repos {
    canonical_data.repositories.push(repo);
}

// 3. Enrich with GitHub
orchestrator.sync_all(&canonical_path).await?;

// 4. Update Jira with star counts (optional)
// update_jira_custom_field(issue_key, stars_field, value)
```

### Webhook Integration

Listen for Jira webhooks to trigger sync on issue updates:

```rust
// In future webhook server implementation
#[post("/jira-webhook")]
async fn handle_jira_webhook(payload: JiraWebhookPayload) -> Result<()> {
    if payload.issue_event_type_name == "issue_updated" {
        // Trigger selective sync
        orchestrator.sync_from_jira().await?;
    }
    Ok(())
}
```

---

## API Reference

### Jira REST API v3

**Documentation:** https://developer.atlassian.com/cloud/jira/platform/rest/v3/

**Key Endpoints:**
- `GET /rest/api/3/issue/{issueKey}` - Get single issue
- `POST /rest/api/3/search` - Search with JQL
- `GET /rest/api/3/myself` - Verify authentication

**Authentication:**
- Basic Auth: `Authorization: Basic base64(email:api_token)`
- API tokens: https://id.atlassian.com/manage-profile/security/api-tokens

**Rate Limits:**
- Varies by Jira Cloud plan
- Typically: 10-100 requests per second
- Monitor `X-RateLimit-*` response headers

---

## Complete Implementation Checklist

- [ ] Add dependencies to Cargo.toml
- [ ] Create src/sync/adapters/jira.rs with full implementation
- [ ] Update src/sync/adapters/mod.rs to export JiraAdapter
- [ ] Add JiraConfig to src/config/settings.rs
- [ ] Update CredentialManager for Jira tokens
- [ ] Add Jira sync to SyncOrchestrator
- [ ] Update CLI with --from-jira flag
- [ ] Configure Jira custom fields
- [ ] Set up API token and environment variables
- [ ] Test connection with check_connection()
- [ ] Run sync: cargo run -- sync --from-jira
- [ ] Verify results with: cargo run -- stats

---

This guide provides complete implementation for Jira integration with full API documentation and examples.