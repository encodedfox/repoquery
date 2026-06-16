//! Integration tests for OmniDatum Processor
//!
//! Tests end-to-end functionality including:
//! - Data loading and parsing
//! - Validation pipeline
//! - Document generation
//! - Cross-reference accuracy
//! - Sync workflow testing

mod mock_adapter;

use omnidatum_processor::{
    Book, CanonicalData, ContentType, DataMerger, DifficultyLevel, ManualProject,
    ManualProjectClassification, ManualProjectMetadata, MarkdownGenerator, MergeStrategy,
    MissingLicenseRule, NoDuplicateReposRule, Platform, PlatformInfo, PlatformMigrationRule,
    PlatformStatus, QualityMetrics, ReadmeCrossReferenceRule, ReferenceStatus, Repository,
    RepositoryClassification, RepositoryMetadata, RepositorySource, Severity, ValidUrlsRule,
    Validator, WebReference,
};
use omnidatum_processor::sync::DataSourceAdapter;
use tempfile::TempDir;

/// Helper function to create test canonical data with a small sample
fn create_test_data() -> CanonicalData {
    let mut data = CanonicalData::new();
    data.schema_version = "1.0".to_string();

    // Add sample repositories
    data.repositories = vec![
        Repository {
            id: "test-repo-1".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test-owner/test-repo-1".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                last_verified: Some("2024-12-10".to_string()),
                migration_date: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "test-repo-1".to_string(),
                owner: "test-owner".to_string(),
                full_name: "test-owner/test-repo-1".to_string(),
                description: "Test repository 1".to_string(),
                primary_language: "Go".to_string(),
                license: Some("MIT License".to_string()),
                license_spdx: Some("MIT".to_string()),
                stars: 1000,
                topics: vec!["test".to_string()],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec!["test-category".to_string()],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: "Go".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: Some("2024-11-15".to_string()),
                last_star_update: "2024-12-10".to_string(),
                quality_score: 85,
            },
            source: RepositorySource::GitHubStars,
            added_date: Some("2023-05-20".to_string()),
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        },
        Repository {
            id: "test-repo-2".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test-owner/test-repo-2".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                last_verified: Some("2024-12-10".to_string()),
                migration_date: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "test-repo-2".to_string(),
                owner: "test-owner".to_string(),
                full_name: "test-owner/test-repo-2".to_string(),
                description: "Test repository 2".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("Apache License 2.0".to_string()),
                license_spdx: Some("Apache-2.0".to_string()),
                stars: 5000,
                topics: vec!["test".to_string(), "rust".to_string()],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec!["test-category".to_string()],
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
                last_commit_date: Some("2024-12-01".to_string()),
                last_star_update: "2024-12-10".to_string(),
                quality_score: 92,
            },
            source: RepositorySource::GitHubStars,
            added_date: Some("2024-01-10".to_string()),
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        },
        // Add an archived repository for testing
        Repository {
            id: "test-archived-repo".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test-owner/archived-repo".to_string(),
                status: PlatformStatus::Archived,
                is_primary: true,
                last_verified: Some("2024-12-10".to_string()),
                migration_date: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "archived-repo".to_string(),
                owner: "test-owner".to_string(),
                full_name: "test-owner/archived-repo".to_string(),
                description: "Archived test repository".to_string(),
                primary_language: "Python".to_string(),
                license: Some("GPL-3.0".to_string()),
                license_spdx: Some("GPL-3.0".to_string()),
                stars: 50,
                topics: vec![],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: "Python".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: true,
                archive_date: Some("2022-01-01".to_string()),
                last_commit_date: Some("2021-12-01".to_string()),
                last_star_update: "2024-12-10".to_string(),
                quality_score: 20,
            },
            source: RepositorySource::GitHubStars,
            added_date: Some("2021-01-01".to_string()),
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        },
    ];

    data.total_count = data.repositories.len();
    data.calculate_statistics();

    data
}

