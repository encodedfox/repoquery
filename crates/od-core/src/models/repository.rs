//! Repository data model

use super::platform::{MigrationRecord, PlatformInfo};
use super::relation::Relation;
use serde::{Deserialize, Serialize};

/// Source of repository data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositorySource {
    /// From GitHub stars
    GitHubStars,
    /// Manually added
    Manual,
    /// Derived from other sources
    Derived,
}

/// Quality metrics for a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub archive_status: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_commit_date: Option<String>,

    pub last_star_update: String,

    /// Quality score 0-100 based on stars, activity, relevance
    pub quality_score: u8,
}

/// Repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    pub name: String,
    pub owner: String,
    pub full_name: String,
    pub description: String,
    pub primary_language: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_spdx: Option<String>,

    pub stars: u32,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub topics: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// Optional language breakdown percentages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_breakdown: Option<std::collections::HashMap<String, u8>>,

    /// Languages that comprise >30% of codebase
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub secondary_languages: Vec<String>,
}

/// Repository classification and organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryClassification {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub categories: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub readme_sections: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub web_reference_topics: Vec<String>,

    /// Language category for LIST/TABLE organization
    pub language_category: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_notes: Option<String>,

    /// Whether to include in README GitHub Projects section
    #[serde(default)]
    pub readme_inclusion: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme_inclusion_reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub significance_notes: Option<String>,
}

/// Complete repository record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique identifier (e.g., "github-com-owner-repo")
    pub id: String,

    /// Platform information and migration tracking
    pub platforms: Vec<PlatformInfo>,

    /// Core metadata
    pub metadata: RepositoryMetadata,

    /// Classification and organization
    pub classification: RepositoryClassification,

    /// Quality tracking
    pub quality_metrics: QualityMetrics,

    /// Data source
    pub source: RepositorySource,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_date: Option<String>,

    #[serde(default)]
    pub manually_curated: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub curator_notes: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub relations: Vec<Relation>,

    /// For forked repos: the parent repository's full name (e.g., "owner/repo")
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fork_parent: Option<String>,

    /// For forked repos: the parent repository's URL
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fork_parent_url: Option<String>,

    /// Custom user tags for organization
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub custom_tags: Vec<String>,

    /// For forked repos: commits ahead of upstream
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fork_ahead: Option<u32>,

    /// For forked repos: commits behind upstream
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fork_behind: Option<u32>,
}

impl QualityMetrics {
    /// Calculate quality score (0-100) based on stars, activity, and metadata.
    pub fn calculate_score(
        stars: u32,
        has_license: bool,
        has_description: bool,
        has_topics: bool,
        is_archived: bool,
    ) -> u8 {
        let star_score: u8 = match stars {
            0..=10 => 0,
            11..=100 => 10,
            101..=500 => 25,
            501..=1000 => 40,
            1001..=5000 => 60,
            5001..=10000 => 75,
            _ => 90,
        };
        let mut score = star_score;
        if has_license { score = score.saturating_add(5); }
        if has_description { score = score.saturating_add(3); }
        if has_topics { score = score.saturating_add(2); }
        if is_archived { score = score.saturating_sub(20); }
        score.min(100)
    }
}

impl Repository {
    /// Check if repository meets archive criteria
    pub fn is_archive_candidate(&self) -> bool {
        self.quality_metrics.archive_status
            || (self.metadata.stars < 10 && self.is_inactive_for_years(2))
    }

    /// Check if repository has been inactive for specified years
    fn is_inactive_for_years(&self, years: i64) -> bool {
        if let Some(last_commit) = &self.quality_metrics.last_commit_date {
            if let Ok(commit_date) = chrono::NaiveDate::parse_from_str(last_commit, "%Y-%m-%d") {
                let threshold =
                    chrono::Utc::now().date_naive() - chrono::Days::new((years * 365) as u64);
                return commit_date < threshold;
            }
        }
        false
    }

    /// Check if repository meets README inclusion criteria
    pub fn meets_readme_criteria(&self) -> bool {
        self.classification.readme_inclusion
            || self.metadata.stars > 2000
            || self.classification.readme_inclusion_reason.is_some()
    }

    /// Get migration record for this repository
    pub fn migration_record(&self) -> MigrationRecord {
        MigrationRecord {
            repo_id: self.id.clone(),
            migration_status: MigrationRecord::detect_status(&self.platforms),
            platforms: self.platforms.clone(),
            migration_history: vec![], // TODO: Track from metadata
        }
    }

