# Data Source Integration Guides

## Overview

OmniDatum uses an extensible **DataSourceAdapter** pattern that makes it easy to integrate new external data sources. This document indexes all available integration guides and provides a quick-start template.

---

## Available Integration Guides

### 1. Google Sheets
**Guide:** [`INTEGRATION_GUIDE_GOOGLE_SHEETS.md`](INTEGRATION_GUIDE_GOOGLE_SHEETS.md)

**Use Case:** Maintain curated repository lists in spreadsheets
- **Complexity:** Medium (Google API setup required)
- **Authentication:** Service Account or OAuth2
- **Rate Limits:** 100 reads/100 seconds
- **Dependencies:** `google-sheets4`, `yup-oauth2`

**Quick Start:**
```bash
cargo add google-sheets4 yup-oauth2
# Configure spreadsheet ID
cargo run -- sync --from-sheets
```

### 2. Jira
**Guide:** [`INTEGRATION_GUIDE_JIRA.md`](INTEGRATION_GUIDE_JIRA.md)

**Use Case:** Track repositories mentioned in Jira issues/epics
- **Complexity:** Medium (Jira custom fields setup)
- **Authentication:** API Token + Email (Basic Auth)
- **Rate Limits:** Varies by plan (10-100 req/s)
- **Dependencies:** `reqwest`

**Quick Start:**
```bash
cargo add jira_query reqwest
# Configure Jira site URL and project
cargo run -- sync --from-jira
```

### 3. Local Git Scanner
**Guide:** [`INTEGRATION_GUIDE_GIT_REPOSITORY.md`](INTEGRATION_GUIDE_GIT_REPOSITORY.md)

**Use Case:** Discover and index Git repositories on filesystem
- **Complexity:** Low (no external API)
- **Authentication:** None (local filesystem)
- **Rate Limits:** None (disk I/O only)
- **Dependencies:** `git2`, `walkdir`

**Quick Start:**
```bash
cargo add git2 walkdir
# Configure scan root directory
cargo run -- sync --scan-local --scan-root ~/projects
```

---

## DataSourceAdapter Trait

All integrations implement this trait from [`src/sync/adapters/mod.rs`](../src/sync/adapters/mod.rs):

```rust
use crate::models::Repository;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for data source adapters
///
/// Implement this trait to add a new external data source to OmniDatum.
/// The trait uses async methods to support I/O-bound operations.
#[async_trait]
pub trait DataSourceAdapter: Send + Sync {
    /// Fetch repository data from external source
    ///
    /// # Arguments
    /// * `identifier` - Source-specific identifier
    ///   - GitHub: "owner/name"
    ///   - Sheets: row index or range
    ///   - Jira: issue key or JQL
    ///   - Git: local filesystem path
    ///
    /// # Returns
    /// Repository object with all available metadata
    ///
    /// # Errors
    /// - Authentication failures
    /// - Network errors  
    /// - Parse errors
    /// - Not found errors
    ///
    /// # Example
    /// ```rust
    /// let repo = adapter.fetch_repository("rust-lang/rust").await?;
    /// println!("Found: {}", repo.metadata.full_name);
    /// ```
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository>;

    /// Check if connection to source is working
    ///
    /// Used for health checks and initialization validation.
    /// Should be a lightweight operation (single API call).
    ///
    /// # Returns
    /// Ok(()) if source is accessible and authenticated
    /// Err with details if connection fails
    ///
    /// # Example
    /// ```rust
    /// adapter.check_connection().await?;
    /// println!("Connection OK");
    /// ```
    async fn check_connection(&self) -> Result<()>;

    /// Get source name for logging
    ///
    /// # Returns
    /// Human-readable source identifier (lowercase, underscored)
    ///
    /// # Examples
    /// - "github"
    /// - "google_sheets"
    /// - "jira"
    /// - "git_local"
    fn source_name(&self) -> &str {
        "unknown"
    }
}
```

---

## Integration Template

### Quick Start Template

Use this template to create a new adapter:

```rust
//! [SOURCE_NAME] data source adapter

