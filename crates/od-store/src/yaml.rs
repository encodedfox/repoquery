use crate::traits::{RepoFilter, RepoStore};
use anyhow::Result;
use od_core::{CanonicalData, Collection, Repository};
use std::path::{Path, PathBuf};

pub struct YamlStore {
    pub path: PathBuf,
}

impl YamlStore {
    pub fn new(path: &Path) -> Self {
        Self { path: path.to_path_buf() }
    }
}

impl RepoStore for YamlStore {
    fn load_all(&self) -> Result<CanonicalData> {
        Ok(CanonicalData::from_yaml_file(&self.path)?)
    }

    fn save_all(&self, data: &CanonicalData) -> Result<()> {
        Ok(data.to_yaml_file(&self.path)?)
    }

    fn get_repo(&self, id: &str) -> Result<Option<Repository>> {
        let data = self.load_all()?;
        Ok(data.repositories.into_iter().find(|r| r.id == id))
    }

    fn upsert_repo(&self, repo: &Repository) -> Result<()> {
        let mut data = self.load_all()?;
        if let Some(pos) = data.repositories.iter().position(|r| r.id == repo.id) {
            data.repositories[pos] = repo.clone();
        } else {
            data.repositories.push(repo.clone());
        }
        self.save_all(&data)
    }

    fn list_repos(&self, filter: &RepoFilter) -> Result<Vec<Repository>> {
        let data = self.load_all()?;
        Ok(data
            .repositories
            .into_iter()
            .filter(|r| {
                if let Some(lang) = &filter.language {
                    if &r.metadata.primary_language != lang {
                        return false;
                    }
                }
                if let Some(archived) = filter.archived {
                    if r.quality_metrics.archive_status != archived {
                        return false;
                    }
                }
                if let Some(min) = filter.min_stars {
                    if r.metadata.stars < min {
                        return false;
                    }
                }
                if let Some(src) = &filter.source {
                    let src_str = format!("{:?}", r.source).to_lowercase();
                    if !src_str.contains(&src.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .collect())
    }

    fn count_repos(&self, filter: &RepoFilter) -> Result<usize> {
        Ok(self.list_repos(filter)?.len())
    }

    fn delete_repo(&self, id: &str) -> Result<bool> {
        let mut data = self.load_all()?;
        let before = data.repositories.len();
        data.repositories.retain(|r| r.id != id);
        let deleted = data.repositories.len() < before;
        if deleted {
            self.save_all(&data)?;
        }
        Ok(deleted)
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        Ok(self.load_all()?.collections)
    }

    fn get_collection(&self, id: &str) -> Result<Option<Collection>> {
        Ok(self.load_all()?.collections.into_iter().find(|c| c.id == id))
    }

    fn save_collection(&self, collection: &Collection) -> Result<()> {
        let mut data = self.load_all()?;
        if let Some(pos) = data.collections.iter().position(|c| c.id == collection.id) {
            data.collections[pos] = collection.clone();
        } else {
            data.collections.push(collection.clone());
        }
        self.save_all(&data)
    }

    fn delete_collection(&self, id: &str) -> Result<bool> {
        let mut data = self.load_all()?;
        let before = data.collections.len();
        data.collections.retain(|c| c.id != id);
        let deleted = data.collections.len() < before;
        if deleted {
            self.save_all(&data)?;
        }
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::{
        Platform, PlatformInfo, PlatformStatus, QualityMetrics, RepositoryClassification,
        RepositoryMetadata, RepositorySource,
    };
    use tempfile::NamedTempFile;

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

    fn make_data(repos: Vec<Repository>) -> CanonicalData {
        CanonicalData {
            schema_version: "1.0".to_string(),
            last_updated: "2024-01-01T00:00:00Z".to_string(),
            generated_by: "test".to_string(),
            total_count: repos.len(),
            repositories: repos,
            manual_projects: vec![],
            web_references: vec![],
            books: vec![],
            collections: vec![],
            statistics: None,
        }
    }

    #[test]
    fn test_yaml_round_trip() {
        let tmp = NamedTempFile::new().unwrap();
        let store = YamlStore::new(tmp.path());
        let data = make_data(vec![make_repo("repo-a"), make_repo("repo-b")]);
        store.save_all(&data).unwrap();
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.repositories.len(), 2);
        assert_eq!(loaded.repositories[0].id, "repo-a");
    }

    #[test]
    fn test_yaml_upsert_get_delete() {
        let tmp = NamedTempFile::new().unwrap();
        let store = YamlStore::new(tmp.path());
        store.save_all(&make_data(vec![])).unwrap();

        let repo = make_repo("my-repo");
        store.upsert_repo(&repo).unwrap();
        assert!(store.get_repo("my-repo").unwrap().is_some());

        assert!(store.delete_repo("my-repo").unwrap());
        assert!(store.get_repo("my-repo").unwrap().is_none());
    }
}
