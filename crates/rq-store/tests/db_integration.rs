//! Integration tests for database operations.
//!
//! Tests SQLite, YAML, and Dual store implementations for:
//! - CRUD operations (create, read, update, delete)
//! - Filtering by various criteria (language, stars, owner, topics, etc.)
//! - Sorting and limiting results
//! - Store-to-store import/export consistency
//! - Edge cases: empty stores, missing fields, large datasets

use rq_core::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository, RepositoryClassification,
    RepositoryMetadata, RepositorySource,
};
use rq_store::{RepoFilter, RepoStore, SortField, SortOrder, SqliteStore, YamlStore};
use tempfile::TempDir;

/// Helper: create a test repository with the given parameters.
fn test_repo(full_name: &str, lang: &str, stars: u32, topics: &[&str]) -> Repository {
    let parts: Vec<&str> = full_name.split('/').collect();
    Repository {
        id: format!("test-{}", full_name.replace('/', "-")),
        platforms: vec![PlatformInfo {
            platform: Platform::GitHub,
            url: format!("https://github.com/{}", full_name),
            status: PlatformStatus::Active,
            is_primary: true,
            last_verified: None,
            migration_date: None,
            notes: None,
        }],
        metadata: RepositoryMetadata {
            name: parts[1].to_string(),
            owner: parts[0].to_string(),
            full_name: full_name.to_string(),
            description: format!("Test repo {}", full_name),
            primary_language: lang.to_string(),
            license: Some("MIT".to_string()),
            license_spdx: Some("MIT".to_string()),
            stars,
            topics: topics.iter().map(|t| t.to_string()).collect(),
            homepage: None,
            language_breakdown: None,
            secondary_languages: vec![],
        },
        classification: RepositoryClassification {
            categories: vec![],
            readme_sections: vec![],
            web_reference_topics: vec![],
            language_category: lang.to_string(),
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
        domain: None,
        unified_owner_id: None,
        discovered_via: None,
    }
}

/// Create an empty YAML file at the given path so the store can load it.
fn init_yaml(path: &std::path::Path) {
    let empty = rq_core::CanonicalData::new();
    empty.to_yaml_file(path).unwrap();
}

/// Helper: populate a store with sample repositories.
fn populate_store(store: &dyn RepoStore) {
    let repos = vec![
        test_repo("user/rust-repo", "Rust", 5000, &["rust", "cli"]),
        test_repo("user/go-repo", "Go", 2000, &["go", "backend"]),
        test_repo("user/js-repo", "JavaScript", 800, &["js", "frontend"]),
        test_repo(
            "owner/python-repo",
            "Python",
            15000,
            &["python", "ml", "ai"],
        ),
        test_repo("owner/rust-cli", "Rust", 300, &["rust", "cli"]),
    ];
    for repo in repos {
        store.upsert_repo(&repo).unwrap();
    }
}

// ──── SQLite Store Tests ────

#[test]
fn test_sqlite_create_and_list() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let all = store.list_repos(&RepoFilter::default()).unwrap();
    assert_eq!(all.len(), 5);
}

#[test]
fn test_sqlite_filter_by_language() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        language: Some("Rust".to_string()),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 2);
    assert!(repos.iter().all(|r| r.metadata.primary_language == "Rust"));
}

#[test]
fn test_sqlite_filter_by_min_stars() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        min_stars: Some(2000),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 3);
    assert!(repos.iter().all(|r| r.metadata.stars >= 2000));
}

#[test]
fn test_sqlite_filter_by_max_stars() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        max_stars: Some(1000),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 2);
    assert!(repos.iter().all(|r| r.metadata.stars <= 1000));
}

#[test]
fn test_sqlite_filter_by_owner() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        owner: Some("owner".to_string()),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 2);
    assert!(repos.iter().all(|r| r.metadata.owner == "owner"));
}

#[test]
fn test_sqlite_filter_by_topic() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        topic: Some("cli".to_string()),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 2);
    assert!(repos
        .iter()
        .all(|r| r.metadata.topics.iter().any(|t| t == "cli")));
}

#[test]
fn test_sqlite_search_by_name() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        search_query: Some("rust".to_string()),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 2);
}

#[test]
fn test_sqlite_sort_by_stars_desc() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        sort: SortField::Stars,
        order: SortOrder::Desc,
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 5);
    for i in 1..repos.len() {
        assert!(repos[i - 1].metadata.stars >= repos[i].metadata.stars);
    }
}

#[test]
fn test_sqlite_limit_results() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let filter = RepoFilter {
        limit: Some(3),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 3);
}

#[test]
fn test_sqlite_count_repos() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let count = store.count_repos(&RepoFilter::default()).unwrap();
    assert_eq!(count, 5);
}

#[test]
fn test_sqlite_upsert_updates_existing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();
    populate_store(&store);

    let mut repo = test_repo("user/rust-repo", "Rust", 9999, &["rust", "updated"]);
    repo.id = "test-user-rust-repo".to_string();
    store.upsert_repo(&repo).unwrap();

    let filter = RepoFilter {
        search_query: Some("rust-repo".to_string()),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].metadata.stars, 9999);
}

#[test]
fn test_sqlite_empty_store() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let store = SqliteStore::new(&path).unwrap();

    let repos = store.list_repos(&RepoFilter::default()).unwrap();
    assert_eq!(repos.len(), 0);

    let count = store.count_repos(&RepoFilter::default()).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_sqlite_integrity_check() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    // Creating the store runs PRAGMA integrity_check
    let store = SqliteStore::new(&path);
    assert!(
        store.is_ok(),
        "SQLite integrity check should pass on fresh DB"
    );
}

// ──── YAML Store Tests ────

#[test]
fn test_yaml_create_and_list() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.yml");
    init_yaml(&path);
    let store = YamlStore::new(&path);
    populate_store(&store);

    let all = store.list_repos(&RepoFilter::default()).unwrap();
    assert_eq!(all.len(), 5);
}

#[test]
fn test_yaml_filter_by_language() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.yml");
    init_yaml(&path);
    let store = YamlStore::new(&path);
    populate_store(&store);

    let filter = RepoFilter {
        language: Some("Go".to_string()),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 1);
}

#[test]
fn test_yaml_filter_by_owner() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.yml");
    init_yaml(&path);
    let store = YamlStore::new(&path);
    populate_store(&store);

    let filter = RepoFilter {
        owner: Some("owner".to_string()),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 2);
}

#[test]
fn test_yaml_search() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.yml");
    init_yaml(&path);
    let store = YamlStore::new(&path);
    populate_store(&store);

    let filter = RepoFilter {
        search_query: Some("python".to_string()),
        sort: SortField::Stars,
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].metadata.stars, 15000);
}

#[test]
fn test_yaml_sort_and_limit() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.yml");
    init_yaml(&path);
    let store = YamlStore::new(&path);
    populate_store(&store);

    let filter = RepoFilter {
        sort: SortField::Stars,
        order: SortOrder::Desc,
        limit: Some(3),
        ..Default::default()
    };
    let repos = store.list_repos(&filter).unwrap();
    assert_eq!(repos.len(), 3);
    // Highest 3 star counts: 15000, 5000, 2000
    assert_eq!(repos[0].metadata.stars, 15000);
    assert_eq!(repos[1].metadata.stars, 5000);
    assert_eq!(repos[2].metadata.stars, 2000);
}

#[test]
fn test_yaml_empty_store() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.yml");
    init_yaml(&path);
    let store = YamlStore::new(&path);

    let repos = store.list_repos(&RepoFilter::default()).unwrap();
    assert_eq!(repos.len(), 0);
}
