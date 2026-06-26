use anyhow::Result;
use rq_core::{Book, CanonicalData, Collection, ManualProject, Repository, WebReference};

/// Sort field for repository queries
#[derive(Debug, Default, Clone, PartialEq)]
pub enum SortField {
    #[default]
    Stars,
    Name,
    Updated,
    Created,
    Quality,
}

/// Sort direction
#[derive(Debug, Default, Clone, PartialEq)]
pub enum SortOrder {
    #[default]
    Desc,
    Asc,
}

/// Abstraction over persistence backends.
pub trait RepoStore {
    fn load_all(&self) -> Result<CanonicalData>;
    fn save_all(&self, data: &CanonicalData) -> Result<()>;
    fn get_repo(&self, id: &str) -> Result<Option<Repository>>;
    fn upsert_repo(&self, repo: &Repository) -> Result<()>;
    fn list_repos(&self, filter: &RepoFilter) -> Result<Vec<Repository>>;
    fn count_repos(&self, filter: &RepoFilter) -> Result<usize>;
    fn delete_repo(&self, id: &str) -> Result<bool>;

    fn list_collections(&self) -> Result<Vec<Collection>>;
    fn get_collection(&self, id: &str) -> Result<Option<Collection>>;
    fn save_collection(&self, collection: &Collection) -> Result<()>;
    fn delete_collection(&self, id: &str) -> Result<bool>;
}

/// Filter criteria for repository queries.
#[derive(Debug, Default, Clone)]
pub struct RepoFilter {
    pub language: Option<String>,
    pub archived: Option<bool>,
    pub min_stars: Option<u32>,
    pub max_stars: Option<u32>,
    pub source: Option<String>,
    pub tag: Option<String>,
    pub owner: Option<String>,
    pub license: Option<String>,
    pub topic: Option<String>,
    pub updated_after: Option<String>,
    pub created_before: Option<String>,
    pub sort: SortField,
    pub order: SortOrder,
    pub limit: Option<usize>,
    pub search_query: Option<String>,
}

// Suppress unused-import warnings — these are part of the public trait surface.
const _: fn() = || {
    let _: Option<Book> = None;
    let _: Option<Collection> = None;
    let _: Option<ManualProject> = None;
    let _: Option<WebReference> = None;
    let _: Option<rq_core::Seed> = None;
    let _: Option<rq_core::TraversalEdge> = None;
    let _: Option<rq_core::FgatToken> = None;
    let _: Option<rq_core::NormalizedDomain> = None;
    let _: Option<rq_core::UnifiedIdentity> = None;
    let _: Option<rq_core::IdentityAlias> = None;
    let _: Option<rq_core::DomainRepository> = None;
};

/// Abstraction over graph expansion storage.
pub trait GraphStore {
    fn add_seed(&self, seed: &rq_core::Seed) -> Result<()>;
    fn list_seeds(&self) -> Result<Vec<rq_core::Seed>>;
    fn get_seed(&self, id: &str) -> Result<Option<rq_core::Seed>>;
    fn update_seed_status(&self, id: &str, status: &str) -> Result<()>;

    fn add_traversal_edge(&self, edge: &rq_core::TraversalEdge) -> Result<()>;
    fn traversal_edges_by_seed(&self, seed_id: &str) -> Result<Vec<rq_core::TraversalEdge>>;
    fn traversal_edges_by_user(&self, user_id: &str) -> Result<Vec<rq_core::TraversalEdge>>;
    fn has_user_been_visited(&self, seed_id: &str, user_id: &str, max_depth: u32) -> Result<bool>;

    fn add_fgat_token(&self, token: &rq_core::FgatToken) -> Result<()>;
    fn list_fgat_tokens(&self) -> Result<Vec<rq_core::FgatToken>>;
    fn update_fgat_token_status(&self, id: &str, status: &str) -> Result<()>;
    fn delete_fgat_token(&self, id: &str) -> Result<()>;

    fn add_domain(&self, domain: &rq_core::NormalizedDomain) -> Result<()>;
    fn get_domain(&self, domain: &str) -> Result<Option<rq_core::NormalizedDomain>>;

    fn add_unified_identity(&self, identity: &rq_core::UnifiedIdentity) -> Result<()>;
    fn add_identity_alias(&self, alias: &rq_core::IdentityAlias) -> Result<()>;
    fn find_identity_by_alias(&self, platform: &str, username: &str) -> Result<Option<String>>;

    fn add_domain_repository(&self, repo: &rq_core::DomainRepository) -> Result<()>;
    fn domain_repositories_by_domain(&self, domain: &str)
        -> Result<Vec<rq_core::DomainRepository>>;
}
