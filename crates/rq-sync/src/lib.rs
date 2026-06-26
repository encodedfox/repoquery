//! od-sync: External source sync and adapters.

pub mod adapters;
pub mod cache;
pub mod client;
pub mod error;
pub mod progress;

mod orchestrator;

pub use adapters::{DataSourceAdapter, GitHubAdapter};
pub use cache::{CacheEntry, SyncCache};
pub use client::{redact_sensitive, retry_with_backoff};
pub use error::SyncError;
pub use orchestrator::SyncOrchestrator;
pub use progress::ProgressTracker;

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub total: usize,
    pub synced: usize,
    pub cached: usize,
    pub failed: usize,
    pub duration: std::time::Duration,
    pub failures: Vec<(String, String)>,
    /// Count of repos synced per relation type (e.g. "starred" -> 42)
    pub by_relation: std::collections::HashMap<String, usize>,
}
