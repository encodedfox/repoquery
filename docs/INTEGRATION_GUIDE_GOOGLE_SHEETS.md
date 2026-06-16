# Integration Guide: Google Sheets Data Source

## Overview

This guide shows how to integrate Google Sheets as an external data source for OmniDatum. This is useful for maintaining curated lists in spreadsheets that can be synced into the canonical repository data.

**Use Case:** Maintain a Google Sheet with columns for repository URL, description, tags, quality notes that gets synced into OmniDatum's canonical format.

---

## Prerequisites

### Dependencies

Add to [`Cargo.toml`](../Cargo.toml):

```toml
[dependencies]
# ... existing dependencies ...

# Google Sheets API
google-sheets4 = "5.0"
yup-oauth2 = "9.0"
hyper = "1.0"
hyper-rustls = "0.27"
```

### Google Cloud Setup

1. Create a Google Cloud Project at https://console.cloud.google.com
2. Enable Google Sheets API
3. Create Service Account or OAuth2 credentials
4. Download credentials JSON file
5. Store at `~/.config/omnidatum/google-credentials.json`

---

## DataSourceAdapter Trait Reference

Located in [`src/sync/adapters/mod.rs`](../src/sync/adapters/mod.rs):

```rust
use crate::models::Repository;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait DataSourceAdapter: Send + Sync {
    /// Fetch repository data from external source
    ///
    /// # Arguments
    /// * `identifier` - Source-specific identifier (e.g., "Sheet1!A2:F100" for Sheets)
    ///
    /// # Returns
    /// Repository data with metadata from the source
    ///
    /// # Errors
    /// Returns error if fetch fails, invalid format, or authentication issues
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository>;

    /// Check if connection to source is working
    ///
    /// # Returns
    /// Ok(()) if connection is healthy, Err with details otherwise
    async fn check_connection(&self) -> Result<()>;

    /// Get source name for logging
    ///
    /// # Returns
    /// Human-readable source name (e.g., "google_sheets")
    fn source_name(&self) -> &str {
        "unknown"
    }
}
```

---

## Implementation

### Step 1: Create Google Sheets Adapter

Create [`src/sync/adapters/google_sheets.rs`](../src/sync/adapters/google_sheets.rs):

