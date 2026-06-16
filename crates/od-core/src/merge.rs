//! Merge logic for combining multiple data sources

use crate::models::*;
use std::collections::HashMap;

/// Merge strategy for handling conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Prefer manual additions over starred data
    PreferManual,
    /// Prefer starred data over manual
    PreferStarred,
    /// Fail on conflicts
    Strict,
}

/// Merger for combining data sources
pub struct DataMerger {
    strategy: MergeStrategy,
}

impl DataMerger {
    /// Create new merger with strategy
    pub fn new(strategy: MergeStrategy) -> Self {
        Self { strategy }
    }

    /// Merge base canonical data with manual additions, web refs, and books
    pub fn merge(
        &self,
        base: CanonicalData,
        manual: Option<CanonicalData>,
        web_refs: Option<Vec<WebReference>>,
        books: Option<Vec<Book>>,
    ) -> crate::Result<CanonicalData> {
        let mut merged = base;

        // Merge manual projects
        if let Some(manual_data) = manual {
            tracing::info!(
                "Merging {} manual projects",
                manual_data.manual_projects.len()
            );
            self.merge_manual_projects(&mut merged, manual_data.manual_projects)?;
        }

        // Add web references
        if let Some(refs) = web_refs {
            tracing::info!("Adding {} web references", refs.len());
            merged.web_references = refs;
        }

        // Add books
        if let Some(book_list) = books {
            tracing::info!("Adding {} books", book_list.len());
            merged.books = book_list;
        }

        // Recalculate statistics
        merged.total_count = merged.repositories.len() + merged.manual_projects.len();
        merged.calculate_statistics();
        merged.last_updated = chrono::Utc::now().to_rfc3339();

        tracing::info!("Merge complete: {} total repositories", merged.total_count);

        Ok(merged)
    }

    /// Merge manual projects into repository list
    fn merge_manual_projects(
        &self,
        data: &mut CanonicalData,
        manual_projects: Vec<ManualProject>,
    ) -> crate::Result<()> {
        // Build index of existing repos by URL for conflict detection
        let mut existing_urls: HashMap<String, usize> = HashMap::new();
        for (idx, repo) in data.repositories.iter().enumerate() {
            for platform in &repo.platforms {
                existing_urls.insert(platform.url.to_lowercase(), idx);
            }
        }

        for manual in manual_projects {
            // Check if this project already exists in starred repos
            let mut conflict_idx = None;
            for platform in &manual.platforms {
                if let Some(&idx) = existing_urls.get(&platform.url.to_lowercase()) {
                    conflict_idx = Some(idx);
                    break;
                }
            }

            match conflict_idx {
                Some(idx) => {
                    // Conflict: manual project matches existing starred repo
                    match self.strategy {
                        MergeStrategy::PreferManual => {
                            tracing::warn!(
                                "Conflict: {} exists in both sources, preferring manual version",
                                manual.name
                            );
                            // Replace starred version with manual
                            let manual_repo = manual.to_repository();
                            data.repositories[idx] = manual_repo;
                        }
                        MergeStrategy::PreferStarred => {
                            tracing::warn!(
                                "Conflict: {} exists in both sources, keeping starred version",
                                manual.name
                            );
                            // Keep starred version, discard manual
                        }
                        MergeStrategy::Strict => {
                            return Err(crate::CoreError::Config(format!(
                                "Conflict: {} exists in both starred repos and manual additions",
                                manual.name
                            )));
                        }
                    }
                }
                None => {
                    // No conflict: add manual project to the list
                    tracing::info!("Adding manual project: {}", manual.name);
                    data.manual_projects.push(manual);
                }
            }
        }

        Ok(())
    }

    /// Enrich repository data with additional metadata
    pub fn enrich_repositories(&self, data: &mut CanonicalData) -> crate::Result<()> {
        tracing::info!("Enriching {} repositories", data.repositories.len());

        // Could add enrichment logic here:
        // - Fetch additional data from GitHub API
        // - Enhance descriptions
        // - Add topics/tags
        // - Update star counts
        // - Detect additional migrations

        Ok(())
    }

