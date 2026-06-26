use crate::traits::{GraphStore, RepoFilter, RepoStore, SortField, SortOrder};
use anyhow::{Context, Result};
use rq_core::{
    Book, CanonicalData, Collection, DomainRepository, EdgeType, FgatToken, IdentityAlias,
    ManualProject, NormalizedDomain, PlatformKind, Repository, Seed, SeedStatus, TokenStatus,
    TraversalEdge, UnifiedIdentity, WebReference,
};
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
        store.verify_integrity()?;
        store.init_schema()?;
        Ok(store)
    }

    fn verify_integrity(&self) -> Result<()> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .context("Failed to run integrity check")?;
        if result != "ok" {
            tracing::warn!("Database integrity check failed: {}", result);
        } else {
            tracing::debug!("Database integrity check passed");
        }
        Ok(())
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
                fork_behind INTEGER,
                domain TEXT,
                unified_owner_id TEXT,
                discovered_via TEXT
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
            );

            -- Multi-user graph expansion tables
            CREATE TABLE IF NOT EXISTS seeds (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                username TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                depth INTEGER NOT NULL DEFAULT 0,
                max_depth INTEGER NOT NULL DEFAULT 2,
                added_at TEXT NOT NULL,
                completed_at TEXT,
                error_message TEXT,
                UNIQUE(platform, username)
            );
            CREATE TABLE IF NOT EXISTS traversal_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                seed_id TEXT NOT NULL,
                from_user_id TEXT NOT NULL,
                to_user_id TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                depth INTEGER NOT NULL,
                discovered_at TEXT NOT NULL,
                FOREIGN KEY (seed_id) REFERENCES seeds(id)
            );
            CREATE TABLE IF NOT EXISTS fgat_tokens (
                id TEXT PRIMARY KEY,
                platform TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'available',
                requests_used INTEGER NOT NULL DEFAULT 0,
                rate_limit_limit INTEGER,
                rate_limit_remaining INTEGER,
                rate_limit_reset_at TEXT,
                last_used_at TEXT,
                added_at TEXT NOT NULL,
                expires_at TEXT,
                notes TEXT
            );
            CREATE TABLE IF NOT EXISTS normalized_domains (
                domain TEXT PRIMARY KEY,
                canonical_domain TEXT NOT NULL,
                platform TEXT,
                is_verified INTEGER NOT NULL DEFAULT 0,
                confidence_score REAL NOT NULL DEFAULT 1.0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS unified_identities (
                id TEXT PRIMARY KEY,
                canonical_username TEXT NOT NULL,
                primary_domain TEXT NOT NULL,
                confidence_score REAL NOT NULL DEFAULT 1.0,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS identity_aliases (
                unified_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                username TEXT NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0,
                verification_method TEXT,
                FOREIGN KEY (unified_id) REFERENCES unified_identities(id),
                UNIQUE(platform, username)
            );
            CREATE TABLE IF NOT EXISTS domain_repositories (
                id TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                repo_path TEXT NOT NULL,
                full_name TEXT NOT NULL,
                platform TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                indexed_at TEXT NOT NULL,
                UNIQUE(domain, repo_path)
            );

            CREATE INDEX IF NOT EXISTS idx_traversal_edges_seed ON traversal_edges(seed_id);
            CREATE INDEX IF NOT EXISTS idx_traversal_edges_from ON traversal_edges(from_user_id);
            CREATE INDEX IF NOT EXISTS idx_traversal_edges_to ON traversal_edges(to_user_id);
            CREATE INDEX IF NOT EXISTS idx_fgat_tokens_status ON fgat_tokens(status, platform);
            CREATE INDEX IF NOT EXISTS idx_identity_aliases_platform ON identity_aliases(platform, username);
            CREATE INDEX IF NOT EXISTS idx_domain_repos_domain ON domain_repositories(domain);",
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
              custom_tags_json, fork_ahead, fork_behind,
              domain, unified_owner_id, discovered_via)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
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
                repo.domain,
                repo.unified_owner_id,
                repo.discovered_via,
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
                    custom_tags_json, fork_ahead, fork_behind,
                    domain, unified_owner_id, discovered_via
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
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                ))
            })?
            .map(|r| {
                let (
                    id,
                    meta_j,
                    class_j,
                    qual_j,
                    plat_j,
                    src_j,
                    added,
                    curated,
                    notes,
                    rel_j,
                    tags_j,
                    ahead,
                    behind,
                    domain,
                    unified_owner_id,
                    discovered_via,
                ) = r?;
                repo_from_row(
                    &id,
                    &meta_j,
                    &class_j,
                    &qual_j,
                    &plat_j,
                    &src_j,
                    added,
                    curated,
                    notes,
                    &rel_j,
                    tags_j.as_deref(),
                    ahead,
                    behind,
                    domain,
                    unified_owner_id,
                    discovered_via,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let manual_projects = load_json_table::<ManualProject>(&self.conn, "manual_projects")?;
        let web_references = load_json_table::<WebReference>(&self.conn, "web_references")?;
        let books = load_json_table::<Book>(&self.conn, "books")?;
        let collections = load_json_table::<Collection>(&self.conn, "collections")?;

        let schema_version =
            get_meta(&self.conn, "schema_version").unwrap_or_else(|| "1.0".to_string());
        let last_updated =
            get_meta(&self.conn, "last_updated").unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
        let generated_by =
            get_meta(&self.conn, "generated_by").unwrap_or_else(|| "repoquery".to_string());
        let total_count =
            repositories.len() + manual_projects.len() + web_references.len() + books.len();

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
                    custom_tags_json, fork_ahead, fork_behind,
                    domain, unified_owner_id, discovered_via
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
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
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
        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(lang) = &filter.language {
            conditions.push("primary_language = ?".to_string());
            param_values.push(Box::new(lang.clone()));
        }
        if let Some(archived) = filter.archived {
            conditions.push("archive_status = ?".to_string());
            param_values.push(Box::new(if archived { 1i64 } else { 0i64 }));
        }
        if let Some(min) = filter.min_stars {
            conditions.push("stars >= ?".to_string());
            param_values.push(Box::new(min as i64));
        }
        if let Some(max) = filter.max_stars {
            conditions.push("stars <= ?".to_string());
            param_values.push(Box::new(max as i64));
        }
        if let Some(src) = &filter.source {
            conditions.push("source = ?".to_string());
            param_values.push(Box::new(src.clone()));
        }
        if let Some(tag) = &filter.tag {
            let escaped = tag.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
            conditions.push("custom_tags_json LIKE ? ESCAPE '\\'".to_string());
            param_values.push(Box::new(format!("%\"{}\"%", escaped)));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, metadata_json, classification_json, quality_json, platforms_json,
                    source, added_date, manually_curated, curator_notes, relations_json,
                    custom_tags_json, fork_ahead, fork_behind,
                    domain, unified_owner_id, discovered_via
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
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, Option<String>>(15)?,
            ))
        })?;

        let mut repos: Vec<Repository> = rows
            .map(|r| {
                let (
                    id,
                    meta_j,
                    class_j,
                    qual_j,
                    plat_j,
                    src_j,
                    added,
                    curated,
                    notes,
                    rel_j,
                    tags_j,
                    ahead,
                    behind,
                    domain,
                    unified_owner_id,
                    discovered_via,
                ) = r?;
                repo_from_row(
                    &id,
                    &meta_j,
                    &class_j,
                    &qual_j,
                    &plat_j,
                    &src_j,
                    added,
                    curated,
                    notes,
                    &rel_j,
                    tags_j.as_deref(),
                    ahead,
                    behind,
                    domain,
                    unified_owner_id,
                    discovered_via,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        // Post-filter on fields stored in JSON columns
        if let Some(ref owner) = filter.owner {
            repos.retain(|r| r.metadata.owner.to_lowercase() == owner.to_lowercase());
        }
        if let Some(ref license) = filter.license {
            repos.retain(|r| {
                r.metadata.license_spdx.as_deref().map(|l| l.to_lowercase())
                    == Some(license.to_lowercase())
            });
        }
        if let Some(ref topic) = filter.topic {
            repos.retain(|r| {
                r.metadata
                    .topics
                    .iter()
                    .any(|t| t.to_lowercase() == topic.to_lowercase())
            });
        }
        if let Some(ref query) = filter.search_query {
            let q = query.to_lowercase();
            repos.retain(|r| {
                r.metadata.name.to_lowercase().contains(&q)
                    || r.metadata.description.to_lowercase().contains(&q)
                    || r.metadata
                        .topics
                        .iter()
                        .any(|t| t.to_lowercase().contains(&q))
            });
        }

        // Sort
        match filter.sort {
            SortField::Stars => {
                if filter.order == SortOrder::Asc {
                    repos.sort_by_key(|r| r.metadata.stars);
                } else {
                    repos.sort_by_key(|b| std::cmp::Reverse(b.metadata.stars));
                }
            }
            SortField::Name => {
                if filter.order == SortOrder::Asc {
                    repos.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
                } else {
                    repos.sort_by(|a, b| b.metadata.name.cmp(&a.metadata.name));
                }
            }
            SortField::Updated => {
                if filter.order == SortOrder::Asc {
                    repos.sort_by(|a, b| {
                        a.quality_metrics
                            .last_star_update
                            .cmp(&b.quality_metrics.last_star_update)
                    });
                } else {
                    repos.sort_by(|a, b| {
                        b.quality_metrics
                            .last_star_update
                            .cmp(&a.quality_metrics.last_star_update)
                    });
                }
            }
            SortField::Created => {
                repos.sort_by(|a, b| a.id.cmp(&b.id)); // fallback: sort by id
            }
            SortField::Quality => {
                if filter.order == SortOrder::Asc {
                    repos.sort_by_key(|r| r.quality_metrics.quality_score);
                } else {
                    repos.sort_by(|a, b| {
                        b.quality_metrics
                            .quality_score
                            .cmp(&a.quality_metrics.quality_score)
                    });
                }
            }
        }

        // Limit
        if let Some(limit) = filter.limit {
            repos.truncate(limit);
        }

        Ok(repos)
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

impl GraphStore for SqliteStore {
    fn add_seed(&self, seed: &Seed) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO seeds (id, platform, username, status, depth, max_depth, added_at, completed_at, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                seed.id,
                seed.platform.to_string(),
                seed.username,
                serde_json::to_string(&seed.status)?,
                seed.depth as i64,
                seed.max_depth as i64,
                seed.added_at,
                seed.completed_at,
                seed.error_message,
            ],
        )?;
        Ok(())
    }

    fn list_seeds(&self) -> Result<Vec<Seed>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, platform, username, status, depth, max_depth, added_at, completed_at, error_message FROM seeds ORDER BY added_at DESC",
        )?;
        let seeds = stmt
            .query_map([], |row| {
                Ok(Seed {
                    id: row.get(0)?,
                    platform: row
                        .get::<_, String>(1)?
                        .parse::<PlatformKind>()
                        .unwrap_or(PlatformKind::GitHub),
                    username: row.get(2)?,
                    status: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(SeedStatus::Pending),
                    depth: row.get::<_, i64>(4)? as u32,
                    max_depth: row.get::<_, i64>(5)? as u32,
                    added_at: row.get(6)?,
                    completed_at: row.get(7)?,
                    error_message: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(seeds)
    }

    fn get_seed(&self, id: &str) -> Result<Option<Seed>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, platform, username, status, depth, max_depth, added_at, completed_at, error_message FROM seeds WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Seed {
                id: row.get(0)?,
                platform: row
                    .get::<_, String>(1)?
                    .parse::<PlatformKind>()
                    .unwrap_or(PlatformKind::GitHub),
                username: row.get(2)?,
                status: serde_json::from_str(&row.get::<_, String>(3)?)
                    .unwrap_or(SeedStatus::Pending),
                depth: row.get::<_, i64>(4)? as u32,
                max_depth: row.get::<_, i64>(5)? as u32,
                added_at: row.get(6)?,
                completed_at: row.get(7)?,
                error_message: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn update_seed_status(&self, id: &str, status: &str) -> Result<()> {
        let json = serde_json::to_string(status)?;
        self.conn.execute(
            "UPDATE seeds SET status = ?1 WHERE id = ?2",
            params![json, id],
        )?;
        Ok(())
    }

    fn add_traversal_edge(&self, edge: &TraversalEdge) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO traversal_edges (seed_id, from_user_id, to_user_id, relation_type, depth, discovered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                edge.seed_id,
                edge.from_user_id,
                edge.to_user_id,
                serde_json::to_string(&edge.relation_type)?,
                edge.depth as i64,
                edge.discovered_at,
            ],
        )?;
        Ok(())
    }

    fn traversal_edges_by_seed(&self, seed_id: &str) -> Result<Vec<TraversalEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, seed_id, from_user_id, to_user_id, relation_type, depth, discovered_at
             FROM traversal_edges WHERE seed_id = ?1 ORDER BY depth ASC, discovered_at ASC",
        )?;
        let edges = stmt
            .query_map(params![seed_id], |row| {
                Ok(TraversalEdge {
                    id: row.get(0)?,
                    seed_id: row.get(1)?,
                    from_user_id: row.get(2)?,
                    to_user_id: row.get(3)?,
                    relation_type: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(EdgeType::Follows),
                    depth: row.get::<_, i64>(5)? as u32,
                    discovered_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    fn traversal_edges_by_user(&self, user_id: &str) -> Result<Vec<TraversalEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, seed_id, from_user_id, to_user_id, relation_type, depth, discovered_at
             FROM traversal_edges WHERE from_user_id = ?1 OR to_user_id = ?1 ORDER BY depth ASC",
        )?;
        let edges = stmt
            .query_map(params![user_id], |row| {
                Ok(TraversalEdge {
                    id: row.get(0)?,
                    seed_id: row.get(1)?,
                    from_user_id: row.get(2)?,
                    to_user_id: row.get(3)?,
                    relation_type: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(EdgeType::Follows),
                    depth: row.get::<_, i64>(5)? as u32,
                    discovered_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(edges)
    }

    fn has_user_been_visited(&self, seed_id: &str, user_id: &str, max_depth: u32) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM traversal_edges WHERE seed_id = ?1 AND to_user_id = ?2 AND depth <= ?3",
            params![seed_id, user_id, max_depth as i64],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn add_fgat_token(&self, token: &FgatToken) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO fgat_tokens (id, platform, token_hash, status, requests_used, rate_limit_limit, rate_limit_remaining, rate_limit_reset_at, last_used_at, added_at, expires_at, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                token.id,
                token.platform.to_string(),
                token.token_hash,
                serde_json::to_string(&token.status)?,
                token.requests_used as i64,
                token.rate_limit_limit,
                token.rate_limit_remaining,
                token.rate_limit_reset_at,
                token.last_used_at,
                token.added_at,
                token.expires_at,
                token.notes,
            ],
        )?;
        Ok(())
    }

    fn list_fgat_tokens(&self) -> Result<Vec<FgatToken>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, platform, token_hash, status, requests_used, rate_limit_limit, rate_limit_remaining, rate_limit_reset_at, last_used_at, added_at, expires_at, notes
             FROM fgat_tokens ORDER BY added_at DESC",
        )?;
        let tokens = stmt
            .query_map([], |row| {
                Ok(FgatToken {
                    id: row.get(0)?,
                    platform: row
                        .get::<_, String>(1)?
                        .parse::<PlatformKind>()
                        .unwrap_or(PlatformKind::GitHub),
                    token_hash: row.get(2)?,
                    raw_token: None,
                    status: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(TokenStatus::Available),
                    requests_used: row.get::<_, i64>(4)? as u64,
                    rate_limit_limit: row.get(5)?,
                    rate_limit_remaining: row.get(6)?,
                    rate_limit_reset_at: row.get(7)?,
                    last_used_at: row.get(8)?,
                    added_at: row.get(9)?,
                    expires_at: row.get(10)?,
                    notes: row.get(11)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(tokens)
    }

    fn update_fgat_token_status(&self, id: &str, status: &str) -> Result<()> {
        let json = serde_json::to_string(status)?;
        self.conn.execute(
            "UPDATE fgat_tokens SET status = ?1 WHERE id = ?2",
            params![json, id],
        )?;
        Ok(())
    }

    fn delete_fgat_token(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM fgat_tokens WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    fn add_domain(&self, domain: &NormalizedDomain) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO normalized_domains (domain, canonical_domain, platform, is_verified, confidence_score, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                domain.domain,
                domain.canonical_domain,
                domain.platform.as_ref().map(|p| p.to_string()),
                if domain.is_verified { 1i64 } else { 0i64 },
                domain.confidence_score,
                domain.created_at,
                domain.updated_at,
            ],
        )?;
        Ok(())
    }

    fn get_domain(&self, domain: &str) -> Result<Option<NormalizedDomain>> {
        let mut stmt = self.conn.prepare(
            "SELECT domain, canonical_domain, platform, is_verified, confidence_score, created_at, updated_at FROM normalized_domains WHERE domain = ?1",
        )?;
        let mut rows = stmt.query(params![domain])?;
        if let Some(row) = rows.next()? {
            Ok(Some(NormalizedDomain {
                domain: row.get(0)?,
                canonical_domain: row.get(1)?,
                platform: row
                    .get::<_, Option<String>>(2)?
                    .and_then(|s| s.parse().ok()),
                is_verified: row.get::<_, i64>(3)? != 0,
                confidence_score: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn add_unified_identity(&self, identity: &UnifiedIdentity) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO unified_identities (id, canonical_username, primary_domain, confidence_score, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                identity.id,
                identity.canonical_username,
                identity.primary_domain,
                identity.confidence_score,
                identity.created_at,
            ],
        )?;
        Ok(())
    }

    fn add_identity_alias(&self, alias: &IdentityAlias) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO identity_aliases (unified_id, platform, username, verified, verification_method)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                alias.unified_id,
                alias.platform.to_string(),
                alias.username,
                if alias.verified { 1i64 } else { 0i64 },
                alias.verification_method,
            ],
        )?;
        Ok(())
    }

    fn find_identity_by_alias(&self, platform: &str, username: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT unified_id FROM identity_aliases WHERE platform = ?1 AND username = ?2",
            params![platform, username],
            |row| row.get(0),
        );
        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn add_domain_repository(&self, repo: &DomainRepository) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO domain_repositories (id, domain, repo_path, full_name, platform, metadata_json, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                repo.id,
                repo.domain,
                repo.repo_path,
                repo.full_name,
                repo.platform.to_string(),
                repo.metadata_json,
                repo.indexed_at,
            ],
        )?;
        Ok(())
    }

    fn domain_repositories_by_domain(&self, domain: &str) -> Result<Vec<DomainRepository>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, domain, repo_path, full_name, platform, metadata_json, indexed_at
             FROM domain_repositories WHERE domain = ?1 ORDER BY full_name ASC",
        )?;
        let repos = stmt
            .query_map(params![domain], |row| {
                Ok(DomainRepository {
                    id: row.get(0)?,
                    domain: row.get(1)?,
                    repo_path: row.get(2)?,
                    full_name: row.get(3)?,
                    platform: row
                        .get::<_, String>(4)?
                        .parse::<PlatformKind>()
                        .unwrap_or(PlatformKind::GitHub),
                    metadata_json: row.get(5)?,
                    indexed_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(repos)
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
    domain: Option<String>,
    unified_owner_id: Option<String>,
    discovered_via: Option<String>,
) -> Result<Repository> {
    let custom_tags: Vec<String> = tags_j
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let json = format!(
        r#"{{"id":{},"metadata":{},"classification":{},"quality_metrics":{},"platforms":{},"source":{},"added_date":{},"manually_curated":{},"curator_notes":{},"relations":{},"custom_tags":{},"fork_ahead":{},"fork_behind":{},"domain":{},"unified_owner_id":{},"discovered_via":{}}}"#,
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
        fork_ahead
            .map(|v| v as u32)
            .map_or("null".to_string(), |v| v.to_string()),
        fork_behind
            .map(|v| v as u32)
            .map_or("null".to_string(), |v| v.to_string()),
        serde_json::to_string(&domain)?,
        serde_json::to_string(&unified_owner_id)?,
        serde_json::to_string(&discovered_via)?,
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
            serde_json::from_str(json).map_err(|e| anyhow::anyhow!("loading {}: {}", table, e))
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
    use rq_core::{
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
            domain: None,
            unified_owner_id: None,
            discovered_via: None,
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
        assert!(results
            .iter()
            .all(|r| r.metadata.primary_language == "Rust"));
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

        let mut c = rq_core::Collection::new("c1".to_string(), "My List".to_string());
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
        repo.relations = vec![rq_core::Relation::Owned, rq_core::Relation::Starred];
        store.upsert_repo(&repo).unwrap();

        let got = store.get_repo("r1").unwrap().unwrap();
        assert_eq!(
            got.relations,
            vec![rq_core::Relation::Owned, rq_core::Relation::Starred]
        );
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

        let filter = RepoFilter {
            tag: Some("ml".to_string()),
            ..Default::default()
        };
        let results = store.list_repos(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "r1");
    }
}