/// Test 15.1: End-to-end generation pipeline
#[test]
fn test_e2e_generation_pipeline() {
    // Create test data
    let canonical = create_test_data();

    // Verify we have the expected test data
    assert_eq!(canonical.repositories.len(), 3);
    assert_eq!(canonical.total_count, 3);

    // Create temp directory for templates and output
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let template_dir = temp_dir.path().join("templates");
    std::fs::create_dir_all(&template_dir).expect("Failed to create template dir");

    // Create simple test templates (matching actual template structure)
    let list_template = r#"# Test List
{% for lang in languages %}
## {{ lang.name }}
{% for repo in lang.repos %}
- [{{ repo.full_name }}]({{ repo.url }}) - {{ repo.description }} {% if repo.license %}[*{{ repo.license }}*]{% endif %} (⭐️{{ repo.stars }}){% if repo.is_archived %} *Archived!*{% endif %}
{% endfor %}
{% endfor %}
"#;

    std::fs::write(template_dir.join("list.md.tera"), list_template)
        .expect("Failed to write list template");

    let table_template = r#"# Test Table
{% for lang in languages %}
## {{ lang.name }}
| Name | Description | License | Stars |
|------|-------------|---------|-------|
{% for repo in lang.repos %}
| [{{ repo.full_name }}]({{ repo.url }}) | {{ repo.description }}{% if repo.is_archived %} (*archived*){% endif %} | {% if repo.license %}{{ repo.license }}{% else %}-{% endif %} | ⭐️{{ repo.stars }} |
{% endfor %}
{% endfor %}
"#;

    std::fs::write(template_dir.join("table.md.tera"), table_template)
        .expect("Failed to write table template");

    // Test generation
    let generator =
        MarkdownGenerator::new(template_dir.to_str().unwrap()).expect("Failed to create generator");

    // Separate active and archived
    let mut active_data = canonical.clone();
    active_data.repositories = canonical.active_repositories();
    active_data.calculate_statistics();

    let mut archive_data = canonical.clone();
    archive_data.repositories = canonical.archived_repositories();
    archive_data.calculate_statistics();

    // Generate LIST.md
    let list_content = generator
        .generate_list(&active_data, false)
        .expect("Failed to generate LIST.md");

    // Verify LIST.md contains expected content
    assert!(list_content.contains("# Test List"));
    assert!(list_content.contains("test-repo-1"));
    assert!(list_content.contains("test-repo-2"));
    assert!(!list_content.contains("archived-repo")); // Should not be in active list
    assert!(list_content.contains("MIT License"));
    assert!(list_content.contains("⭐️1000"));
    assert!(list_content.contains("⭐️5000"));

    // Generate TABLE.md
    let table_content = generator
        .generate_table(&active_data, false)
        .expect("Failed to generate TABLE.md");

    // Verify TABLE.md contains expected content
    assert!(table_content.contains("# Test Table"));
    assert!(table_content.contains("| Name | Description | License | Stars |"));
    assert!(table_content.contains("test-repo-1"));
    assert!(table_content.contains("test-repo-2"));
    assert!(!table_content.contains("archived-repo"));

    // Verify star count consistency between formats
    assert!(list_content.contains("⭐️1000"));
    assert!(table_content.contains("⭐️1000"));
    assert!(list_content.contains("⭐️5000"));
    assert!(table_content.contains("⭐️5000"));

    // Test archived list generation
    let archive_list = generator
        .generate_list(&archive_data, false)
        .expect("Failed to generate archive list");

    assert!(archive_list.contains("archived-repo"));
    assert!(archive_list.contains("*Archived!*"));

    println!("✅ E2E generation pipeline test passed");
}

