//! Codeberg API adapter (stub for future implementation)

use super::DataSourceAdapter;
use anyhow::Result;
use async_trait::async_trait;
use rq_core::Repository;

/// Codeberg API adapter
///
/// TODO: Implement Codeberg API integration
/// This is a stub for future implementation
pub struct CodebergAdapter;

impl CodebergAdapter {
    /// Create new Codeberg adapter
    pub async fn new() -> Result<Self> {
        Err(anyhow::anyhow!("Codeberg adapter not yet implemented"))
    }
}

#[async_trait]
impl DataSourceAdapter for CodebergAdapter {
    async fn fetch_repository(&self, _identifier: &str) -> Result<Repository> {
        Err(anyhow::anyhow!("Codeberg adapter not yet implemented"))
    }

    async fn check_connection(&self) -> Result<()> {
        Err(anyhow::anyhow!("Codeberg adapter not yet implemented"))
    }

    fn source_name(&self) -> &str {
        "codeberg"
    }
}
