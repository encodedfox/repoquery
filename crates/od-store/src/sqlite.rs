use crate::traits::{RepoFilter, RepoStore};
use anyhow::{Context, Result};
use od_core::{Book, CanonicalData, Collection, ManualProject, Repository, WebReference};
use rusqlite::{params, Connection};
use std::path::Path;

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = if path == Path::new(":memory:") {
            Connection::open_in_memory()?
        } else {
            Connection::open(path)?
        };
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS repositories (
                id TEXT PRIMARY KEY,
                name TEXT,
                owner TEXT,
                full_name TEXT,
                primary_language TEXT,
                stars INTEGER,
                archive_status INTEGER,
                quality_score INTEGER,
                language_category TEXT,
                source TEXT,
                metadata_json TEXT NOT NULL,
                classification_json TEXT NOT NULL,
                quality_json TEXT NOT NULL,
                platforms_json TEXT NOT NULL,
                added_date TEXT,
                manually_curated INTEGER NOT NULL DEFAULT 0,
                curator_notes TEXT,
                relations_json TEXT NOT NULL DEFAULT '[]',
                custom_tags_json TEXT NOT NULL DEFAULT '[]',
                fork_ahead INTEGER,
                fork_behind INTEGER
            );
            CREATE TABLE IF NOT EXISTS manual_projects (
                id TEXT PRIMARY KEY,
                data_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS web_references (
                id TEXT PRIMARY KEY,
                data_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS books (
                id TEXT PRIMARY KEY,
                data_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                data_json TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    fn upsert_repo_inner(&self, repo: &Repository) -> Result<()> {
        // Store source as its JSON-serialized form (e.g. "github_stars") — keep quotes
        // so round-trip via repo_from_row works without re-quoting.
        let source_json = serde_json::to_string(&repo.source)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO repositories
             (id, name, owner, full_name, primary_language, stars, archive_status,
              quality_score, language_category, source,
              metadata_json, classification_json, quality_json, platforms_json,
              added_date, manually_curated, curator_notes, relations_json,
              custom_tags_json, fork_ahead, fork_behind)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
            params![
                repo.id,
                repo.metadata.name,
                repo.metadata.owner,
                repo.metadata.full_name,
                repo.metadata.primary_language,
                repo.metadata.stars as i64,
                if repo.quality_metrics.archive_status { 1i64 } else { 0i64 },
                repo.quality_metrics.quality_score as i64,
                repo.classification.language_category,
                source_json,
                serde_json::to_string(&repo.metadata)?,
                serde_json::to_string(&repo.classification)?,
                serde_json::to_string(&repo.quality_metrics)?,
                serde_json::to_string(&repo.platforms)?,
                repo.added_date,
                if repo.manually_curated { 1i64 } else { 0i64 },
                repo.curator_notes,
                serde_json::to_string(&repo.relations)?,
                serde_json::to_string(&repo.custom_tags)?,
                repo.fork_ahead.map(|v| v as i64),
                repo.fork_behind.map(|v| v as i64),
            ],
        )?;
        Ok(())
    }
}

impl RepoStore for SqliteStore {
    fn load_all(&self) -> Result<CanonicalData> {
        let mut stmt = self.conn.prepare(
            "SELECT id, metadata_json, classification_json, quality_json, platforms_json,
                    source, added_date, manually_curated, curator_notes, relations_json,
                    custom_tags_json, fork_ahead, fork_behind
             FROM repositories",
        )?;
        let repositories: Vec<Repository> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                    row.get::<_, Option<i64>>(12)?,
                ))
            })?
            .map(|r| {
                let (id, meta_j, class_j, qual_j, plat_j, src_j, added, curated, notes, rel_j, tags_j, ahead, behind) = r?;
                repo_from_row(&id, &meta_j, &class_j, &qual_j, &plat_j, &src_j, added, curated, notes, &rel_j, tags_j.as_deref(), ahead, behind)
            })
            .collect::<Result<Vec<_>>>()?;

        let manual_projects = load_json_table::<ManualProject>(&self.conn, "manual_projects")?;
        let web_references = load_json_table::<WebReference>(&self.conn, "web_references")?;
        let books = load_json_table::<Book>(&self.conn, "books")?;
        let collections = load_json_table::<Collection>(&self.conn, "collections")?;

        let schema_version = get_meta(&self.conn, "schema_version")
            .unwrap_or_else(|| "1.0".to_string());
        let last_updated = get_meta(&self.conn, "last_updated")
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let generated_by = get_meta(&self.conn, "generated_by")
            .unwrap_or_else(|| "omnidatum-processor".to_string());
        let total_count = repositories.len()
            + manual_projects.len()
            + web_references.len()
            + books.len();

        Ok(CanonicalData {
            schema_version,
            last_updated,
            generated_by,
            total_count,
            repositories,
            manual_projects,
            web_references,
            books,
            collections,
            statistics: None,
        })
    }

    fn save_all(&self, data: &CanonicalData) -> Result<()> {
        self.conn.execute_batch("BEGIN;")?;

        self.conn.execute("DELETE FROM repositories", [])?;
        self.conn.execute("DELETE FROM manual_projects", [])?;
        self.conn.execute("DELETE FROM web_references", [])?;
        self.conn.execute("DELETE FROM books", [])?;
        self.conn.execute("DELETE FROM collections", [])?;

        for repo in &data.repositories {
            self.upsert_repo_inner(repo)?;
        }
        for item in &data.manual_projects {
            self.conn.execute(
                "INSERT OR REPLACE INTO manual_projects (id, data_json) VALUES (?1, ?2)",
                params![item.id, serde_json::to_string(item)?],
            )?;
        }
        for item in &data.web_references {
            self.conn.execute(
                "INSERT OR REPLACE INTO web_references (id, data_json) VALUES (?1, ?2)",
                params![item.id, serde_json::to_string(item)?],
            )?;
        }
        for item in &data.books {
            self.conn.execute(
                "INSERT OR REPLACE INTO books (id, data_json) VALUES (?1, ?2)",
                params![item.id, serde_json::to_string(item)?],
            )?;
        }
        for item in &data.collections {
            self.conn.execute(
                "INSERT OR REPLACE INTO collections (id, data_json) VALUES (?1, ?2)",
                params![item.id, serde_json::to_string(item)?],
            )?;
        }

        set_meta(&self.conn, "schema_version", &data.schema_version)?;
        set_meta(&self.conn, "last_updated", &data.last_updated)?;
        set_meta(&self.conn, "generated_by", &data.generated_by)?;

        self.conn.execute_batch("COMMIT;")?;
        Ok(())
    }

    fn get_repo(&self, id: &str) -> Result<Option<Repository>> {
        let mut stmt = self.conn.prepare(
            "SELECT metadata_json, classification_json, quality_json, platforms_json,
                    source, added_date, manually_curated, curator_notes, relations_json,
                    custom_tags_json, fork_ahead, fork_behind
             FROM repositories WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let repo = repo_from_row(
                id,
                &row.get::<_, String>(0)?,
                &row.get::<_, String>(1)?,
                &row.get::<_, String>(2)?,
                &row.get::<_, String>(3)?,
                &row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<String>>(7)?,
                &row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?.as_deref(),
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<i64>>(11)?,
            )?;
            Ok(Some(repo))
        } else {
            Ok(None)
        }
    }

    fn upsert_repo(&self, repo: &Repository) -> Result<()> {
        self.upsert_repo_inner(repo)
    }

    fn list_repos(&self, filter: &RepoFilter) -> Result<Vec<Repository>> {
        // Build WHERE clause and collect params as boxed values.
        let mut conditions: Vec<&str> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(lang) = &filter.language {
            conditions.push("primary_language = ?");
            param_values.push(Box::new(lang.clone()));
        }
        if let Some(archived) = filter.archived {
            conditions.push("archive_status = ?");
            param_values.push(Box::new(if archived { 1i64 } else { 0i64 }));
        }
        if let Some(min) = filter.min_stars {
            conditions.push("stars >= ?");
            param_values.push(Box::new(min as i64));
        }
        if let Some(src) = &filter.source {
            conditions.push("source = ?");
            param_values.push(Box::new(src.clone()));
        }
        if let Some(tag) = &filter.tag {
            conditions.push("custom_tags_json LIKE ?");
            param_values.push(Box::new(format!("%\"{}\"%" , tag)));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, metadata_json, classification_json, quality_json, platforms_json,
                    source, added_date, manually_curated, curator_notes, relations_json,
                    custom_tags_json, fork_ahead, fork_behind
             FROM repositories{}",
            where_clause
        );

        let refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, Option<i64>>(12)?,
            ))
        })?;

        rows.map(|r| {
            let (id, meta_j, class_j, qual_j, plat_j, src_j, added, curated, notes, rel_j, tags_j, ahead, behind) = r?;
            repo_from_row(&id, &meta_j, &class_j, &qual_j, &plat_j, &src_j, added, curated, notes, &rel_j, tags_j.as_deref(), ahead, behind)
        })
        .collect()
    }

    fn count_repos(&self, filter: &RepoFilter) -> Result<usize> {
        Ok(self.list_repos(filter)?.len())
    }

    fn delete_repo(&self, id: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM repositories WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        load_json_table::<Collection>(&self.conn, "collections")
    }

    fn get_collection(&self, id: &str) -> Result<Option<Collection>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data_json FROM collections WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            Ok(Some(serde_json::from_str(&json)?))
        } else {
            Ok(None)
        }
    }

    fn save_collection(&self, collection: &Collection) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO collections (id, data_json) VALUES (?1, ?2)",
            params![collection.id, serde_json::to_string(collection)?],
        )?;
        Ok(())
    }

    fn delete_collection(&self, id: &str) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Reconstruct a Repository from its stored column values.
