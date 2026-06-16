//! Sync operation metadata and quality reporting

use serde::{Deserialize, Serialize};

/// Metadata about a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    pub last_sync: chrono::DateTime<chrono::Utc>,
    pub sync_duration_ms: u64,
    pub repos_synced: usize,
    pub repos_cached: usize,
    pub repos_failed: usize,
    pub rate_limit_remaining: u32,
    pub rate_limit_reset: chrono::DateTime<chrono::Utc>,
}

/// Quality report after sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQualityReport {
    pub timestamp: String,
    pub repos_synced: usize,
    pub data_completeness: DataCompletenessMetrics,
    pub anomalies: Vec<DataAnomaly>,
}

/// Data completeness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataCompletenessMetrics {
    /// Percentage with license information
    pub license_coverage: f32,
    /// Percentage with description
    pub description_coverage: f32,
    /// Percentage with topics
    pub topics_coverage: f32,
    /// Percentage with homepage
    pub homepage_coverage: f32,
}

/// Data anomaly detected during sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAnomaly {
    pub repo_id: String,
    pub anomaly_type: AnomalyType,
    pub message: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Types of data anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    /// Star count dropped significantly
    StarDrop,
    /// Star count spiked significantly
    StarSpike,
    /// Description changed substantially
    DescriptionChange,
    /// Primary language changed
    LanguageChange,
    /// Repository newly archived
    NewlyArchived,
}

impl SyncQualityReport {
    /// Save report to JSON file
    pub fn to_json_file(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(path, content)?;
        Ok(())
    }
}