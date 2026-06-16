//! Validation rules for external data consistency

use super::framework::*;
use od_core::{CanonicalData, PlatformStatus};
use std::collections::HashMap;

/// Rule E007: External data consistency checks
///
/// Validates data integrity after external sync operations:
/// - Star counts are reasonable (not > 1M)
/// - Archive status matches platform status
/// - Popular repos have descriptions
/// - Required fields present after sync
pub struct ExternalDataConsistencyRule;

impl ValidationRule for ExternalDataConsistencyRule {
    fn name(&self) -> &str {
        "external_data_consistency"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut issues = Vec::new();

        for repo in &data.repositories {
            // Check for unrealistic star counts
            if repo.metadata.stars > 1_000_000 {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::ExternalDataInconsistent,
                    severity: Severity::Warning,
                    message: format!(
                        "Unusually high star count: {} has {} stars (verify with source)",
                        repo.metadata.full_name, repo.metadata.stars
                    ),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: Some(repo.id.clone()),
                    },
                    suggestion: "Verify star count with GitHub or external source".to_string(),
                    auto_fixable: false,
                });
            }

            // Check archive status consistency
            if repo.quality_metrics.archive_status {
                let has_active_platform = repo
                    .platforms
                    .iter()
                    .any(|p| p.status == PlatformStatus::Active);

                if has_active_platform {
                    issues.push(ValidationIssue {
                        code: ValidationErrorCode::ExternalDataInconsistent,
                        severity: self.default_severity(),
                        message: format!(
                            "Repository marked as archived but has active platform status: {}",
                            repo.metadata.full_name
                        ),
                        location: ValidationLocation {
                            file: "repositories.yml".to_string(),
                            line: None,
                            section: Some(repo.id.clone()),
                        },
                        suggestion:
                            "Update platform status to Archived to match archive_status = true"
                                .to_string(),
                        auto_fixable: true,
                    });
                }
            }

            // Check for empty critical fields in popular repos
            if repo.metadata.description.trim().is_empty() && repo.metadata.stars > 100 {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::ExternalDataInconsistent,
                    severity: Severity::Warning,
                    message: format!(
                        "Popular repository ({} stars) missing description: {}",
                        repo.metadata.stars, repo.metadata.full_name
                    ),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: Some(repo.id.clone()),
                    },
                    suggestion: "Re-sync repository or add description manually".to_string(),
                    auto_fixable: false,
                });
            }

            // Check for missing owner in repos with stars
            if repo.metadata.owner.trim().is_empty() && repo.metadata.stars > 0 {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::ExternalDataInconsistent,
                    severity: Severity::Warning,
                    message: format!(
                        "Repository missing owner field: {}",
                        repo.metadata.full_name
                    ),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: Some(repo.id.clone()),
                    },
                    suggestion: "Extract owner from full_name or re-sync".to_string(),
                    auto_fixable: true,
                });
            }
        }

        ValidationResult::multiple(issues)
    }
}

/// Rule E008: Duplicate repository names
///
/// Detects when multiple repository entries have the same full_name,
/// which could indicate:
/// - Duplicate entries that should be merged
/// - Same repo on different platforms (should be one entry with multiple platforms)
/// - Data quality issues from sync
pub struct DuplicateRepositoryNameRule;

impl ValidationRule for DuplicateRepositoryNameRule {
    fn name(&self) -> &str {
        "duplicate_repository_names"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut seen_names: HashMap<String, Vec<String>> = HashMap::new();
        let mut issues = Vec::new();

        // Build map of full_name -> repo IDs
        for repo in &data.repositories {
            let full_name = repo.metadata.full_name.to_lowercase();
            seen_names
                .entry(full_name.clone())
                .or_default()
                .push(repo.id.clone());
        }

        // Check for duplicates
        for (full_name, ids) in seen_names {
            if ids.len() > 1 {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::DuplicateRepositoryName,
                    severity: self.default_severity(),
                    message: format!(
                        "Duplicate repository name '{}' found in {} entries: {:?}",
                        full_name,
                        ids.len(),
                        ids
                    ),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: None,
                    },
                    suggestion: format!(
                        "Merge duplicate entries or differentiate by platform. Found in: {}",
                        ids.join(", ")
                    ),
                    auto_fixable: false,
                });
            }
        }

        ValidationResult::multiple(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::*;

    fn create_test_repo(full_name: &str, stars: u32, archived: bool) -> Repository {
        let parts: Vec<&str> = full_name.split('/').collect();
        Repository {
            id: format!("test-{}", full_name.replace('/', "-")),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: format!("https://github.com/{}", full_name),
                status: if archived {
                    PlatformStatus::Archived
                } else {
                    PlatformStatus::Active
                },
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: parts[1].to_string(),
                owner: parts[0].to_string(),
                full_name: full_name.to_string(),
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
                archive_status: archived,
                archive_date: if archived {
                    Some("2024-01-01".to_string())
                } else {
                    None
                },
                last_commit_date: None,
                last_star_update: "2024-12-10".to_string(),
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

    #[test]
    fn test_external_data_consistency_unrealistic_stars() {
        let rule = ExternalDataConsistencyRule;
        let mut data = CanonicalData::new();

        // Add repo with >1M stars
        let repo = create_test_repo("test/repo", 1_500_000, false);
        data.repositories.push(repo);

        let result = rule.check(&data);
        assert!(!result.issues.is_empty());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("Unusually high star count")));
    }

    #[test]
    fn test_external_data_consistency_archive_mismatch() {
        let rule = ExternalDataConsistencyRule;
        let mut data = CanonicalData::new();

        // Repo marked archived but platform status is active
        let mut repo = create_test_repo("test/repo", 100, false);
        repo.quality_metrics.archive_status = true; // Marked archived
        repo.platforms[0].status = PlatformStatus::Active; // But platform active - INCONSISTENT

        data.repositories.push(repo);

        let result = rule.check(&data);
        assert!(result.has_errors());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("archived but has active platform")));
    }

    #[test]
    fn test_external_data_consistency_missing_description() {
        let rule = ExternalDataConsistencyRule;
        let mut data = CanonicalData::new();

        let mut repo = create_test_repo("test/repo", 500, false);
        repo.metadata.description = "".to_string(); // Empty description

        data.repositories.push(repo);

        let result = rule.check(&data);
        assert!(!result.issues.is_empty());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("missing description")));
    }

    #[test]
    fn test_duplicate_repository_name_detection() {
        let rule = DuplicateRepositoryNameRule;
        let mut data = CanonicalData::new();

        // Add same repo twice with different IDs
        let repo1 = create_test_repo("test/repo", 100, false);
        let mut repo2 = create_test_repo("test/repo", 200, false);
        repo2.id = "different-id".to_string();

        data.repositories.push(repo1);
        data.repositories.push(repo2);

        let result = rule.check(&data);
        assert!(result.has_errors());
        assert!(result
            .issues
            .iter()
            .any(|i| i.message.contains("Duplicate repository name")));
    }

    #[test]
    fn test_duplicate_repository_name_no_duplicates() {
        let rule = DuplicateRepositoryNameRule;
        let mut data = CanonicalData::new();

        data.repositories.push(create_test_repo("test/repo1", 100, false));
        data.repositories.push(create_test_repo("test/repo2", 200, false));

        let result = rule.check(&data);
        assert!(result.issues.is_empty());
    }
}