/// `src_j` is the JSON-encoded source string (e.g. `"github_stars"` with quotes).
#[allow(clippy::too_many_arguments)]
fn repo_from_row(
    id: &str,
    meta_j: &str,
    class_j: &str,
    qual_j: &str,
    plat_j: &str,
    src_j: &str,
    added: Option<String>,
    curated: i64,
    notes: Option<String>,
    rel_j: &str,
    tags_j: Option<&str>,
    fork_ahead: Option<i64>,
    fork_behind: Option<i64>,
) -> Result<Repository> {
    let custom_tags: Vec<String> = tags_j
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let json = format!(
        r#"{{"id":{},"metadata":{},"classification":{},"quality_metrics":{},"platforms":{},"source":{},"added_date":{},"manually_curated":{},"curator_notes":{},"relations":{},"custom_tags":{},"fork_ahead":{},"fork_behind":{}}}"#,
        serde_json::to_string(id)?,
        meta_j,
        class_j,
        qual_j,
        plat_j,
        src_j,
        serde_json::to_string(&added)?,
        if curated != 0 { "true" } else { "false" },
        serde_json::to_string(&notes)?,
        rel_j,
        serde_json::to_string(&custom_tags)?,
        fork_ahead.map(|v| v as u32).map_or("null".to_string(), |v| v.to_string()),
        fork_behind.map(|v| v as u32).map_or("null".to_string(), |v| v.to_string()),
    );
    serde_json::from_str(&json).context("deserializing repository from SQLite row")
}

