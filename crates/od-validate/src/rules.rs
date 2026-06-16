//! Validation rules implementation

use super::framework::*;
use od_core::CanonicalData;
use regex::Regex;
use std::collections::HashSet;

/// Rule E001: Check for duplicate repository URLs
pub struct NoDuplicateReposRule;

impl ValidationRule for NoDuplicateReposRule {
    fn name(&self) -> &str {
        "no_duplicate_repos"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut seen_urls: HashSet<String> = HashSet::new();
        let mut issues = Vec::new();

        for repo in &data.repositories {
            for platform in &repo.platforms {
                let url = platform.url.to_lowercase();

                if seen_urls.contains(&url) {
                    issues.push(ValidationIssue {
                        code: ValidationErrorCode::DuplicateRepo,
                        severity: self.default_severity(),
                        message: format!("Duplicate repository URL: {}", platform.url),
                        location: ValidationLocation {
                            file: "repositories.yml".to_string(),
                            line: None,
                            section: Some(repo.id.clone()),
                        },
                        suggestion: "Remove duplicate or merge repository entries".to_string(),
                        auto_fixable: false,
                    });
                } else {
                    seen_urls.insert(url);
                }
            }
        }

        ValidationResult::multiple(issues)
    }
}

/// Rule E002: Check for missing license information
pub struct MissingLicenseRule {
    /// Allow repos with <10 stars to skip license requirement
    pub allow_low_star_repos: bool,
}

impl ValidationRule for MissingLicenseRule {
    fn name(&self) -> &str {
        "missing_license"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning // Changed to warning since many repos don't specify
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut issues = Vec::new();

        for repo in &data.repositories {
            if repo.metadata.license.is_none() {
                // Skip low-star repos if configured
                if self.allow_low_star_repos && repo.metadata.stars < 10 {
                    continue;
                }

                issues.push(ValidationIssue {
                    code: ValidationErrorCode::MissingLicense,
                    severity: self.default_severity(),
                    message: format!("Repository missing license: {}", repo.metadata.full_name),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: Some(repo.id.clone()),
                    },
                    suggestion: "Add license information or mark as unlicensed".to_string(),
                    auto_fixable: false,
                });
            }
        }

        ValidationResult::multiple(issues)
    }
}

/// Rule E003: Check for valid URL formats
pub struct ValidUrlsRule {
    url_regex: Regex,
}

impl ValidUrlsRule {
    pub fn new() -> Result<Self, crate::ValidateError> {
        Ok(Self {
            url_regex: Regex::new(r"^https?://[a-zA-Z0-9\-._~:/?#\[\]@!$&'()*+,;=]+$")
                .map_err(|e| crate::ValidateError::Other(e.to_string()))?,
        })
    }
}

impl ValidationRule for ValidUrlsRule {
    fn name(&self) -> &str {
        "valid_urls"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut issues = Vec::new();

        // Check repository URLs
        for repo in &data.repositories {
            for platform in &repo.platforms {
                if !self.url_regex.is_match(&platform.url) {
                    issues.push(ValidationIssue {
                        code: ValidationErrorCode::InvalidUrl,
                        severity: self.default_severity(),
                        message: format!("Invalid URL format: {}", platform.url),
                        location: ValidationLocation {
                            file: "repositories.yml".to_string(),
                            line: None,
                            section: Some(repo.id.clone()),
                        },
                        suggestion: "Fix URL format to be valid http/https URL".to_string(),
                        auto_fixable: false,
                    });
                }
            }
        }

        // Check web reference URLs
        for reference in &data.web_references {
            if !self.url_regex.is_match(&reference.url) {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::InvalidUrl,
                    severity: self.default_severity(),
                    message: format!("Invalid URL format: {}", reference.url),
                    location: ValidationLocation {
                        file: "web_references.yml".to_string(),
                        line: None,
                        section: Some(reference.id.clone()),
                    },
                    suggestion: "Fix URL format to be valid http/https URL".to_string(),
                    auto_fixable: false,
                });
            }
        }

        ValidationResult::multiple(issues)
    }
}

/// Rule E004: Check README cross-references exist
pub struct ReadmeCrossReferenceRule;

impl ValidationRule for ReadmeCrossReferenceRule {
    fn name(&self) -> &str {
        "readme_cross_reference"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut issues = Vec::new();

        // Check web references point to existing repos
        for reference in &data.web_references {
            for repo_ref in &reference.related_repos {
                // Try to find by full_name (which is what's stored in related_repos)
                let found = data
                    .repositories
                    .iter()
                    .any(|r| r.metadata.full_name == *repo_ref || r.id == *repo_ref);

                if !found {
                    issues.push(ValidationIssue {
                        code: ValidationErrorCode::BrokenCrossRef,
                        severity: self.default_severity(),
                        message: format!("Web reference links to non-existent repo: {}", repo_ref),
                        location: ValidationLocation {
                            file: "web_references.yml".to_string(),
                            line: None,
                            section: Some(reference.id.clone()),
                        },
                        suggestion: format!(
                            "Add repo {} to starred list or remove from related_repos",
                            repo_ref
                        ),
                        auto_fixable: false,
                    });
                }
            }
        }

        // Check book references point to existing repos
        for book in &data.books {
            for repo_ref in &book.related_repos {
                let found = data
                    .repositories
                    .iter()
                    .any(|r| r.metadata.full_name == *repo_ref || r.id == *repo_ref);

                if !found {
                    issues.push(ValidationIssue {
                        code: ValidationErrorCode::BrokenCrossRef,
                        severity: self.default_severity(),
                        message: format!("Book links to non-existent repo: {}", repo_ref),
                        location: ValidationLocation {
                            file: "books.yml".to_string(),
                            line: None,
                            section: Some(book.id.clone()),
                        },
                        suggestion: format!(
                            "Add repo {} to starred list or remove from related_repos",
                            repo_ref
                        ),
                        auto_fixable: false,
                    });
                }
            }
        }

        ValidationResult::multiple(issues)
    }
}

