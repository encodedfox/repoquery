//! od-store: trait-based persistence layer (SQLite + YAML).

pub mod error;
pub mod sqlite;
pub mod traits;
pub mod yaml;

pub use error::StoreError;
pub use sqlite::SqliteStore;
pub use traits::{RepoFilter, RepoStore};
pub use yaml::YamlStore;

use anyhow::{bail, Result};
use od_core::CanonicalData;
use std::path::Path;

/// Open a store based on file extension.
/// `.db` / `.sqlite` → SqliteStore; `.yml` / `.yaml` → YamlStore.
pub fn open_store(path: &Path) -> Result<Box<dyn RepoStore>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("db") | Some("sqlite") => Ok(Box::new(SqliteStore::new(path)?)),
        Some("yml") | Some("yaml") => Ok(Box::new(YamlStore::new(path))),
        _ => bail!(
            "Cannot determine store type from path '{}'. Use .db/.sqlite or .yml/.yaml",
            path.display()
        ),
    }
}

// ── backward-compat functions ─────────────────────────────────────────────────

/// Load canonical data from a YAML file.
pub fn load_canonical(path: &Path) -> Result<CanonicalData> {
    Ok(CanonicalData::from_yaml_file(path)?)
}

/// Save canonical data to a YAML file.
pub fn save_canonical(path: &Path, data: &CanonicalData) -> Result<()> {
    Ok(data.to_yaml_file(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::{
        Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository,
        RepositoryClassification, RepositoryMetadata, RepositorySource,
    };

    fn make_repo(id: &str) -> Repository {
        Repository {
            id: id.to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: format!("https://github.com/owner/{}", id),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: id.to_string(),
                owner: "owner".to_string(),
                full_name: format!("owner/{}", id),
                description: "desc".to_string(),
                primary_language: "Rust".to_string(),
                license: None,
                license_spdx: None,
                stars: 42,
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
                last_star_update: "2024-01-01".to_string(),
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
        }
    }

    #[test]
    fn test_cross_store_equivalence() {
        let data = CanonicalData {
            schema_version: "1.0".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            generated_by: "test".to_string(),
            total_count: 2,
            repositories: vec![make_repo("repo-a"), make_repo("repo-b")],
            manual_projects: vec![],
            web_references: vec![],
            books: vec![],
            collections: vec![],
            statistics: None,
        };

        // Save to YAML, load into SQLite, compare
        let yaml_file = tempfile::NamedTempFile::with_suffix(".yml").unwrap();
        let yaml_store = YamlStore::new(yaml_file.path());
        yaml_store.save_all(&data).unwrap();

        let sqlite_store = SqliteStore::new(std::path::Path::new(":memory:")).unwrap();
        let from_yaml = yaml_store.load_all().unwrap();
        sqlite_store.save_all(&from_yaml).unwrap();
        let from_sqlite = sqlite_store.load_all().unwrap();

        assert_eq!(from_yaml.repositories.len(), from_sqlite.repositories.len());
        assert_eq!(from_yaml.schema_version, from_sqlite.schema_version);
        for (y, s) in from_yaml.repositories.iter().zip(from_sqlite.repositories.iter()) {
            assert_eq!(y.id, s.id);
            assert_eq!(y.metadata.stars, s.metadata.stars);
            assert_eq!(y.metadata.primary_language, s.metadata.primary_language);
            assert_eq!(y.quality_metrics.archive_status, s.quality_metrics.archive_status);
        }
    }
}