```rust
//! Google Sheets data source adapter

use super::DataSourceAdapter;
use crate::config::OmnidatumConfig;
use crate::models::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository,
    RepositoryClassification, RepositoryMetadata, RepositorySource,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use google_sheets4::{
    api::ValueRange,
    hyper, hyper_rustls,
    oauth2::{self, ServiceAccountAuthenticator},
    Sheets,
};

/// Google Sheets adapter
///
/// Reads repository data from a Google Sheet with expected columns:
/// A: Repository URL
/// B: Description
/// C: Primary Language
/// D: Tags (comma-separated)
/// E: License
/// F: Quality Notes
pub struct GoogleSheetsAdapter {
    hub: Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    spreadsheet_id: String,
    range: String,
}

impl GoogleSheetsAdapter {
    /// Create new Google Sheets adapter
    ///
    /// # Arguments
    /// * `config` - Application configuration
    /// * `spreadsheet_id` - Google Sheets document ID
    /// * `range` - Cell range (e.g., "Sheet1!A2:F100")
    ///
    /// # Returns
    /// Configured adapter ready to fetch data
    pub async fn new(
        config: &OmnidatumConfig,
        spreadsheet_id: String,
        range: String,
    ) -> Result<Self> {
        // Load service account credentials
        let creds_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("omnidatum")
            .join("google-credentials.json");

        let service_account_key = yup_oauth2::read_service_account_key(&creds_path)
            .await
            .context("Failed to read Google service account credentials")?;

        // Create authenticator
        let auth = ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await
            .context("Failed to create Google authenticator")?;

        // Create Sheets API client
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_only()
            .enable_http1()
            .build();

        let client = hyper::Client::builder().build(connector);
        let hub = Sheets::new(client, auth);

        log::info!("Google Sheets adapter initialized for spreadsheet: {}", spreadsheet_id);

        Ok(Self {
            hub,
            spreadsheet_id,
            range,
        })
    }

    /// Parse row data into Repository
    fn parse_row(&self, row: &[String], row_index: usize) -> Result<Repository> {
        if row.len() < 2 {
            return Err(anyhow::anyhow!(
                "Row {} has insufficient columns (need at least URL and description)",
                row_index
            ));
        }

        let url = row[0].trim();
        let description = row.get(1).map(|s| s.trim()).unwrap_or("");
        let language = row.get(2).map(|s| s.trim()).unwrap_or("Unknown");
        let tags = row.get(3).map(|s| s.trim()).unwrap_or("");
        let license = row.get(4).map(|s| s.trim().to_string());
        let notes = row.get(5).map(|s| s.trim().to_string());

        // Parse URL to extract owner/name
        let (platform, owner, name) = self.parse_repository_url(url)?;

        let full_name = format!("{}/{}", owner, name);
        let id = format!("sheets-{}-{}", platform.to_lowercase(), full_name.replace('/', "-"));

        Ok(Repository {
            id,
            platforms: vec![PlatformInfo {
                platform,
                url: url.to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: name.to_string(),
                owner: owner.to_string(),
                full_name,
                description: description.to_string(),
                primary_language: language.to_string(),
                license,
                license_spdx: None,
                stars: 0, // Not available from sheets
                topics: tags
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: language.to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: notes,
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: None,
                last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                quality_score: 70,
            },
            source: RepositorySource::Manual, // From curated sheet
            added_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            manually_curated: true,
            curator_notes: notes,
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
        } else if url.contains("codeberg.org") {
            let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[parts.len() - 2];
                let name = parts[parts.len() - 1];
                return Ok((Platform::Codeberg, owner.to_string(), name.to_string()));
            }
        } else if url.contains("gitlab.com") {
            let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[parts.len() - 2];
                let name = parts[parts.len() - 1];
                return Ok((Platform::GitLab, owner.to_string(), name.to_string()));
            }
        }

        Err(anyhow::anyhow!("Could not parse repository URL: {}", url))
    }
}

#[async_trait]
impl DataSourceAdapter for GoogleSheetsAdapter {
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository> {
        // For sheets, identifier is the row index
        let row_index: usize = identifier
            .parse()
            .context("Identifier must be row index")?;

        // Fetch all rows
        let result = self
            .hub
            .spreadsheets()
            .values_get(&self.spreadsheet_id, &self.range)
            .doit()
            .await
            .context("Failed to fetch data from Google Sheets")?;

        let (_, value_range) = result;
        let values = value_range
            .values
            .ok_or_else(|| anyhow::anyhow!("No data in sheet"))?;

        if row_index >= values.len() {
            return Err(anyhow::anyhow!("Row index {} out of range", row_index));
        }

        // Convert row values to strings
        let row: Vec<String> = values[row_index]
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        self.parse_row(&row, row_index)
    }

    async fn check_connection(&self) -> Result<()> {
        // Try to read just first row to verify access
        let result = self
            .hub
            .spreadsheets()
            .values_get(&self.spreadsheet_id, "A1")
            .doit()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Google Sheets connection failed: {}", e)),
        }
    }

    fn source_name(&self) -> &str {
        "google_sheets"
    }
}

/// Helper to fetch all repositories from sheet
impl GoogleSheetsAdapter {
    /// Fetch all rows from the configured range
    ///
    /// # Returns
    /// Vector of Repository objects, one per row
    pub async fn fetch_all(&self) -> Result<Vec<Repository>> {
        let result = self
            .hub
            .spreadsheets()
            .values_get(&self.spreadsheet_id, &self.range)
            .doit()
            .await
            .context("Failed to fetch data from Google Sheets")?;

        let (_, value_range) = result;
        let values = value_range
            .values
            .ok_or_else(|| anyhow::anyhow!("No data in sheet"))?;

        let mut repositories = Vec::new();

        for (idx, row_values) in values.iter().enumerate() {
            let row: Vec<String> = row_values
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect();

            match self.parse_row(&row, idx) {
                Ok(repo) => repositories.push(repo),
                Err(e) => {
                    log::warn!("Failed to parse row {}: {}", idx, e);
                    continue;
                }
            }
        }

        log::info!("Fetched {} repositories from Google Sheets", repositories.len());
        Ok(repositories)
    }
}
```

### Step 2: Register Adapter

Update [`src/sync/adapters/mod.rs`](../src/sync/adapters/mod.rs):

```rust
// Add module
pub mod google_sheets;
pub use google_sheets::GoogleSheetsAdapter;
```

### Step 3: Add Configuration