/// Rule E005: Check platform migration completeness
pub struct PlatformMigrationRule;

impl ValidationRule for PlatformMigrationRule {
    fn name(&self) -> &str {
        "platform_migration_complete"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut issues = Vec::new();

        for repo in &data.repositories {
            // Check if description mentions migration but only has one platform
            let desc_lower = repo.metadata.description.to_lowercase();
            let mentions_migration = desc_lower.contains("moved to")
                || desc_lower.contains("migrated to")
                || (desc_lower.contains("mirror") && desc_lower.contains("codeberg"));

            if mentions_migration && repo.platforms.len() == 1 {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::PlatformMismatch,
                    severity: self.default_severity(),
                    message: format!(
                        "Repository mentions migration but only has one platform: {}",
                        repo.metadata.full_name
                    ),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: Some(repo.id.clone()),
                    },
                    suggestion: "Add secondary platform URL or update description".to_string(),
                    auto_fixable: false,
                });
            }
        }

        ValidationResult::multiple(issues)
    }
}

/// Rule E006: Check for missing essential metadata
pub struct MissingMetadataRule;

impl ValidationRule for MissingMetadataRule {
    fn name(&self) -> &str {
        "missing_metadata"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut issues = Vec::new();

        for repo in &data.repositories {
            // Check for missing owner (should be split from full_name)
            if repo.metadata.owner.is_empty() {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::MissingMetadata,
                    severity: self.default_severity(),
                    message: format!("Repository missing owner: {}", repo.metadata.full_name),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: Some(repo.id.clone()),
                    },
                    suggestion: "Extract owner from full_name".to_string(),
                    auto_fixable: true,
                });
            }

            // Check for empty description
            if repo.metadata.description.trim().is_empty() {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::MissingMetadata,
                    severity: self.default_severity(),
                    message: format!(
                        "Repository has empty description: {}",
                        repo.metadata.full_name
                    ),
                    location: ValidationLocation {
                        file: "repositories.yml".to_string(),
                        line: None,
                        section: Some(repo.id.clone()),
                    },
                    suggestion: "Add description from GitHub".to_string(),
                    auto_fixable: false,
                });
            }
        }

        ValidationResult::multiple(issues)
    }
}

/// Rule for detecting stale content
pub struct StaleContentRule {
    /// Number of days before content is considered stale
    pub stale_days: i64,
}

impl ValidationRule for StaleContentRule {
    fn name(&self) -> &str {
        "stale_content"
    }

    fn default_severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let mut issues = Vec::new();

        // Check web references
        for reference in &data.web_references {
            if reference.is_stale() {
                issues.push(ValidationIssue {
                    code: ValidationErrorCode::MissingMetadata, // Using closest code
                    severity: self.default_severity(),
                    message: format!("Web reference is stale (>2 years): {}", reference.title),
                    location: ValidationLocation {
                        file: "web_references.yml".to_string(),
                        line: None,
                        section: Some(reference.id.clone()),
                    },
                    suggestion: "Review and update reference, or verify it's still relevant"
                        .to_string(),
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

    #[test]
    fn test_no_duplicate_repos_rule() {
        let rule = NoDuplicateReposRule;
        let mut data = CanonicalData::new();

        // Add two repos with same URL
        data.repositories.push(Repository {
            id: "repo1".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test/repo".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "repo".to_string(),
                owner: "test".to_string(),
                full_name: "test/repo".to_string(),
                description: "Test".to_string(),
                primary_language: "Rust".to_string(),
                license: None,
                license_spdx: None,
                stars: 10,
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
        });

        data.repositories.push(Repository {
            id: "repo2".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test/repo".to_string(), // Duplicate
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "repo".to_string(),
                owner: "test".to_string(),
                full_name: "test/repo".to_string(),
                description: "Test".to_string(),
                primary_language: "Rust".to_string(),
                license: None,
                license_spdx: None,
                stars: 10,
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
        });

        let result = rule.check(&data);
        assert!(result.has_errors());
        assert_eq!(result.issues.len(), 1);
    }

    #[test]
    fn test_valid_urls_rule() {
        let rule = ValidUrlsRule::new().unwrap();
        let mut data = CanonicalData::new();

        // Add repo with invalid URL
        data.repositories.push(Repository {
            id: "repo1".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "not-a-valid-url".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "repo".to_string(),
                owner: "test".to_string(),
                full_name: "test/repo".to_string(),
                description: "Test".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: None,
                stars: 10,
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
        });

        let result = rule.check(&data);
        assert!(result.has_errors());
    }
}