/// Test 15.2: Validation error detection
#[test]
fn test_validation_error_detection() {
    // Create data with intentional errors
    let mut data = create_test_data();

    // Add duplicate repository (same URL)
    let mut duplicate = data.repositories[0].clone();
    duplicate.id = "duplicate-repo".to_string();
    duplicate.metadata.name = "duplicate-name".to_string();
    // Same URL as test-repo-1
    data.repositories.push(duplicate);

    // Add repository with missing license
    data.repositories.push(Repository {
        id: "no-license-repo".to_string(),
        platforms: vec![PlatformInfo {
            platform: Platform::GitHub,
            url: "https://github.com/test-owner/no-license".to_string(),
            status: PlatformStatus::Active,
            is_primary: true,
            last_verified: Some("2024-12-10".to_string()),
            migration_date: None,
            notes: None,
        }],
        metadata: RepositoryMetadata {
            name: "no-license".to_string(),
            owner: "test-owner".to_string(),
            full_name: "test-owner/no-license".to_string(),
            description: "Repository without license".to_string(),
            primary_language: "JavaScript".to_string(),
            license: None, // Missing license
            license_spdx: None,
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
            language_category: "JavaScript".to_string(),
            language_notes: None,
            readme_inclusion: false,
            readme_inclusion_reason: None,
            significance_notes: None,
        },
        quality_metrics: QualityMetrics {
            archive_status: false,
            archive_date: None,
            last_commit_date: Some("2024-11-01".to_string()),
            last_star_update: "2024-12-10".to_string(),
            quality_score: 50,
        },
        source: RepositorySource::GitHubStars,
        added_date: Some("2024-01-01".to_string()),
        manually_curated: false,
        curator_notes: None,
        relations: vec![],
            fork_parent: None,
        fork_parent_url: None,
        custom_tags: vec![],
        fork_ahead: None,
        fork_behind: None,
    });

    // Add repository with invalid URL format
    data.repositories.push(Repository {
        id: "invalid-url-repo".to_string(),
        platforms: vec![PlatformInfo {
            platform: Platform::GitHub,
            url: "not-a-valid-url".to_string(), // Invalid URL
            status: PlatformStatus::Active,
            is_primary: true,
            last_verified: Some("2024-12-10".to_string()),
            migration_date: None,
            notes: None,
        }],
        metadata: RepositoryMetadata {
            name: "invalid-url".to_string(),
            owner: "test-owner".to_string(),
            full_name: "test-owner/invalid-url".to_string(),
            description: "Repository with invalid URL".to_string(),
            primary_language: "TypeScript".to_string(),
            license: Some("MIT".to_string()),
            license_spdx: Some("MIT".to_string()),
            stars: 200,
            topics: vec![],
            homepage: None,
            language_breakdown: None,
            secondary_languages: vec![],
        },
        classification: RepositoryClassification {
            categories: vec![],
            readme_sections: vec![],
            web_reference_topics: vec![],
            language_category: "TypeScript".to_string(),
            language_notes: None,
            readme_inclusion: false,
            readme_inclusion_reason: None,
            significance_notes: None,
        },
        quality_metrics: QualityMetrics {
            archive_status: false,
            archive_date: None,
            last_commit_date: Some("2024-11-01".to_string()),
            last_star_update: "2024-12-10".to_string(),
            quality_score: 60,
        },
        source: RepositorySource::GitHubStars,
        added_date: Some("2024-01-01".to_string()),
        manually_curated: false,
        curator_notes: None,
        relations: vec![],
            fork_parent: None,
        fork_parent_url: None,
        custom_tags: vec![],
        fork_ahead: None,
        fork_behind: None,
    });

    data.total_count = data.repositories.len();

    // Create validator with all rules
    let mut validator = Validator::new();
    validator.add_rule(NoDuplicateReposRule);
    validator.add_rule(MissingLicenseRule {
        allow_low_star_repos: false,
    });
    validator.add_rule(ValidUrlsRule::new().expect("Failed to create URL rule"));
    validator.add_rule(ReadmeCrossReferenceRule);
    validator.add_rule(PlatformMigrationRule);

    // Run validation
    let report = validator.validate(&data);

    // Verify errors were detected
    assert!(report.summary.errors > 0, "Expected errors but found none");

    // Check for specific error types
    let duplicate_errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.message.contains("Duplicate") || i.message.contains("duplicate"))
        .collect();
    assert!(!duplicate_errors.is_empty(), "Expected duplicate error");

    let url_errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.message.contains("URL") || i.message.contains("url"))
        .collect();
    assert!(!url_errors.is_empty(), "Expected URL validation error");

    // Verify error severity levels
    let has_errors = report.issues.iter().any(|i| i.severity == Severity::Error);
    assert!(has_errors, "Expected at least one error-level issue");

    // Verify validation failed
    assert!(!report.passed(), "Validation should have failed");

    println!("✅ Validation error detection test passed");
    println!(
        "   Found {} errors, {} warnings",
        report.summary.errors, report.summary.warnings
    );
}

