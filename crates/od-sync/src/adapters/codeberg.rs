//! Codeberg API adapter (stub for future implementation)

use super::DataSourceAdapter;
use od_core::Repository;
use anyhow::Result;
use async_trait::async_trait;

/// Codeberg API adapter
///
/// TODO: Implement Codeberg API integration
/// This is a stub for future implementation
pub struct CodebergAdapter;

impl CodebergAdapter {
    /// Create new Codeberg adapter
    pub async fn new() -> Result<Self> {
        unimplemented!("Codeberg adapter not yet implemented")
    }
}

#[async_trait]
impl DataSourceAdapter for CodebergAdapter {
    async fn fetch_repository(&self, _identifier: &str) -> Result<Repository> {
        unimplemented!("Codeberg adapter not yet implemented")
    }

    async fn check_connection(&self) -> Result<()> {
        unimplemented!("Codeberg adapter not yet implemented")
    }

    fn source_name(&self) -> &str {
        "codeberg"
    }
}