use super::DataSourceAdapter;
use crate::config::OmnidatumConfig;
use crate::models::{Platform, PlatformInfo, PlatformStatus, Repository, /* ... */};
use anyhow::Result;
use async_trait::async_trait;

/// [SOURCE_NAME] adapter configuration
#[derive(Debug, Clone)]
pub struct MySourceConfig {
    pub api_url: String,
    pub api_key: String,
    // Add source-specific config
}

/// [SOURCE_NAME] data source adapter
pub struct MySourceAdapter {
    client: reqwest::Client,  // Or appropriate client
    config: MySourceConfig,
}

impl MySourceAdapter {
    /// Create new adapter
    pub async fn new(omnidatum_config: &OmnidatumConfig, source_config: MySourceConfig) -> Result<Self> {
        // 1. Load credentials
        // 2. Create HTTP client or connection
        // 3. Verify authentication
        // 4. Return configured adapter
        
        todo!("Implement initialization")
    }

    /// Convert source-specific data to Repository model
    fn convert_to_repository(&self, source_data: SourceData) -> Result<Repository> {
        // 1. Extract required fields (URL, description)
        // 2. Determine platform (GitHub, GitLab, etc.)
        // 3. Parse owner/name from URL
        // 4. Build Repository struct
        // 5. Set source-specific curator notes
        
        todo!("Implement conversion")
    }
}

#[async_trait]
impl DataSourceAdapter for MySourceAdapter {
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository> {
        // 1. Parse identifier
        // 2. Make API call or read data
        // 3. Convert to Repository model
        // 4. Return result
        
        todo!("Implement fetch")
    }

    async fn check_connection(&self) -> Result<()> {
        // Simple health check
        // Should be fast (<1s)
        
        todo!("Implement health check")
    }

    fn source_name(&self) -> &str {
        "my_source"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection() {
        // Test connection logic
    }

    #[test]
    fn test_conversion() {
        // Test data conversion
    }
}
```

---

## Common Integration Patterns

### Pattern 1: API-Based Source (GitHub, Jira, Sheets)

**Steps:**
1. Add HTTP client dependency (`reqwest`, `octocrab`, etc.)
2. Implement authentication (Bearer, Basic, OAuth2)
3. Create API client wrapper
4. Map API response to Repository model
5. Handle rate limits
6. Cache responses

**Example:**
- GitHub adapter (see [`src/sync/adapters/github.rs`](../src/sync/adapters/github.rs))
- Jira adapter (see [`INTEGRATION_GUIDE_JIRA.md`](INTEGRATION_GUIDE_JIRA.md))

### Pattern 2: File-Based Source (CSV, JSON, Local Git)

**Steps:**
1. Add file parsing dependency (`csv`, `walkdir`, `git2`)
2. Read files from filesystem
3. Parse into structured data
4. Map to Repository model
5. Handle missing or malformed data

**Example:**
- Git local scanner (see [`INTEGRATION_GUIDE_GIT_REPOSITORY.md`](INTEGRATION_GUIDE_GIT_REPOSITORY.md))

### Pattern 3: Database Source (PostgreSQL, MongoDB)

**Steps:**
1. Add database driver (`sqlx`, `mongodb`, `diesel`)
2. Establish connection with credentials
3. Query repository table/collection
4. Map results to Repository model
5. Handle connection pooling

**Template:**
```rust
use sqlx::{PgPool, postgres::PgPoolOptions};

pub struct PostgresAdapter {
    pool: PgPool,
}

impl PostgresAdapter {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        
        Ok(Self { pool })
    }
    
