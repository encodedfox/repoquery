//! GitLab API adapter (stub for future implementation)

use super::DataSourceAdapter;
use od_core::Repository;
use anyhow::Result;
use async_trait::async_trait;

/// GitLab API adapter
///
/// TODO: Implement GitLab API integration
/// This is a stub for future implementation
pub struct GitLabAdapter;

impl GitLabAdapter {
    /// Create new GitLab adapter
    pub async fn new() -> Result<Self> {
        unimplemented!("GitLab adapter not yet implemented")
    }
}

#[async_trait]
impl DataSourceAdapter for GitLabAdapter {
    async fn fetch_repository(&self, _identifier: &str) -> Result<Repository> {
        unimplemented!("GitLab adapter not yet implemented")
    }

    async fn check_connection(&self) -> Result<()> {
        unimplemented!("GitLab adapter not yet implemented")
    }

    fn source_name(&self) -> &str {
        "gitlab"
    }
}