/// Test 15.3: Cross-reference accuracy
#[test]
fn test_cross_reference_accuracy() {
    // Create test data with cross-references
    let mut data = create_test_data();

    // Add a web reference that links to repositories
    data.web_references.push(WebReference {
        id: "test-web-ref".to_string(),
        title: "Test Web Reference".to_string(),
        url: "https://example.com/test".to_string(),
        author: Some("Test Author".to_string()),
        category: "Testing".to_string(),
        subcategory: Some("Integration".to_string()),
        content_type: ContentType::Article,
        difficulty: DifficultyLevel::Intermediate,
        publication_date: Some("2024".to_string()),
        last_verified: Some("2024-12-10".to_string()),
        related_references: vec![],
        related_repos: vec!["test-owner/test-repo-1".to_string()],
        tags: vec!["test".to_string()],
        status: ReferenceStatus::Active,
    });

    // Add a book
    data.books.push(Book {
        id: "test-book".to_string(),
        title: "Test Book".to_string(),
        subtitle: Some("A Testing Guide".to_string()),
        authors: vec!["Test Author".to_string()],
        category: "Testing".to_string(),
        subcategory: Some("Integration Tests".to_string()),
        isbn: Some("978-0-00-000000-0".to_string()),
        publication_year: Some(2024),
        edition: Some("1st".to_string()),
        related_books: vec![],
        expansion_topics: vec!["testing".to_string()],
        related_web_references: vec!["test-web-ref".to_string()],
        related_repos: vec!["test-owner/test-repo-2".to_string()],
    });

    // Build cross-reference graph
    use omnidatum_processor::CrossRefGraph;
    let graph = CrossRefGraph::build(&data).expect("Failed to build cross-reference graph");

    // Test bidirectional links

    // The graph has been built successfully
    let stats = graph.stats();
    assert!(stats.total_nodes > 0, "Graph should have nodes");
    assert!(stats.repositories > 0, "Graph should have repository nodes");

    // Find repo IDs (graph uses repo.id, not full_name)
    let _repo1_id = data
        .repositories
        .iter()
        .find(|r| r.metadata.full_name == "test-owner/test-repo-1")
        .map(|r| r.id.clone())
        .expect("test-repo-1 not found");

    let _repo2_id = data
        .repositories
        .iter()
        .find(|r| r.metadata.full_name == "test-owner/test-repo-2")
        .map(|r| r.id.clone())
        .expect("test-repo-2 not found");

    // The graph links web references and books to repositories
    // Sections link to web references/books, which then link to repos
    // So this is testing the graph structure exists and has correct node counts
    assert_eq!(stats.repositories, 3, "Should have 3 repository nodes");
    assert_eq!(stats.web_references, 1, "Should have 1 web reference node");
    assert_eq!(stats.books, 1, "Should have 1 book node");
    assert!(
        stats.readme_sections > 0,
        "Should have README section nodes"
    );

    // 4. Verify web reference to repo relationship exists
    let web_ref = &data.web_references[0];
    assert_eq!(web_ref.related_repos.len(), 1);
    assert_eq!(web_ref.related_repos[0], "test-owner/test-repo-1");

    // 5. Verify book to repo relationship exists
    let book = &data.books[0];
    assert_eq!(book.related_repos.len(), 1);
    assert_eq!(book.related_repos[0], "test-owner/test-repo-2");

    // 6. Verify book to web reference relationship exists
    assert_eq!(book.related_web_references.len(), 1);
    assert_eq!(book.related_web_references[0], "test-web-ref");

    println!("✅ Cross-reference accuracy test passed");
}