Update [`src/config/settings.rs`](../src/config/settings.rs) to add Google Sheets config:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncConfig {
    // ... existing fields ...
    
    /// Google Sheets integration settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_sheets: Option<GoogleSheetsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleSheetsConfig {
    pub enabled: bool,
    pub spreadsheet_id: String,
    pub range: String,
}
```

### Step 4: Update Configuration File

Add to `~/.config/omnidatum/config.toml`:

```toml
[sync.google_sheets]
enabled = true
spreadsheet_id = "1ABC...XYZ"  # Your Google Sheets document ID
range = "Sheet1!A2:F100"        # Range containing repository data
```

### Step 5: Integrate with SyncOrchestrator

Update [`src/sync/mod.rs`](../src/sync/mod.rs) to support Google Sheets:

```rust
impl SyncOrchestrator {
    /// Sync from Google Sheets if configured
    pub async fn sync_from_google_sheets(&mut self) -> Result<Vec<Repository>> {
        if let Some(sheets_config) = &self.config.sync.google_sheets {
            if sheets_config.enabled {
                log::info!("Syncing from Google Sheets: {}", sheets_config.spreadsheet_id);
                
                let adapter = GoogleSheetsAdapter::new(
                    &self.config,
                    sheets_config.spreadsheet_id.clone(),
                    sheets_config.range.clone(),
                ).await?;
                
                // Check connection
                adapter.check_connection().await?;
                
                // Fetch all rows
                let repos = adapter.fetch_all().await?;
                
                log::info!("Fetched {} repos from Google Sheets", repos.len());
                return Ok(repos);
            }
        }
        
        Ok(vec![])
    }
}
```

---

## Google Sheet Format

### Expected Columns

| Column | Header | Type | Required | Example |
|--------|--------|------|----------|---------|
| A | URL | String | ✅ Yes | `https://github.com/owner/repo` |
| B | Description | String | ✅ Yes | `Amazing project for XYZ` |
| C | Language | String | No | `Rust` |
| D | Tags | CSV | No | `database, distributed` |
| E | License | String | No | `MIT` |
| F | Notes | String | No | `Curator observations` |

### Example Sheet

```
| URL                                | Description              | Language | Tags              | License | Notes          |
|------------------------------------|--------------------------|----------|-------------------|---------|----------------|
| https://github.com/rust-lang/rust  | Systems programming lang | Rust     | compiler,systems  | MIT     | Core language  |
| https://github.com/tikv/tikv       | Distributed KV store     | Rust     | database,raft     | Apache  | Production use |
```

---

## Usage

### CLI Integration

Add sync source flag to sync command:

```rust
/// Sync repository metadata from external sources
Sync {
    // ... existing fields ...
    
    /// Sync from Google Sheets instead of GitHub
    #[arg(long)]
    from_sheets: bool,
}
```

### Sync from Sheets

```bash
# Sync from Google Sheets
cargo run -- sync --from-sheets

# Merge with existing data
cargo run -- merge
```

---

## Testing

### Unit Tests

Add to `src/sync/adapters/google_sheets.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url() {
        let adapter = create_test_adapter();
        let result = adapter.parse_repository_url("https://github.com/rust-lang/rust");
        assert!(result.is_ok());
        let (platform, owner, name) = result.unwrap();
        assert_eq!(platform, Platform::GitHub);
        assert_eq!(owner, "rust-lang");
        assert_eq!(name, "rust");
    }

    #[test]
    fn test_parse_row_minimal() {
        let adapter = create_test_adapter();
        let row = vec![
            "https://github.com/test/repo".to_string(),
            "Test repository".to_string(),
        ];
        
        let result = adapter.parse_row(&row, 0);
        assert!(result.is_ok());
        let repo = result.unwrap();
        assert_eq!(repo.metadata.full_name, "test/repo");
        assert_eq!(repo.metadata.description, "Test repository");
    }

    fn create_test_adapter() -> GoogleSheetsAdapter {
        // Create stub adapter for testing (no actual API calls)
        GoogleSheetsAdapter {
            hub: create_mock_hub(),
            spreadsheet_id: "test".to_string(),
            range: "A1:F10".to_string(),
        }
    }
}
```

### Integration Test

```rust
#[tokio::test]
async fn test_google_sheets_sync() {
    // This requires actual Google Sheets credentials
    // Skip if not configured
    let creds_path = dirs::config_dir()
        .unwrap()
        .join("omnidatum")
        .join("google-credentials.json");
    
    if !creds_path.exists() {
        println!("Skipping: Google credentials not configured");
        return;
    }

    let config = OmnidatumConfig::default();
    let adapter = GoogleSheetsAdapter::new(
        &config,
        "test_spreadsheet_id".to_string(),
        "Sheet1!A2:F10".to_string(),
    ).await.unwrap();

    // Test connection
    let result = adapter.check_connection().await;
    assert!(result.is_ok());
}
```

---

## Error Handling

### Common Errors

**Authentication Failed:**
```
Error: Failed to read Google service account credentials

Solution: 
1. Download credentials JSON from Google Cloud Console
2. Save to ~/.config/omnidatum/google-credentials.json
3. Ensure file has proper read permissions
```

