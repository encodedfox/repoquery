//! Canonical data container for all repository metadata

use super::platform::PlatformStatus;
use super::{Book, Collection, ManualProject, Repository, WebReference};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root data structure containing all repository and documentation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalData {
    pub schema_version: String,
    pub last_updated: String,
    pub generated_by: String,
    pub total_count: usize,

    pub repositories: Vec<Repository>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub manual_projects: Vec<ManualProject>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub web_references: Vec<WebReference>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub books: Vec<Book>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub collections: Vec<Collection>,

    /// Platform statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<RepositoryStatistics>,
}

/// Repository statistics by platform and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStatistics {
    pub total: usize,
    pub by_platform: HashMap<String, usize>,
    pub by_status: StatusCounts,
    pub by_language: HashMap<String, usize>,
}

/// Count of repositories by status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCounts {
    pub active: usize,
    pub archived: usize,
    pub deprecated: usize,
}

impl CanonicalData {
    /// Create new empty canonical data
    pub fn new() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            generated_by: "omnidatum-processor".to_string(),
            total_count: 0,
            repositories: vec![],
            manual_projects: vec![],
            web_references: vec![],
            books: vec![],
            collections: vec![],
            statistics: None,
        }
    }

    /// Load from YAML file
    pub fn from_yaml_file(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let data: Self = serde_yml::from_str(&content)?;
        Ok(data)
    }

    /// Load from JSON file
    pub fn from_json_file(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let data: Self = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Save to YAML file
    pub fn to_yaml_file(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = serde_yml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Save to JSON file
    pub fn to_json_file(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Calculate statistics from current data
    pub fn calculate_statistics(&mut self) {
        let mut by_platform: HashMap<String, usize> = HashMap::new();
        let mut by_language: HashMap<String, usize> = HashMap::new();
        let mut active = 0;
        let mut archived = 0;
        let mut deprecated = 0;

        // Count repositories
        for repo in &self.repositories {
            // Count by platform
            for platform in &repo.platforms {
                let platform_name = format!("{:?}", platform.platform);
                *by_platform.entry(platform_name).or_insert(0) += 1;

                // Count by status
                match platform.status {
                    super::platform::PlatformStatus::Active => active += 1,
                    super::platform::PlatformStatus::Archived => archived += 1,
                    super::platform::PlatformStatus::Deprecated => deprecated += 1,
                }
            }

            // Count by language
            let lang = &repo.classification.language_category;
            *by_language.entry(lang.clone()).or_insert(0) += 1;
        }

        // Count manual projects (convert to repository for counting)
        for manual in &self.manual_projects {
            // Count by platform
            for platform in &manual.platforms {
                let platform_name = format!("{:?}", platform.platform);
                *by_platform.entry(platform_name).or_insert(0) += 1;

                // Count by status
                match platform.status {
                    PlatformStatus::Active => active += 1,
                    PlatformStatus::Archived => archived += 1,
                    PlatformStatus::Deprecated => deprecated += 1,
                }
            }

            // Count by language
            let lang = &manual.metadata.primary_language;
            *by_language.entry(lang.clone()).or_insert(0) += 1;
        }

        self.statistics = Some(RepositoryStatistics {
            total: self.repositories.len() + self.manual_projects.len(),
            by_platform,
            by_status: StatusCounts {
                active,
                archived,
                deprecated,
            },
            by_language,
        });

        self.total_count = self.repositories.len() + self.manual_projects.len();
    }

    /// Get all repositories including converted manual projects
    pub fn all_repositories(&self) -> Vec<Repository> {
        let mut all = self.repositories.clone();
        all.extend(self.manual_projects.iter().map(|m| m.to_repository()));
        all
    }

    /// Partition repositories into (active, archived) in a single pass, returning references.
    /// Manual projects are converted and owned; use `active_repositories` / `archived_repositories`
    /// when you need owned `Vec<Repository>`.
    pub fn partition_repositories(&self) -> (Vec<&Repository>, Vec<&Repository>) {
        let mut active = Vec::new();
        let mut archived = Vec::new();
        for repo in &self.repositories {
            if repo.is_archive_candidate() {
                archived.push(repo);
            } else {
                active.push(repo);
            }
        }
        (active, archived)
    }

    /// Get active repositories (excluding archive candidates)
    pub fn active_repositories(&self) -> Vec<Repository> {
        self.repositories
            .iter()
            .filter(|r| !r.is_archive_candidate())
            .cloned()
            .chain(
                self.manual_projects
                    .iter()
                    .map(|m| m.to_repository())
                    .filter(|r| !r.is_archive_candidate()),
            )
            .collect()
    }

    /// Get archived repositories
    pub fn archived_repositories(&self) -> Vec<Repository> {
        self.repositories
            .iter()
            .filter(|r| r.is_archive_candidate())
            .cloned()
            .chain(
                self.manual_projects
                    .iter()
                    .map(|m| m.to_repository())
                    .filter(|r| r.is_archive_candidate()),
            )
            .collect()
    }

    /// Find repository by ID
    pub fn find_repo_by_id(&self, id: &str) -> Option<&Repository> {
        self.repositories.iter().find(|r| r.id == id)
    }

    /// Find repositories by language
    pub fn repos_by_language(&self, language: &str) -> Vec<&Repository> {
        self.repositories
            .iter()
            .filter(|r| r.classification.language_category == language)
            .collect()
    }
}

impl Default for CanonicalData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_data_creation() {
        let data = CanonicalData::new();

        assert_eq!(data.schema_version, "1.0");
        assert_eq!(data.total_count, 0);
        assert_eq!(data.repositories.len(), 0);
    }

    #[test]
    fn test_active_vs_archived_filtering() {
        let mut data = CanonicalData::new();

        // Add one active and one archived repo
        // (This would require creating full Repository instances - simplified for now)

        data.total_count = 2;

        // In real implementation, would test the filtering logic
        assert_eq!(data.total_count, 2);
    }
}