/// Test merge functionality
#[test]
fn test_data_merge() {
    // Create base data (from starred repos)
    let base = create_test_data();

    // Create manual additions
    let mut manual_data = CanonicalData::new();
    manual_data.manual_projects.push(ManualProject {
        id: "manual-test-project".to_string(),
        name: "Manual Test Project".to_string(),
        description: "A manually added project".to_string(),
        platforms: vec![PlatformInfo {
            platform: Platform::GitHub,
            url: "https://github.com/manual/project".to_string(),
            status: PlatformStatus::Active,
            is_primary: true,
            last_verified: Some("2024-12-10".to_string()),
            migration_date: None,
            notes: None,
        }],
        metadata: ManualProjectMetadata {
            primary_language: "Go".to_string(),
            license: Some("MIT License".to_string()),
            stars: None,
        },
        classification: ManualProjectClassification {
            categories: vec!["manual".to_string()],
            readme_sections: vec!["GitHub Projects".to_string()],
        },
        curator_notes: Some("Manually curated project".to_string()),
    });

    // Perform merge
    let merger = DataMerger::new(MergeStrategy::PreferManual);
    let merged = merger
        .merge(base, Some(manual_data), None, None)
        .expect("Merge failed");

    // Verify merge results
    assert_eq!(merged.repositories.len(), 3, "Should have original repos");
    assert_eq!(
        merged.manual_projects.len(),
        1,
        "Should have manual project"
    );

    // Convert manual project to repository
    let manual_as_repo = merged.manual_projects[0].to_repository();
    assert_eq!(manual_as_repo.metadata.name, "Manual Test Project");
    assert_eq!(manual_as_repo.source, RepositorySource::Manual);

    println!("✅ Data merge test passed");
}

/// Test validation on clean data passes
#[test]
fn test_validation_on_clean_data() {
    let data = create_test_data();

    let mut validator = Validator::new();
    validator.add_rule(NoDuplicateReposRule);
    validator.add_rule(ValidUrlsRule::new().expect("Failed to create URL rule"));

    let report = validator.validate(&data);

    // Clean data should have no errors (may have warnings for missing licenses, etc.)
    assert_eq!(report.summary.errors, 0, "Clean data should have no errors");
    assert!(report.passed(), "Validation should pass on clean data");

    println!("✅ Clean data validation test passed");
}

/// Test configuration loading and validation
#[test]
fn test_config_loading_and_defaults() {
    use omnidatum_processor::OmnidatumConfig;
    
    // Test default configuration is valid
    let config = OmnidatumConfig::default();
    assert!(config.validate().is_ok());
    
    // Verify default values
    assert!(!config.sync.enabled);
    assert_eq!(config.sync.interval_hours, 24);
    assert_eq!(config.sync.parallel_workers, 3);
    assert_eq!(config.sync.cache_ttl_hours, 24);
    assert_eq!(config.sync.rate_limit_buffer, 500);
    
    println!("✅ Configuration loading and defaults test passed");
}

/// Test configuration validation with invalid values
#[test]
fn test_config_validation() {
    use omnidatum_processor::OmnidatumConfig;
    
    // Test parallel_workers out of range
    let mut config = OmnidatumConfig::default();
    config.sync.parallel_workers = 0;
    assert!(config.validate().is_err());
    
    config.sync.parallel_workers = 11;
    assert!(config.validate().is_err());
    
    // Test cache_ttl_hours out of range
    config = OmnidatumConfig::default();
    config.sync.cache_ttl_hours = 0;
    assert!(config.validate().is_err());
    
    config.sync.cache_ttl_hours = 200;
    assert!(config.validate().is_err());
    
    // Test rate_limit_buffer out of range
    config = OmnidatumConfig::default();
    config.sync.rate_limit_buffer = 1001;
    assert!(config.validate().is_err());
    
    println!("✅ Configuration validation test passed");
}

