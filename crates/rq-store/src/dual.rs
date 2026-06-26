use crate::traits::{RepoFilter, RepoStore};
use crate::SqliteStore;
use crate::YamlStore;
use anyhow::Result;
use rq_core::{CanonicalData, Collection, Repository};
use std::path::Path;

pub struct DualStore {
    sqlite: SqliteStore,
    yaml: YamlStore,
}

impl DualStore {
    pub fn new(sqlite_path: &Path, yaml_path: &Path) -> Result<Self> {
        let sqlite = SqliteStore::new(sqlite_path)?;
        let yaml = YamlStore::new(yaml_path);
        Ok(Self { sqlite, yaml })
    }

    /// Rebuild SQLite from YAML (useful after switching to dual mode)
    pub fn rebuild_sqlite_from_yaml(&self) -> Result<()> {
        let data = self.yaml.load_all()?;
        self.sqlite.save_all(&data)
    }

    /// Rebuild YAML from SQLite (useful for migration)
    pub fn rebuild_yaml_from_sqlite(&self) -> Result<()> {
        let data = self.sqlite.load_all()?;
        self.yaml.save_all(&data)
    }
}

impl RepoStore for DualStore {
    fn load_all(&self) -> Result<CanonicalData> {
        self.sqlite.load_all()
    }

    fn save_all(&self, data: &CanonicalData) -> Result<()> {
        self.sqlite.save_all(data)?;
        self.yaml.save_all(data)?;
        Ok(())
    }

    fn get_repo(&self, id: &str) -> Result<Option<Repository>> {
        self.sqlite.get_repo(id)
    }

    fn upsert_repo(&self, repo: &Repository) -> Result<()> {
        self.sqlite.upsert_repo(repo)?;
        self.yaml.upsert_repo(repo)?;
        Ok(())
    }

    fn list_repos(&self, filter: &RepoFilter) -> Result<Vec<Repository>> {
        self.sqlite.list_repos(filter)
    }

    fn count_repos(&self, filter: &RepoFilter) -> Result<usize> {
        self.sqlite.count_repos(filter)
    }

    fn delete_repo(&self, id: &str) -> Result<bool> {
        let deleted = self.sqlite.delete_repo(id)?;
        self.yaml.delete_repo(id)?;
        Ok(deleted)
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        self.sqlite.list_collections()
    }

    fn get_collection(&self, id: &str) -> Result<Option<Collection>> {
        self.sqlite.get_collection(id)
    }

    fn save_collection(&self, collection: &Collection) -> Result<()> {
        self.sqlite.save_collection(collection)?;
        self.yaml.save_collection(collection)?;
        Ok(())
    }

    fn delete_collection(&self, id: &str) -> Result<bool> {
        let deleted = self.sqlite.delete_collection(id)?;
        self.yaml.delete_collection(id)?;
        Ok(deleted)
    }
}