    /// Detect platform migrations from descriptions
    pub fn detect_migrations(&self, data: &mut CanonicalData) -> crate::Result<usize> {
        let mut migration_count = 0;

        for repo in &mut data.repositories {
            let desc_lower = repo.metadata.description.to_lowercase();

            // Already has multiple platforms
            if repo.platforms.len() > 1 {
                continue;
            }

            // Check for migration keywords
            if desc_lower.contains("moved to codeberg") {
                // Note: Actual Codeberg URL would need to be researched
                // For now, flag it in curator notes
                repo.curator_notes =
                    Some("Migration to Codeberg mentioned - URL needs verification".to_string());
                migration_count += 1;
            } else if desc_lower.contains("moved to gitlab") {
                repo.curator_notes =
                    Some("Migration to GitLab mentioned - URL needs verification".to_string());
                migration_count += 1;
            }
        }

        tracing::info!("Detected {} potential migrations", migration_count);
        Ok(migration_count)
    }

    /// Calculate quality scores for all repositories
    pub fn calculate_quality_scores(&self, data: &mut CanonicalData) -> crate::Result<()> {
        for repo in &mut data.repositories {
            repo.quality_metrics.quality_score = QualityMetrics::calculate_score(
                repo.metadata.stars,
                repo.metadata.license.is_some(),
                !repo.metadata.description.is_empty(),
                !repo.metadata.topics.is_empty(),
                repo.quality_metrics.archive_status || repo.is_archive_candidate(),
            );
        }
        Ok(())
    }
}

impl Default for DataMerger {
    fn default() -> Self {
        Self::new(MergeStrategy::PreferManual)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_no_conflicts() {
        let merger = DataMerger::new(MergeStrategy::PreferManual);

        let mut base = CanonicalData::new();
        base.repositories.push(create_test_repo("repo1", 100));

        let mut manual = CanonicalData::new();
        manual
            .manual_projects
            .push(create_test_manual_project("repo2"));

        let result = merger.merge(base, Some(manual), None, None).unwrap();

        assert_eq!(result.repositories.len(), 1);
        assert_eq!(result.manual_projects.len(), 1);
        assert_eq!(result.total_count, 2);
    }

    #[test]
    fn test_merge_prefer_manual() {
        let merger = DataMerger::new(MergeStrategy::PreferManual);

        let mut base = CanonicalData::new();
        base.repositories.push(create_test_repo("test-repo", 100));

        let mut manual = CanonicalData::new();
        manual
            .manual_projects
            .push(create_conflicting_manual_project());

        let result = merger.merge(base, Some(manual), None, None).unwrap();

        // Manual version should replace starred version
        assert_eq!(result.repositories.len(), 1);
        assert!(result.repositories[0].manually_curated);
    }

    // Helper functions for tests
    fn create_test_repo(name: &str, stars: u32) -> Repository {
        Repository {
            id: format!("test-{}", name),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: format!("https://github.com/test/{}", name),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: name.to_string(),
                owner: "test".to_string(),
                full_name: format!("test/{}", name),
                description: "Test repo".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: None,
                stars,
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
                last_star_update: "2025-12-10".to_string(),
                quality_score: 70,
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
        }
    }

    fn create_test_manual_project(name: &str) -> ManualProject {
        ManualProject {
            id: format!("manual-{}", name),
            name: name.to_string(),
            description: "Manual project".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: format!("https://github.com/manual/{}", name),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: ManualProjectMetadata {
                primary_language: "Go".to_string(),
                license: Some("Apache 2.0".to_string()),
                stars: Some(500),
            },
            classification: ManualProjectClassification {
                categories: vec!["manual".to_string()],
                readme_sections: vec!["GitHub Projects".to_string()],
            },
            curator_notes: Some("Manually added".to_string()),
        }
    }

    fn create_conflicting_manual_project() -> ManualProject {
        ManualProject {
            id: "manual-conflict".to_string(),
            name: "test-repo".to_string(),
            description: "Manual version".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test/test-repo".to_string(), // Same URL as test repo
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: ManualProjectMetadata {
                primary_language: "Go".to_string(),
                license: Some("Apache 2.0".to_string()),
                stars: Some(200),
            },
            classification: ManualProjectClassification {
                categories: vec!["manual".to_string()],
                readme_sections: vec!["GitHub Projects".to_string()],
            },
            curator_notes: Some("Manual override".to_string()),
        }
    }
}