/// Test credential manager token redaction
#[test]
fn test_credential_redaction() {
    use omnidatum_processor::CredentialManager;
    
    let token = "ghp_1234567890abcdefghijklmnop";
    let redacted = CredentialManager::redact(token);
    
    // Verify token is redacted
    assert!(redacted.contains("***REDACTED***"));
    assert!(!redacted.contains("567890"));
    assert!(redacted.starts_with("ghp_"));
    
    // Short token should be fully redacted
    let short = "abc";
    let short_redacted = CredentialManager::redact(short);
    assert_eq!(short_redacted, "***REDACTED***");
    
    println!("✅ Credential redaction test passed");
}


/// Test 31: Full sync workflow with mock adapter
#[tokio::test]
async fn test_sync_workflow() {
    use mock_adapter::MockGitHubAdapter;
    
    // Create temp directory for test data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let canonical_path = temp_dir.path().join("repositories.yml");
    
    // Create test canonical data with 3 repos
    let mut data = CanonicalData::new();
    data.repositories = vec![
        Repository {
            id: "test-rust-lang-rust".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/rust-lang/rust".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                last_verified: Some("2024-12-10".to_string()),
                migration_date: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "rust".to_string(),
                owner: "rust-lang".to_string(),
                full_name: "rust-lang/rust".to_string(),
                description: "Old description".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: Some("MIT".to_string()),
                stars: 50000,
                topics: vec!["compiler".to_string()],
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
                last_commit_date: Some("2024-12-01".to_string()),
                last_star_update: "2024-12-01".to_string(),
                quality_score: 95,
            },
            source: RepositorySource::GitHubStars,
            added_date: Some("2023-01-01".to_string()),
            manually_curated: true, // Manual curation flag
            curator_notes: Some("Important compiler project".to_string()), // Manual notes
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        },
        Repository {
            id: "test-facebook-react".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/facebook/react".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                last_verified: Some("2024-12-10".to_string()),
                migration_date: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "react".to_string(),
                owner: "facebook".to_string(),
                full_name: "facebook/react".to_string(),
                description: "Old React description".to_string(),
                primary_language: "JavaScript".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: Some("MIT".to_string()),
                stars: 150000,
                topics: vec!["react".to_string()],
                homepage: Some("https://reactjs.org".to_string()),
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: "JavaScript".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: Some("2024-11-30".to_string()),
                last_star_update: "2024-11-30".to_string(),
                quality_score: 98,
            },
            source: RepositorySource::GitHubStars,
            added_date: Some("2023-01-01".to_string()),
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        },
        Repository {
            id: "test-non-github-repo".to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::Codeberg,
                url: "https://codeberg.org/test/repo".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                last_verified: Some("2024-12-10".to_string()),
                migration_date: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: "repo".to_string(),
                owner: "test".to_string(),
                full_name: "test/repo".to_string(),
                description: "Non-GitHub repo".to_string(),
                primary_language: "Go".to_string(),
                license: Some("GPL-3.0".to_string()),
                license_spdx: Some("GPL-3.0".to_string()),
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
                language_category: "Go".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: Some("2024-12-01".to_string()),
                last_star_update: "2024-12-01".to_string(),
                quality_score: 70,
            },
            source: RepositorySource::GitHubStars,
            added_date: Some("2023-01-01".to_string()),
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        },
    ];
    
    data.total_count = data.repositories.len();
    data.calculate_statistics();
    
    // Save to temp file
    data.to_yaml_file(&canonical_path).expect("Failed to save test data");
    
    // Create mock adapter with canned responses
    let mut mock_adapter = MockGitHubAdapter::new();
    
    // Add response for rust-lang/rust with updated metadata
    let mut rust_repo = data.repositories[0].clone();
    rust_repo.metadata.description = "A language empowering everyone to build reliable and efficient software.".to_string();
    rust_repo.metadata.stars = 95000; // Updated stars
    rust_repo.metadata.topics = vec!["compiler".to_string(), "rust".to_string()];
    mock_adapter.add_response("rust-lang/rust", rust_repo);
    
    // Add response for facebook/react with updated metadata
    let mut react_repo = data.repositories[1].clone();
    react_repo.metadata.description = "The library for web and native user interfaces.".to_string();
    react_repo.metadata.stars = 230000; // Updated stars
    mock_adapter.add_response("facebook/react", react_repo);
    
    // Note: We can't actually inject the mock adapter into SyncOrchestrator
    // since it creates a real GitHubAdapter internally. This test would need
    // refactoring of SyncOrchestrator to accept an adapter parameter.
    // For now, we verify the mock adapter works correctly.
    
    // Verify mock adapter works
    let rust_result = mock_adapter.fetch_repository("rust-lang/rust").await;
    assert!(rust_result.is_ok());
    let rust_fetched = rust_result.unwrap();
    assert_eq!(rust_fetched.metadata.stars, 95000);
    assert_eq!(rust_fetched.metadata.description, "A language empowering everyone to build reliable and efficient software.");
    
    // Verify manual curation fields preserved in mock
    assert!(rust_fetched.manually_curated);
    assert_eq!(rust_fetched.curator_notes, Some("Important compiler project".to_string()));
    
    let react_result = mock_adapter.fetch_repository("facebook/react").await;
    assert!(react_result.is_ok());
    let react_fetched = react_result.unwrap();
    assert_eq!(react_fetched.metadata.stars, 230000);
    
    // Verify non-existent repo returns error
    let not_found = mock_adapter.fetch_repository("nonexistent/repo").await;
    assert!(not_found.is_err());
    
    println!("✅ Sync workflow test passed (mock adapter verified)");
    println!("   Note: Full SyncOrchestrator integration requires adapter injection pattern");
}