**API Quota Exceeded:**
```
Error: Quota exceeded for quota metric 'Read requests'

Solution:
1. Google Sheets API has rate limits (100 requests/100 seconds/user by default)
2. Add delays between requests
3. Request quota increase in Google Cloud Console
```

**Invalid Range:**
```
Error: Unable to parse range: Sheet1!A2:F100

Solution:
1. Verify sheet name matches exactly
2. Ensure range is valid (A1 notation)
3. Check sheet has data in specified range
```

---

## Configuration Reference

### Complete Config Example

`~/.config/omnidatum/config.toml`:

```toml
[sync]
enabled = true
interval_hours = 24
parallel_workers = 3
cache_ttl_hours = 24
rate_limit_buffer = 500
request_timeout_secs = 30

[sync.google_sheets]
enabled = true
spreadsheet_id = "1ABC...XYZ"
range = "Sheet1!A2:F100"

[credentials]
source = "env"
```

### Environment Variables

```bash
# Google credentials (if using OAuth instead of service account)
export GOOGLE_APPLICATION_CREDENTIALS=~/.config/omnidatum/google-credentials.json

# GitHub token (for hybrid sync)
export GITHUB_TOKEN=ghp_your_token_here
```

---

## Advanced Usage

### Hybrid Sync (Sheets + GitHub)

Sync curated list from Sheets, then enrich with GitHub API data:

```rust
// 1. Fetch from sheets
let sheet_repos = orchestrator.sync_from_google_sheets().await?;

// 2. Add to canonical data
for repo in sheet_repos {
    canonical_data.repositories.push(repo);
}

// 3. Sync GitHub repos to update stars, etc.
orchestrator.sync_all(&canonical_path).await?;
```

### Scheduled Updates

Use cron to sync daily:

```bash
# crontab -e
0 2 * * * cd /path/to/omnidatum && cargo run --release -- sync --from-sheets && cargo run --release -- generate
```

---

## Performance Considerations

- **Rate Limits:** Google Sheets API has 100 reads/100s/user limit
- **Batch Size:** Fetch all rows in single request (more efficient)
- **Caching:** Cache sheet data for 24h, only refresh on change
- **Authentication:** Service account is faster than OAuth for automated sync

---

## Security Notes

1. **Service Account Key**: Store `google-credentials.json` with 0600 permissions
2. **Spreadsheet Access**: Grant service account read access to specific sheets only
3. **No Write Access**: Adapter only reads, never writes to sheets
4. **Audit Logging**: All sheet accesses logged with timestamps

---

## Troubleshooting

**Issue: "Failed to read Google service account credentials"**
- Check file exists at `~/.config/omnidatum/google-credentials.json`
- Verify JSON format is valid
- Ensure file permissions allow reading

**Issue: "The caller does not have permission"**
- Share spreadsheet with service account email
- Grant "Viewer" role minimum
- Check spreadsheet ID is correct

**Issue: "Request had insufficient authentication scopes"**
- Service account needs `https://www.googleapis.com/auth/spreadsheets.readonly` scope
- Regenerate credentials with correct scopes

---

## Complete Example

### 1. Setup

```bash
# Install dependencies
cargo add google-sheets4 yup-oauth2 hyper hyper-rustls

# Configure
echo 'spreadsheet_id = "YOUR_SHEET_ID"' >> ~/.config/omnidatum/config.toml
```

### 2. Create Adapter File

Copy complete implementation to `src/sync/adapters/google_sheets.rs`

### 3. Register Module

Add to `src/sync/adapters/mod.rs`:
```rust
pub mod google_sheets;
pub use google_sheets::GoogleSheetsAdapter;
```

### 4. Run Sync

```bash
cargo run -- sync --from-sheets
```

### 5. Verify Results

```bash
cargo run -- stats
# Should show repos from sheets
```

---

## API Reference

### Google Sheets API v4

**Documentation:** https://developers.google.com/sheets/api/reference/rest

**Key Endpoints Used:**
- `spreadsheets.values.get` - Read cell values
- Authentication via Service Account or OAuth2

**Rate Limits:**
- Read requests: 100 per 100 seconds per user
- Write requests: 100 per 100 seconds per user (not used)

### Dependencies

- **google-sheets4**: Official Google Sheets API client
- **yup-oauth2**: OAuth2 authentication for Google APIs
- **hyper**: HTTP client (required by google-sheets4)
- **hyper-rustls**: TLS support for hyper

---

This guide provides everything needed to integrate Google Sheets as a data source for OmniDatum.