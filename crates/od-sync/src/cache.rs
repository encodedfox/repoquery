//! Sync cache with ETag support

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Cache entry for synced repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub repo_id: String,
    pub last_sync: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    pub stars: u32,
    pub archive_status: bool,
}

impl CacheEntry {
    /// Check if cache entry is fresh (within TTL)
    pub fn is_fresh(&self, ttl_hours: u32) -> bool {
        let now = chrono::Utc::now();
        let age = now - self.last_sync;
        age.num_hours() < ttl_hours as i64
    }
}

/// Sync cache manager
pub struct SyncCache {
    entries: HashMap<String, CacheEntry>,
    cache_path: PathBuf,
}

impl SyncCache {
    /// Load cache from file
    pub fn load() -> Result<Self> {
        let cache_path = Self::cache_file_path();

        let entries = if cache_path.exists() {
            let content = std::fs::read_to_string(&cache_path)?;
            serde_json::from_str(&content).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse cache file: {}. Starting with empty cache.", e);
                HashMap::new()
            })
        } else {
            HashMap::new()
        };

        Ok(Self {
            entries,
            cache_path,
        })
    }

    /// Get cache entry
    pub fn get(&self, repo_id: &str) -> Option<&CacheEntry> {
        self.entries.get(repo_id)
    }

    /// Insert or update cache entry
    pub fn insert(&mut self, repo_id: &str, metadata: &od_core::RepositoryMetadata) {
        let entry = CacheEntry {
            repo_id: repo_id.to_string(),
            last_sync: chrono::Utc::now(),
            etag: None, // TODO: Get from response headers in future optimization
            sha: None,  // TODO: Get from Git commit in future optimization
            stars: metadata.stars,
            archive_status: false, // TODO: Get from repo in future
        };

        self.entries.insert(repo_id.to_string(), entry);
    }

    /// Save cache to file
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.entries)?;
        
        // Check size before write
        let content_len = content.len();
        std::fs::write(&self.cache_path, content)?;

        // Prune old entries if cache is large (>10MB)
        if content_len > 10_000_000 {
            tracing::info!("Cache size exceeds 10MB, pruning recommended");
            // TODO: Implement pruning logic in future task
        }

        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        self.save()
    }

    /// Get cache file path
    fn cache_file_path() -> PathBuf {
        PathBuf::from("data/cache/sync_metadata.json")
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let oldest = self
            .entries
            .values()
            .map(|e| e.last_sync)
            .min()
            .unwrap_or_else(chrono::Utc::now);

        CacheStats {
            total_entries: self.entries.len(),
            oldest_entry: oldest,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub oldest_entry: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_is_fresh() {
        let entry = CacheEntry {
            repo_id: "test".to_string(),
            last_sync: chrono::Utc::now(),
            etag: None,
            sha: None,
            stars: 100,
            archive_status: false,
        };

        // Entry just created should be fresh within 24 hours
        assert!(entry.is_fresh(24));
    }

    #[test]
    fn test_cache_entry_is_stale() {
        let old_time = chrono::Utc::now() - chrono::Duration::hours(48);
        let entry = CacheEntry {
            repo_id: "test".to_string(),
            last_sync: old_time,
            etag: None,
            sha: None,
            stars: 100,
            archive_status: false,
        };

        // Entry from 48 hours ago should be stale within 24 hour TTL
        assert!(!entry.is_fresh(24));
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = SyncCache {
            entries: HashMap::new(),
            cache_path: PathBuf::from("test_cache.json"),
        };

        let metadata = od_core::RepositoryMetadata {
            name: "test".to_string(),
            owner: "owner".to_string(),
            full_name: "owner/test".to_string(),
            description: "Test".to_string(),
            primary_language: "Rust".to_string(),
            license: None,
            license_spdx: None,
            stars: 150,
            topics: vec![],
            homepage: None,
            language_breakdown: None,
            secondary_languages: vec![],
        };

        cache.insert("test-repo", &metadata);

        let entry = cache.get("test-repo");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().stars, 150);
    }

    #[test]
    fn test_cache_clear() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = SyncCache {
            entries: HashMap::new(),
            cache_path: dir.path().join("test_cache_clear.json"),
        };

        let metadata = od_core::RepositoryMetadata {
            name: "test".to_string(),
            owner: "owner".to_string(),
            full_name: "owner/test".to_string(),
            description: "Test".to_string(),
            primary_language: "Rust".to_string(),
            license: None,
            license_spdx: None,
            stars: 100,
            topics: vec![],
            homepage: None,
            language_breakdown: None,
            secondary_languages: vec![],
        };

        cache.insert("test", &metadata);
        assert_eq!(cache.entries.len(), 1);

        cache.clear().ok(); // Ignore file write error in test
        assert_eq!(cache.entries.len(), 0);
    }
}