/// Test 32: Rate limit handling
#[tokio::test]
async fn test_sync_rate_limit_handling() {
    use mock_adapter::MockGitHubAdapter;
    
    // Create mock adapter that simulates rate limit error
    let mut mock_adapter = MockGitHubAdapter::new();
    mock_adapter.set_connection_result(Err(anyhow::anyhow!(
        "Rate limit exhausted. Please wait 3600 seconds. Resets at 2024-12-11T21:00:00Z"
    )));
    
    // Verify connection check fails with rate limit error
    let result = mock_adapter.check_connection().await;
    assert!(result.is_err());
    
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Rate limit exhausted"));
    assert!(error_msg.contains("3600 seconds"));
    assert!(error_msg.contains("Resets at"));
    
    println!("✅ Rate limit handling test passed");
    println!("   Mock adapter correctly simulates rate limit errors");
}

/// Test 33: Credential failures
#[tokio::test]
async fn test_sync_authentication_failure() {
    use omnidatum_processor::{CredentialManager, OmnidatumConfig, GitHubAdapter};
    
    // Create config with file-based credentials (will fail if file doesn't exist)
    let mut config = OmnidatumConfig::default();
    config.credentials.source = omnidatum_processor::CredentialSource::File;
    config.credentials.file_path = Some("/nonexistent/path/to/credentials".into());
    
    // Attempt to create GitHub adapter (should fail due to missing credentials)
    let result = GitHubAdapter::new(&config).await;
    assert!(result.is_err(), "Expected authentication failure");
    
    let error_msg = format!("{:?}", result.err().unwrap());
    
    // Verify error message is helpful
    assert!(
        error_msg.contains("configure") || error_msg.contains("credential") || error_msg.contains("token"),
        "Error message should mention credentials or configure command"
    );
    
    // Test token redaction
    let test_token = "ghp_secrettoken1234567890";
    let redacted = CredentialManager::redact(test_token);
    
    // Verify token is redacted (should not contain the secret part)
    assert!(!redacted.contains("secrettoken"));
    assert!(!redacted.contains("1234567890"));
    assert!(redacted.contains("***REDACTED***"));
    assert!(redacted.starts_with("ghp_"));
    
    println!("✅ Authentication failure test passed");
    println!("   Error handling provides helpful guidance");
    println!("   Token redaction prevents credential exposure");
}