    /// Get primary platform URL
    pub fn primary_url(&self) -> Option<&str> {
        self.platforms
            .iter()
            .find(|p| p.is_primary)
            .map(|p| p.url.as_str())
    }

    /// Check if repository has secondary languages
    pub fn has_secondary_languages(&self) -> bool {
        !self.metadata.secondary_languages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_candidate_by_status() {
        let repo = Repository {
            id: "test-repo".to_string(),
            platforms: vec![],
            metadata: RepositoryMetadata {
                name: "test".to_string(),
                owner: "owner".to_string(),
                full_name: "owner/test".to_string(),
                description: "Test".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: Some("MIT".to_string()),
                stars: 100,
                topics: vec![],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: "Rust".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: true,
                archive_date: Some("2024-01-01".to_string()),
                last_commit_date: None,
                last_star_update: "2024-12-10".to_string(),
                quality_score: 50,
            },
            source: RepositorySource::GitHubStars,
            added_date: None,
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
            fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        };

        assert!(repo.is_archive_candidate());
    }

    #[test]
    fn test_archive_candidate_by_threshold() {
        let repo = Repository {
            id: "test-repo".to_string(),
            platforms: vec![],
            metadata: RepositoryMetadata {
                name: "test".to_string(),
                owner: "owner".to_string(),
                full_name: "owner/test".to_string(),
                description: "Test".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: None,
                stars: 5, // Below threshold
                topics: vec![],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: "Rust".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: Some("2020-01-01".to_string()), // >2 years ago
                last_star_update: "2024-12-10".to_string(),
                quality_score: 30,
            },
            source: RepositorySource::GitHubStars,
            added_date: None,
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
            fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        };

        assert!(repo.is_archive_candidate());
    }

    #[test]
    fn test_meets_readme_criteria_by_stars() {
        let repo = Repository {
            id: "test-repo".to_string(),
            platforms: vec![],
            metadata: RepositoryMetadata {
                name: "test".to_string(),
                owner: "owner".to_string(),
                full_name: "owner/test".to_string(),
                description: "Test".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: None,
                stars: 3000, // Above 2000 threshold
                topics: vec![],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: "Rust".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: None,
                last_star_update: "2024-12-10".to_string(),
                quality_score: 85,
            },
            source: RepositorySource::GitHubStars,
            added_date: None,
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
            fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        };

        assert!(repo.meets_readme_criteria());
    }

    #[test]
    fn test_relations_default_on_deserialize() {
        // YAML without `relations` field must still deserialize (backward compat)
        let yaml = r#"
id: test-repo
platforms: []
metadata:
  name: test
  owner: owner
  full_name: owner/test
  description: Test
  primary_language: Rust
  stars: 100
  language_category: Rust
classification:
  language_category: Rust
quality_metrics:
  archive_status: false
  last_star_update: "2024-01-01"
  quality_score: 50
source: git_hub_stars
manually_curated: false
"#;
        let repo: Repository = serde_yml::from_str(yaml).expect("should deserialize without relations");
        assert!(repo.relations.is_empty());
    }

    #[test]
    fn test_new_fields_default_on_deserialize() {
        // YAML without custom_tags/fork_ahead/fork_behind must still deserialize
        let yaml = r#"
id: test-repo
platforms: []
metadata:
  name: test
  owner: owner
  full_name: owner/test
  description: Test
  primary_language: Rust
  stars: 100
  language_category: Rust
classification:
  language_category: Rust
quality_metrics:
  archive_status: false
  last_star_update: "2024-01-01"
  quality_score: 50
source: git_hub_stars
manually_curated: false
"#;
        let repo: Repository = serde_yml::from_str(yaml).expect("backward compat failed");
        assert!(repo.custom_tags.is_empty());
        assert!(repo.fork_ahead.is_none());
        assert!(repo.fork_behind.is_none());
    }

    #[test]
    fn test_custom_tags_add_remove() {
        let mut tags: Vec<String> = vec![];
        let tag = "ml".to_string();
        if !tags.contains(&tag) { tags.push(tag.clone()); }
        assert_eq!(tags, vec!["ml"]);
        // duplicate ignored
        if !tags.contains(&tag) { tags.push(tag.clone()); }
        assert_eq!(tags.len(), 1);
        tags.retain(|t| t != &tag);
        assert!(tags.is_empty());
    }
}