fn load_json_table<T: serde::de::DeserializeOwned>(
    conn: &Connection,
    table: &str,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(&format!("SELECT data_json FROM {}", table))?;
    let rows: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<_, _>>()?;
    rows.iter()
        .map(|json| {
            serde_json::from_str(json)
                .map_err(|e| anyhow::anyhow!("loading {}: {}", table, e))
        })
        .collect()
}

fn get_meta(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::{
        Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository,
        RepositoryClassification, RepositoryMetadata, RepositorySource,
    };

    fn make_repo(id: &str, language: &str, stars: u32, archived: bool) -> Repository {
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
                primary_language: language.to_string(),
                license: None,
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
                language_category: language.to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: archived,
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
    fn test_round_trip() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let data = make_data(vec![
            make_repo("repo-a", "Rust", 100, false),
            make_repo("repo-b", "Go", 50, true),
        ]);
        store.save_all(&data).unwrap();
        let loaded = store.load_all().unwrap();
        assert_eq!(loaded.repositories.len(), 2);
        assert_eq!(loaded.schema_version, "1.0");
    }

    #[test]
    fn test_upsert_get_delete() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let repo = make_repo("my-repo", "Rust", 200, false);
        store.upsert_repo(&repo).unwrap();

        let got = store.get_repo("my-repo").unwrap().unwrap();
        assert_eq!(got.id, "my-repo");
        assert_eq!(got.metadata.stars, 200);

        let mut updated = repo.clone();
        updated.metadata.stars = 300;
        store.upsert_repo(&updated).unwrap();
        let got2 = store.get_repo("my-repo").unwrap().unwrap();
        assert_eq!(got2.metadata.stars, 300);

        assert!(store.delete_repo("my-repo").unwrap());
        assert!(store.get_repo("my-repo").unwrap().is_none());
        assert!(!store.delete_repo("my-repo").unwrap());
    }

    #[test]
    fn test_list_filter_language() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let data = make_data(vec![
            make_repo("r1", "Rust", 100, false),
            make_repo("r2", "Go", 200, false),
            make_repo("r3", "Rust", 50, false),
        ]);
        store.save_all(&data).unwrap();

        let filter = RepoFilter {
            language: Some("Rust".to_string()),
            ..Default::default()
        };
        let results = store.list_repos(&filter).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.metadata.primary_language == "Rust"));
    }

    #[test]
    fn test_list_filter_stars() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let data = make_data(vec![
            make_repo("r1", "Rust", 10, false),
            make_repo("r2", "Rust", 500, false),
            make_repo("r3", "Rust", 1000, false),
        ]);
        store.save_all(&data).unwrap();

        let filter = RepoFilter {
            min_stars: Some(100),
            ..Default::default()
        };
        let results = store.list_repos(&filter).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_list_filter_archived() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let data = make_data(vec![
            make_repo("r1", "Rust", 100, false),
            make_repo("r2", "Rust", 100, true),
            make_repo("r3", "Rust", 100, true),
        ]);
        store.save_all(&data).unwrap();

        let filter = RepoFilter {
            archived: Some(true),
            ..Default::default()
        };
        let results = store.list_repos(&filter).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_count_repos() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let data = make_data(vec![
            make_repo("r1", "Rust", 100, false),
            make_repo("r2", "Go", 200, false),
        ]);
        store.save_all(&data).unwrap();
        assert_eq!(store.count_repos(&RepoFilter::default()).unwrap(), 2);
    }

    #[test]
    fn test_collection_crud() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();

        assert!(store.list_collections().unwrap().is_empty());

        let mut c = od_core::Collection::new("c1".to_string(), "My List".to_string());
        c.add_repo("repo-a".to_string());
        store.save_collection(&c).unwrap();

        let got = store.get_collection("c1").unwrap().unwrap();
        assert_eq!(got.name, "My List");
        assert_eq!(got.repo_ids, vec!["repo-a"]);

        // upsert update
        c.add_repo("repo-b".to_string());
        store.save_collection(&c).unwrap();
        let got2 = store.get_collection("c1").unwrap().unwrap();
        assert_eq!(got2.repo_ids.len(), 2);

        assert!(store.delete_collection("c1").unwrap());
        assert!(store.get_collection("c1").unwrap().is_none());
        assert!(!store.delete_collection("c1").unwrap());
    }

    #[test]
    fn test_repo_relations_round_trip() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let mut repo = make_repo("r1", "Rust", 100, false);
        repo.relations = vec![od_core::Relation::Owned, od_core::Relation::Starred];
        store.upsert_repo(&repo).unwrap();

        let got = store.get_repo("r1").unwrap().unwrap();
        assert_eq!(got.relations, vec![od_core::Relation::Owned, od_core::Relation::Starred]);
    }

    #[test]
    fn test_custom_tags_round_trip() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let mut repo = make_repo("r1", "Rust", 100, false);
        repo.custom_tags = vec!["ml".to_string(), "cli".to_string()];
        repo.fork_ahead = Some(3);
        repo.fork_behind = Some(7);
        store.upsert_repo(&repo).unwrap();

        let got = store.get_repo("r1").unwrap().unwrap();
        assert_eq!(got.custom_tags, vec!["ml", "cli"]);
        assert_eq!(got.fork_ahead, Some(3));
        assert_eq!(got.fork_behind, Some(7));
    }

    #[test]
    fn test_list_filter_tag() {
        let store = SqliteStore::new(Path::new(":memory:")).unwrap();
        let mut r1 = make_repo("r1", "Rust", 100, false);
        r1.custom_tags = vec!["ml".to_string()];
        let mut r2 = make_repo("r2", "Go", 200, false);
        r2.custom_tags = vec!["cli".to_string()];
        let r3 = make_repo("r3", "Rust", 50, false);
        let data = make_data(vec![r1, r2, r3]);
        store.save_all(&data).unwrap();

        let filter = RepoFilter { tag: Some("ml".to_string()), ..Default::default() };
        let results = store.list_repos(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "r1");
    }
}