    pub async fn fetch_all(&self) -> Result<Vec<Repository>> {
        let rows = sqlx::query!(
            r#"
            SELECT url, description, language, tags
            FROM repositories
            WHERE active = true
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        // Map rows to Repository objects
        Ok(vec![])
    }
}
```

---

## Repository Model Reference

When converting external data to Repository, you must populate these fields:

```rust
pub struct Repository {
    /// Unique identifier (format: "{source}-{platform}-{owner}-{name}")
    pub id: String,

    /// Platform information (can have multiple for mirrors)
    pub platforms: Vec<PlatformInfo>,

    /// Core metadata
    pub metadata: RepositoryMetadata,

    /// Classification for organization
    pub classification: RepositoryClassification,

    /// Quality metrics
    pub quality_metrics: QualityMetrics,

    /// Data source (GitHubStars, Manual, Derived)
    pub source: RepositorySource,

    /// When added to system
    pub added_date: Option<String>,

    /// Whether manually curated
    pub manually_curated: bool,

    /// Curator notes
    pub curator_notes: Option<String>,
}
```

### Required Fields

**Minimum viable repository:**
```rust
Repository {
    id: "source-platform-owner-name".to_string(),
    platforms: vec![PlatformInfo {
        platform: Platform::GitHub,  // or GitLab, Codeberg
        url: "https://github.com/owner/name".to_string(),
        status: PlatformStatus::Active,
        is_primary: true,
        migration_date: None,
        last_verified: Some(today),
        notes: None,
    }],
    metadata: RepositoryMetadata {
        name: "repo-name".to_string(),
        owner: "owner-name".to_string(),
        full_name: "owner/name".to_string(),
        description: "Description here".to_string(),
        primary_language: "Rust".to_string(),
        // ... other fields can be None or default
    },
    // ... other sections with defaults
}
```

---

## Testing Your Integration

### 1. Unit Tests

Test parsing, conversion, error handling:

```rust
#[test]
fn test_parse_identifier() {
    let adapter = MyAdapter::new(/* ... */);
    let result = adapter.parse_identifier("test-input");
    assert!(result.is_ok());
}
```

### 2. Integration Tests

Test actual API calls (with mocks):

```rust
#[tokio::test]
async fn test_fetch_repository() {
    let adapter = MyAdapter::new(/* ... */);
    let repo = adapter.fetch_repository("test-123").await;
    assert!(repo.is_ok());
}
```

### 3. End-to-End Test

```bash
# 1. Configure your source
# 2. Run sync
cargo run -- sync --from-mysource

# 3. Verify data
cargo run -- stats

# 4. Check specific repo
cargo run -- validate
```

---

## Best Practices

### 1. Error Handling

```rust
// Provide context for debugging
let repo = adapter.fetch_repository(id).await
    .context(format!("Failed to fetch repository: {}", id))?;

// Log warnings, don't fail on single errors
match adapter.fetch_repository(id).await {
    Ok(repo) => repositories.push(repo),
    Err(e) => {
        log::warn!("Failed to fetch {}: {}", id, e);
        failed_count += 1;
        continue;  // Process other repositories
    }
}
```

### 2. Rate Limiting

```rust
use tokio::time::{sleep, Duration};

for (i, item) in items.iter().enumerate() {
    let repo = adapter.fetch_repository(&item.id).await?;
    
    // Check rate limit every 100 items
    if i % 100 == 0 {
        adapter.check_rate_limit().await?;
    }
    
    // Add delay to avoid overwhelming API
    sleep(Duration::from_millis(100)).await;
}
```

### 3. Caching

```rust
// Use SyncCache for performance
if let Some(cache_entry) = cache.get(&repo_id) {
    if cache_entry.is_fresh(24) {  // 24 hour TTL
        return Ok(cache_entry.to_repository());
    }
}

// Fetch fresh data
let repo = adapter.fetch_repository(id).await?;
cache.insert(&repo_id, &repo.metadata);
```

### 4. Progress Feedback

```rust
use crate::sync::ProgressTracker;

let mut progress = ProgressTracker::new();
progress.start(total_items);

for item in items {
    match process_item(item).await {
        Ok(_) => progress.increment_synced(&item.name),
        Err(_) => progress.increment_failed(),
    }
}

progress.finish();
```

---

## Adapter Comparison

| Feature | GitHub | Google Sheets | Jira | Git Local |
|---------|--------|---------------|------|-----------|
| **Authentication** | Token | Service Account | API Token | None |
| **Rate Limits** | 5000/hr | 100/100s | Varies | N/A |
| **Real-time Data** | Yes | Yes | Yes | No |
| **Bulk Fetch** | Sequential | Single read | JQL query | Parallel scan |
| **Caching** | ETag | N/A | Recommended | 24h cache |
| **Complexity** | Low | Medium | Medium | Low |

---

## Configuration Schema

### Adding New Source to Config

Update [`src/config/settings.rs`](../src/config/settings.rs):

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncConfig {
    // Existing
    pub enabled: bool,
    pub interval_hours: u32,
    pub parallel_workers: u8,
    pub cache_ttl_hours: u32,
    pub rate_limit_buffer: u16,
    pub request_timeout_secs: u64,
    
    // Add new source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub my_source: Option<MySourceConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MySourceConfig {
    pub enabled: bool,
    pub api_url: String,
    // Add source-specific config
}
```

### Config File Format

```toml
[sync]
enabled = true
interval_hours = 24
parallel_workers = 3
cache_ttl_hours = 24
rate_limit_buffer = 500
request_timeout_secs = 30

[sync.my_source]
enabled = true
api_url = "https://api.example.com"
```

---

## CLI Integration

### Add Command Flag

Update [`src/main.rs`](../src/main.rs) Commands::Sync:

```rust
Sync {
    // Existing flags
    #[arg(long)]
    repos: Option<String>,
    
    #[arg(long, default_value = "false")]
    force: bool,
    
    // Add new source flag
    #[arg(long)]
    from_mysource: bool,
    
    #[arg(short, long, default_value = "data/canonical/repositories.yml")]
    input: PathBuf,
}
```

### Handle in Command Match

```rust
Commands::Sync { from_mysource, .. } => {
    if from_mysource {
        let repos = orchestrator.sync_from_mysource().await?;
        println!("Fetched {} repos from MySource", repos.len());
    } else {
        let result = orchestrator.sync_all(&input).await?;
        println!("Synced: {}", result.synced);
    }
}
```

---

## Common Integration Tasks

### Task 1: Add Dependencies

```bash
# Add to Cargo.toml
cargo add your-api-client
cargo add reqwest  # for HTTP
cargo add serde_json  # for JSON parsing
```

### Task 2: Create Adapter File

```bash
# Create new adapter
touch src/sync/adapters/mysource.rs

# Register in mod.rs
echo "pub mod mysource;" >> src/sync/adapters/mod.rs
echo "pub use mysource::MySourceAdapter;" >> src/sync/adapters/mod.rs
```

### Task 3: Implement Trait

See template above - implement all three required methods:
- `fetch_repository(identifier)` 
- `check_connection()`
- `source_name()`

### Task 4: Add Configuration

Add to [`settings.rs`](../src/config/settings.rs):
- Config struct for source-specific settings
- Add as optional field to SyncConfig

### Task 5: Update Orchestrator

Add to [`src/sync/mod.rs`](../src/sync/mod.rs):
```rust
pub async fn sync_from_mysource(&mut self) -> Result<Vec<Repository>> {
    // Load config, create adapter, fetch data
}
```

### Task 6: Add CLI Command

Update [`src/main.rs`](../src/main.rs) with new flag and handler

### Task 7: Test

```bash
cargo test mysource::
cargo run -- sync --from-mysource
cargo run -- stats
```

---

## Troubleshooting

### Build Errors

**"trait `DataSourceAdapter` is not implemented"**
- Ensure `#[async_trait]` macro is applied
- Verify all three methods are implemented
- Check return types match trait definition

**"lifetime mismatch"**
- Use `'static` lifetime for async trait
- Ensure structs implement `Send + Sync`

### Runtime Errors

**"Failed to connect to source"**
- Check network connectivity
- Verify credentials are configured
- Test with curl or API explorer first

**"Parse error"**
- Log raw response for debugging
- Verify API response format matches expectations
- Handle null/missing fields gracefully

---

## Performance Guidelines

### Batch Operations

```rust
// Prefer batch fetches over individual calls
pub async fn fetch_batch(&self, identifiers: Vec<String>) -> Result<Vec<Repository>> {
    // Single API call for multiple items
}
```

### Parallel Processing

```rust
use futures::stream::{self, StreamExt};

let repos = stream::iter(identifiers)
    .map(|id| async move {
        adapter.fetch_repository(&id).await
    })
    .buffer_unordered(3)  // 3 concurrent requests
    .collect::<Vec<_>>()
    .await;
```

### Memory Management

```rust
// Process in batches for large datasets
for batch in identifiers.chunks(100) {
    let repos = fetch_batch(batch).await?;
    save_to_canonical(&repos)?;
    // Batch goes out of scope, memory freed
}
```

---

## Security Considerations

1. **Credentials:** Never hardcode, use CredentialManager
2. **Logging:** Redact sensitive data in logs
3. **File Permissions:** Secure credential files (0600)
4. **Network:** Use HTTPS, verify certificates
5. **Injection:** Sanitize user input in queries

---

## Additional Resources

### Example Implementations

- **GitHub:** [`src/sync/adapters/github.rs`](../src/sync/adapters/github.rs) - Full production implementation
- **Codeberg:** [`src/sync/adapters/codeberg.rs`](../src/sync/adapters/codeberg.rs) - Stub for future
- **GitLab:** [`src/sync/adapters/gitlab.rs`](../src/sync/adapters/gitlab.rs) - Stub for future

### Documentation

- **Design Doc:** [`.specly_dev/specs/.../design.md`](../.specly_dev/specs/20251211-081212-external-data-source-sync-and-documentation-restructure/design.md)
- **API Reference:** [`docs/API_REFERENCE.md`](API_REFERENCE.md)
- **Architecture:** [`docs/ARCHITECTURE.md`](ARCHITECTURE.md)

### Community

- **Issues:** Report integration problems
- **Discussions:** Share new adapter ideas
- **Pull Requests:** Contribute adapters

---

## Quick Reference Card

### Minimal Adapter Implementation

```rust
use super::DataSourceAdapter;
use async_trait::async_trait;
use crate::models::Repository;
use anyhow::Result;

pub struct MyAdapter {
    client: reqwest::Client,
}

#[async_trait]
impl DataSourceAdapter for MyAdapter {
    async fn fetch_repository(&self, id: &str) -> Result<Repository> {
        // Fetch and convert
        todo!()
    }
    
    async fn check_connection(&self) -> Result<()> {
        // Quick health check
        Ok(())
    }
    
    fn source_name(&self) -> &str {
        "my_source"
    }
}
```

### Typical File Structure

```
src/sync/adapters/
├── mod.rs              # Trait definition, exports
├── github.rs           # GitHub implementation ✅
├── google_sheets.rs    # Your implementation
├── jira.rs             # Your implementation
├── git_local.rs        # Your implementation
└── mysource.rs         # New implementation
```

### Integration Checklist

- [ ] Dependencies added to Cargo.toml
- [ ] Adapter file created in src/sync/adapters/
- [ ] DataSourceAdapter trait implemented
- [ ] Module registered in mod.rs
- [ ] Config struct added to settings.rs
- [ ] Credentials added to CredentialManager
- [ ] Orchestrator method added
- [ ] CLI command updated
- [ ] Tests written
- [ ] Documentation added
- [ ] Integration guide created

---

This index provides everything needed to integrate any external data source with OmniDatum's sync system.