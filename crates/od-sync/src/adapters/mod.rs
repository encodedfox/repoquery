//! Data source adapters for external platforms

use od_core::Repository;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for data source adapters (GitHub, Codeberg, GitLab, etc.)
#[async_trait]
pub trait DataSourceAdapter: Send + Sync {
    /// Fetch repository data from external source
    ///
    /// # Arguments
    /// * `identifier` - Repository identifier (e.g., "owner/name" for GitHub)
    ///
    /// # Returns
    /// Repository data with metadata from the source
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository>;

    /// Check if connection to source is working
    ///
    /// # Returns
    /// Ok(()) if connection is healthy, Err otherwise
    async fn check_connection(&self) -> Result<()>;

    /// Get source name for logging
    fn source_name(&self) -> &str {
        "unknown"
    }
}

// Re-export adapters
pub mod github;
pub use github::GitHubAdapter;

pub mod graphql;
pub use graphql::GitHubGraphQL;

// Stubs for future implementation
pub mod codeberg;
pub use codeberg::CodebergAdapter;

pub mod gitlab;
pub use gitlab::GitLabAdapter;