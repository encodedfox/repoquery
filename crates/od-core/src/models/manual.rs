//! Manual project additions model

use super::platform::PlatformInfo;
use serde::{Deserialize, Serialize};

/// Manually added project (not from GitHub stars)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualProject {
    pub id: String,
    pub name: String,
    pub description: String,
    pub platforms: Vec<PlatformInfo>,

    pub metadata: ManualProjectMetadata,
    pub classification: ManualProjectClassification,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub curator_notes: Option<String>,
}

/// Metadata for manually added projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualProjectMetadata {
    pub primary_language: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stars: Option<u32>,
}

/// Classification for manually added projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualProjectClassification {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub categories: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub readme_sections: Vec<String>,
}

impl ManualProject {
    /// Convert ManualProject to Repository for unified processing
    pub fn to_repository(&self) -> super::repository::Repository {
        super::repository::Repository {
            id: self.id.clone(),
            platforms: self.platforms.clone(),
            metadata: super::repository::RepositoryMetadata {
                name: self.name.clone(),
                owner: "".to_string(), // May not have owner for manual additions
                full_name: self.name.clone(),
                description: self.description.clone(),
                primary_language: self.metadata.primary_language.clone(),
                license: self.metadata.license.clone(),
                license_spdx: None,
                stars: self.metadata.stars.unwrap_or(0),
                topics: vec![],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: super::repository::RepositoryClassification {
                categories: self.classification.categories.clone(),
                readme_sections: self.classification.readme_sections.clone(),
                web_reference_topics: vec![],
                language_category: self.metadata.primary_language.clone(),
                language_notes: None,
                readme_inclusion: true, // Manual additions are significant
                readme_inclusion_reason: Some("manual_curation".to_string()),
                significance_notes: self.curator_notes.clone(),
            },
            quality_metrics: super::repository::QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: None,
                last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                quality_score: 75, // Default for manual additions
            },
            source: super::repository::RepositorySource::Manual,
            added_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            manually_curated: true,
            curator_notes: self.curator_notes.clone(),
            relations: vec![],
            fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::platform::{Platform, PlatformStatus};

    #[test]
    fn test_manual_project_to_repository() {
        let manual = ManualProject {
            id: "manual-test".to_string(),
            name: "TestProject".to_string(),
            description: "Test project".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test/project".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: ManualProjectMetadata {
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                stars: Some(100),
            },
            classification: ManualProjectClassification {
                categories: vec!["tools".to_string()],
                readme_sections: vec!["GitHub Projects".to_string()],
            },
            curator_notes: Some("Important tool".to_string()),
        };

        let repo = manual.to_repository();

        assert_eq!(repo.id, "manual-test");
        assert_eq!(
            repo.source,
            super::super::repository::RepositorySource::Manual
        );
        assert!(repo.manually_curated);
        assert!(repo.classification.readme_inclusion);
    }
}
