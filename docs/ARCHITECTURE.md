# OmniDatum Architecture Documentation

## Overview

The OmniDatum Processor is a high-performance Rust-based CLI tool for managing and synchronizing GitHub starred repository documentation. It processes 800+ repositories with automated validation, external data sync, cross-reference tracking, and multi-format output generation.

## Design Philosophy

### Core Principles

1. **Single Source of Truth**: All repository metadata derives from canonical YAML files
2. **Unidirectional Data Flow**: Clean architecture prevents circular dependencies
3. **Format Independence**: Each output format generated from shared data models
4. **Validation-First**: Build quality checks into the generation process
5. **Extensibility**: Design for future platforms and data sources
6. **Type Safety**: Leverage Rust's type system for correctness
7. **Performance**: Sub-second processing with intelligent caching

## System Architecture

### High-Level Components

```
┌─────────────────┐
│  CLI Interface  │ (clap + tokio)
└────────┬────────┘
         │
         ├─────→ Configure  (credential setup)
         ├─────→ Sync       (GitHub API → repositories.yml)
         ├─────→ Parse      (LIST.md → repositories.yml) [legacy]
         ├─────→ Merge      (repos + manual + refs + books)
         ├─────→ Validate   (9 quality rules: E001-E008)
         ├─────→ Generate   (LIST/TABLE/ARCHIVE *.md)
         ├─────→ Status     (sync health check)
         └─────→ Stats      (analytics)
         
┌─────────────────────────────────────────────────────────┐
│              Core Library (lib.rs)                      │
├─────────────────────────────────────────────────────────┤
│ config/          │ sync/           │ models/            │
│ - settings       │ - orchestrator  │ - repository       │
│ - credentials    │ - adapters/     │ - platform         │
│                  │   - github      │ - manual           │
│ validators/      │   - codeberg*   │ - reference        │
│ - framework      │   - gitlab*     │ - book             │
│ - rules          │ - cache         │ - canonical        │
│ - external_data  │ - progress      │ - sync_metadata    │
│                  │                 │                    │
│ parsers/         │ generators/     │ cross_refs/        │
│ - list_parser    │ - markdown      │ - graph            │
│                  │                 │ - navigator        │
│ merge/           │ readme_updater/ │                    │
│ - merger         │                 │                    │
└─────────────────────────────────────────────────────────┘

* Stub adapters for future implementation
```

### Unidirectional Data Flow

```
External Sources          Canonical Storage           Generated Outputs
───────────────          ──────────────────          ─────────────────
                                                     
GitHub API ────┐                                     ┌──→ LIST.md (active)
               │                                     │
Codeberg* ─────┼──→ Sync ──→ repositories.yml       │
               │              (single source)        ├──→ TABLE.md (active)
GitLab* ───────┘                    │                │
                                    │                ├──→ ARCHIVE.md
Legacy:                             │                │
LIST.md ────────→ Parse ────────────┤                └──→ ARCHIVE_TABLE.md
                                    │
                                    ├──→ Merge ──→        ⚠️  NO FEEDBACK
manual_additions.yml ───────────────┤   merged_data.yml      (Read-only)
web_references.yml ─────────────────┤        │
books.yml ──────────────────────────┘        │
                                             │
                                        Validate (9 rules)
                                             │
                                             ├──→ validation_report.json
                                             │
                                        Generate (Tera templates)
                                             │
                                             └──→ Multiple formats
```

**Key Architectural Decision**: The system enforces unidirectional flow. Generated markdown documents (LIST.md, TABLE.md) are **output only** and never used as input, preventing circular dependencies.

## Project Structure

The project is organized as a 7-crate Cargo workspace for modularity and reusability:

```
crates/
├── od-core/                    # Pure data types and business logic
│   ├── src/
│   │   ├── lib.rs              # Public API re-exports (30+ explicit types)
│   │   ├── models/             # Data structures
│   │   │   ├── mod.rs
│   │   │   ├── repository.rs   # Core repo model
│   │   │   ├── relation.rs     # Phase 3: Relation enum
│   │   │   ├── collection.rs   # Phase 3: Collection struct
│   │   │   ├── canonical.rs    # Canonical data model
│   │   │   ├── platform.rs     # Platform enum + migration
│   │   │   ├── manual.rs       # Manual additions model
│   │   │   ├── reference.rs    # Web reference model
│   │   │   ├── book.rs         # Book model
│   │   │   └── sync_metadata.rs # Sync state tracking
│   │   ├── config/             # Configuration management
│   │   │   ├── mod.rs
│   │   │   ├── settings.rs     # TOML config loading
│   │   │   └── credentials.rs  # Multi-source credential management
│   │   ├── validators/         # Quality assurance
│   │   │   ├── mod.rs
│   │   │   ├── framework.rs    # Validation engine
│   │   │   ├── rules.rs        # E001–E008 rules
│   │   │   └── external_data_rules.rs # External data validation
│   │   ├── parsers/            # Data extraction
│   │   │   ├── mod.rs
│   │   │   └── list_parser.rs  # Legacy LIST.md parser
│   │   ├── merge.rs            # Multi-source data merger
│   │   └── readme_updater.rs   # README stats updater
│   └── Cargo.toml
│
├── od-store/                   # Trait-based persistence layer (Phase 2)
│   ├── src/
│   │   ├── lib.rs              # Public API (RepoStore trait, open_store)
│   │   ├── traits.rs           # RepoStore trait + RepoFilter
│   │   ├── sqlite.rs           # SqliteStore implementation
│   │   ├── yaml.rs             # YamlStore implementation
│   │   └── store.rs            # open_store() convenience function
│   └── Cargo.toml
│
├── od-sync/                    # External data synchronization
│   ├── src/
│   │   ├── lib.rs              # Public API
│   │   ├── orchestrator.rs     # SyncOrchestrator with concurrent sync (Phase 3)
│   │   ├── cache.rs            # ETag-based caching
│   │   ├── progress.rs         # Progress tracking
│   │   ├── client.rs           # HTTP client
│   │   └── adapters/           # Data source adapters
│   │       ├── mod.rs          # DataSourceAdapter trait
│   │       ├── github.rs       # GitHub API with Phase 3 methods
│   │       ├── gitlab.rs       # Stub
│   │       └── codeberg.rs     # Stub
│   └── Cargo.toml
│
├── od-validate/                # Validation framework
│   ├── src/
│   │   ├── lib.rs              # Public API
│   │   ├── framework.rs        # Validation engine
│   │   ├── rules.rs            # Built-in rules
│   │   └── external_data_rules.rs # Sync validation
│   └── Cargo.toml
│
├── od-generate/                # Output generation
│   ├── src/
│   │   ├── lib.rs              # Public API
│   │   └── markdown.rs         # Tera-based generator
│   └── Cargo.toml
│
├── od-graph/                   # Cross-reference tracking
│   ├── src/
│   │   ├── lib.rs              # Public API
│   │   ├── graph.rs            # Graph structure
│   │   └── navigator.rs        # Navigation links
│   └── Cargo.toml
│
└── od-cli/                     # Binary crate
    ├── src/
    │   ├── main.rs             # CLI dispatch layer
    │   ├── commands/           # Command handlers
    │   │   ├── mod.rs
    │   │   ├── parse.rs
    │   │   ├── validate.rs
    │   │   ├── generate.rs
    │   │   ├── merge.rs
    │   │   ├── stats.rs
    │   │   ├── configure.rs
    │   │   ├── migrate_credentials.rs
    │   │   ├── sync_cmd.rs
    │   │   ├── collections.rs  # Phase 3: Collections command
    │   │   └── status.rs
    │   └── lib.rs              # CLI library exports
    ├── tests/
    │   ├── integration_test.rs
    │   └── mock_adapter.rs
    └── Cargo.toml
```

**Total**: ~7,800 lines of production code across 7 crates

### Crate Dependencies

```
od-cli (binary)
  ├── od-core (models, config, validators, parsers, merge)
  ├── od-store (YAML I/O)
  ├── od-sync (GitHub API, adapters, cache)
  ├── od-validate (validation framework)
  ├── od-generate (Tera templates)
  └── od-graph (petgraph relationships)

od-sync
  └── od-core (models, config)

od-validate
  └── od-core (models)

od-generate
  └── od-core (models)

od-graph
  └── od-core (models)

od-store
  └── od-core (models)
```

**Design**: Acyclic dependency graph. od-core has no dependencies on other crates.

## Core Components

### 1. Storage Layer (`store/`)

**Purpose**: Abstract repository persistence with pluggable backends

**Trait-Based Design** (Phase 2):
```rust
pub trait RepoStore: Send + Sync {
    async fn load_all(&self) -> Result<Vec<Repository>>;
    async fn save_all(&self, repos: Vec<Repository>) -> Result<()>;
    async fn get_repo(&self, id: &str) -> Result<Option<Repository>>;
    async fn upsert_repo(&self, repo: Repository) -> Result<()>;
    async fn list_repos(&self, filter: &RepoFilter) -> Result<Vec<Repository>>;
    async fn count_repos(&self, filter: &RepoFilter) -> Result<usize>;
    async fn delete_repo(&self, id: &str) -> Result<()>;
    
    // Phase 3: Collection methods
    async fn create_collection(&self, collection: Collection) -> Result<()>;
    async fn get_collection(&self, id: &str) -> Result<Option<Collection>>;
    async fn list_collections(&self) -> Result<Vec<Collection>>;
    async fn delete_collection(&self, id: &str) -> Result<()>;
}

pub struct RepoFilter {
    pub language: Option<String>,
    pub archived: Option<bool>,
    pub min_stars: Option<u32>,
    pub source: Option<RepositorySource>,
}
```

**Implementations**:
- **SqliteStore**: rusqlite-backed with JSON columns for complex fields, indexed scalar columns for queries, collections table (Phase 3)
- **YamlStore**: Backward-compatible wrapper around existing YAML I/O, collections via CanonicalData (Phase 3)

**Convenience Function**:
```rust
pub fn open_store(path: &str) -> Result<Box<dyn RepoStore>> {
    // Auto-detects backend from file extension (.db → SQLite, .yml → YAML)
}
```

**CLI Commands**:
- `import --from <yaml> --to <db>` — Migrate YAML data to SQLite
- `export --from <db> --to <yaml>` — Export SQLite data to YAML
- `collections list` — Display all collections (Phase 3)
- `collections create <name>` — Create new collection (Phase 3)
- `collections show <name>` — Display collection details (Phase 3)
- `collections add <collection> <repo>` — Add repository to collection (Phase 3)
- `collections remove <collection> <repo>` — Remove repository from collection (Phase 3)
- `collections delete <name>` — Delete collection (Phase 3)

### 2. Configuration System (`config/`)

**Purpose**: Manage application settings and secure credentials

**Key Components**:
```rust
pub struct OmnidatumConfig {
    pub sync: SyncConfig,              // Sync settings
    pub credentials: CredentialsConfig, // Credential management
    pub validation: ValidationConfig,   // Validation rules
    pub generation: GenerationConfig,   // Output settings
}

pub struct SyncConfig {
    pub enabled: bool,
    pub interval_hours: u32,
    pub parallel_workers: u8,           // 1-10
    pub cache_ttl_hours: u32,           // 1-168 hours
    pub rate_limit_buffer: u32,         // 0-1000
    pub request_timeout_secs: u64,
}
```

**Credential Storage**:
- Environment variables (GITHUB_TOKEN, GH_TOKEN)
- Secure file (~/.config/omnidatum/credentials with 0600 permissions)
- OS keychain (macOS Keychain, Linux secret-tool)

### 2. Sync System (`sync/`)

**Purpose**: Fetch and cache repository metadata from external sources

**Architecture**:
```rust
pub trait DataSourceAdapter {
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository>;
    async fn check_connection(&self) -> Result<()>;
    fn source_name(&self) -> &str;
}

pub struct SyncOrchestrator {
    config: OmnidatumConfig,
    cache: SyncCache,
    progress: ProgressTracker,
}
```

**GitHub GraphQL Client** (Phase 6):
```rust
pub struct GraphQLClient {
    token: String,
    client: reqwest::Client,
}

impl GraphQLClient {
    pub async fn fetch_starred_repos(&self, username: &str, after: Option<String>) -> Result<GraphQLResponse> {
        // Paginated query: 100 repos per request
        // Returns: nameWithOwner, url, forkParent { nameWithOwner, url }
    }
    
    pub async fn fetch_owned_repos(&self, username: &str, after: Option<String>) -> Result<GraphQLResponse> {
        // Paginated query for user-owned repositories
    }
    
    pub async fn fetch_forked_repos(&self, username: &str, after: Option<String>) -> Result<GraphQLResponse> {
        // Paginated query for user-forked repositories
    }
    
    pub async fn fetch_watched_repos(&self, username: &str, after: Option<String>) -> Result<GraphQLResponse> {
        // Paginated query for user-watched repositories
    }
}
```

**Performance Improvement**: GraphQL bulk fetch reduces API calls by 100x (100 repos per request vs 1 per REST call).

**Concurrent Sync** (Phase 3):
```rust
pub async fn sync_all(&self, repos: Vec<Repository>) -> Result<SyncResult> {
    let semaphore = Arc::new(Semaphore::new(self.config.sync.parallel_workers));
    let mut join_set = JoinSet::new();
    
    for repo in repos {
        let permit = semaphore.acquire().await?;
        join_set.spawn(async move {
            let _permit = permit;
            self.sync_repository(&repo).await
        });
    }
    
    // Collect results with per-relation tracking
}

pub async fn sync_by_relation(&self, relation: Relation) -> Result<Vec<Repository>> {
    // Fetch repositories by relation type
}
```

**GitHubAdapter Expansion** (Phase 3):
```rust
impl GitHubAdapter {
    pub async fn fetch_user_repos(&self, username: &str) -> Result<Vec<Repository>>;
    pub async fn fetch_user_forks(&self, username: &str) -> Result<Vec<Repository>>;
    pub async fn fetch_watched_repos(&self, username: &str) -> Result<Vec<Repository>>;
    pub async fn fetch_org_repos(&self, org: &str) -> Result<Vec<Repository>>;
}
```

**SyncResult** (Phase 3):
```rust
pub struct SyncResult {
    pub total_synced: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub by_relation: HashMap<String, usize>,  // Counts per relation type
    pub errors: Vec<SyncError>,
}
```

**Features**:
- GitHub API integration with octocrab (REST) and reqwest (GraphQL)
- ETag-based caching with configurable TTL (default 24 hours)
- Rate limit management with configurable buffer (default 500)
- Progress tracking with indicatif
- Selective sync (specific repositories)
- Dry-run mode (preview changes)
- Verbose logging (per-repository details)
- Parallel workers with bounded concurrency (default: 3)

**Sync Flow**:
1. Check cache freshness
2. If stale/missing, fetch from GitHub API (REST or GraphQL)
3. Merge sync data (preserve manual curation fields)
4. Update cache and canonical data
5. Report progress and results by relation type

### 3. Data Models (`models/`)

**Relation Enum** (`relation.rs`, Phase 3)
```rust
pub enum Relation {
    Starred,        // User starred the repository
    Owned,          // User owns the repository
    Forked,         // User forked the repository
    Watching,       // User is watching the repository
    OrgMember,      // User is organization member
    Contributed,    // User contributed to the repository
    ManuallyAdded,  // Manually added by curator
}
```

**Collection Struct** (`collection.rs`, Phase 3)
```rust
pub struct Collection {
    pub id: String,                           // Unique identifier
    pub name: String,                         // Display name
    pub description: Option<String>,          // Collection description
    pub repository_ids: Vec<String>,          // Member repositories
    pub created_at: String,                   // RFC3339 timestamp
    pub updated_at: String,                   // RFC3339 timestamp
}
```

**Repository Model** (`repository.rs`)
```rust
pub struct Repository {
    pub id: String,                           // Unique identifier
    pub platforms: Vec<PlatformInfo>,         // Multi-platform tracking
    pub metadata: RepositoryMetadata,         // Name, description, stars
    pub classification: RepositoryClassification, // Language, topics
    pub quality_metrics: QualityMetrics,      // Archive status, scoring
    pub source: RepositorySource,             // GitHubStars, Manual
    pub manually_curated: bool,               // Preserved during sync
    pub curator_notes: Option<String>,        // Preserved during sync
    pub relations: Vec<Relation>,             // Phase 3: Relationship types (default: empty)
    pub fork_parent: Option<String>,          // Phase 6: Fork parent nameWithOwner (default: None)
    pub fork_parent_url: Option<String>,      // Phase 6: Fork parent URL (default: None)
}
```

**Fork Parent Tracking** (Phase 6):
- `fork_parent` field stores upstream repository identifier (e.g., "torvalds/linux")
- `fork_parent_url` field stores upstream repository URL
- Backward-compatible via `serde(default)` — existing data loads without fork parent fields
- `merge_sync_data()` propagates fork parent from GraphQL sync results
- Enables fork lineage tracking and upstream contribution analysis

**Canonical Container** (`canonical.rs`)
```rust
pub struct CanonicalData {
    pub repositories: Vec<Repository>,        // All repository data
    pub manual_projects: Vec<ManualProject>, // Curated additions
    pub web_references: Vec<WebReference>,   // External links
    pub books: Vec<Book>,                    // Learning resources
    pub collections: Vec<Collection>,         // Phase 3: User-defined groupings (default: empty)
    pub statistics: Option<Statistics>,      // Computed metrics
    pub last_updated: String,                 // RFC3339 timestamp
}
```

### 4. Validation System (`validators/`)

**Framework Architecture**:
```rust
pub trait ValidationRule: Send + Sync {
    fn name(&self) -> &str;
    fn default_severity(&self) -> Severity;
    fn check(&self, data: &CanonicalData) -> ValidationResult;
}

pub struct Validator {
    rules: Vec<Box<dyn ValidationRule>>,
}
```

**Built-in Rules (9 total)**:
- **E001**: `NoDuplicateReposRule` - Prevents duplicate repository URLs
- **E002**: `MissingLicenseRule` - Flags repos without license info
- **E003**: `ValidUrlsRule` - Validates URL formats
- **E004**: `ReadmeCrossReferenceRule` - Checks cross-reference integrity
- **E005**: `PlatformMigrationRule` - Verifies migration completeness
- **E006**: `MissingMetadataRule` - Identifies missing descriptions/owners
- **E007**: `ExternalDataConsistencyRule` - Validates synced data (stars < 1M, archive status matches)
- **E008**: `DuplicateRepositoryNameRule` - Detects duplicate full_names
- **Stale Content** - Flags content >2 years old

### 5. Cross-Reference System (`cross_refs/`)

**Graph Structure**:
```rust
pub enum NodeType {
    Repository(String),
    WebReference(String),
    Book(String),
    ReadmeSection(String),
}

pub enum RelationType {
    Implements,      // Repository implements concept
    References,      // Content references repository
    Alternative,     // Alternative implementation
    Prerequisite,    // Required knowledge/tool
}
```

**Implementation**: Uses `petgraph::DiGraph` for efficient bidirectional traversal

### 6. Template System (`generators/`)

**Tera Integration**:
```rust
pub struct MarkdownGenerator {
    tera: Tera,                              // Template engine
    templates_dir: PathBuf,                  // Template directory
}

pub struct TemplateContext {
    pub total_count: usize,                  // Repository count
    pub languages: Vec<LanguageSection>,     // Grouped by language
    pub platforms: HashMap<String, usize>,   // Platform statistics
    pub include_stats: bool,                 // Include footer stats
}
```

**Per-Collection Generation** (Phase 6):
```rust
impl MarkdownGenerator {
    pub fn generate_list_for_collection(
        &self,
        collection: &Collection,
        repos: &[Repository],
    ) -> Result<String> {
        // Generate LIST.md for specific collection
        // Output to generated/<collection-id>/LIST.md
    }
    
    pub fn generate_table_for_collection(
        &self,
        collection: &Collection,
        repos: &[Repository],
    ) -> Result<String> {
        // Generate TABLE.md for specific collection
        // Output to generated/<collection-id>/TABLE.md
    }
}
```

**CLI Integration** (Phase 6):
```bash
# Generate for specific collection
cargo run -- generate --collection my-collection

# Outputs:
# generated/my-collection/LIST.md
# generated/my-collection/TABLE.md
```

**Features**:
- Filtered repository lists per collection
- Separate output directories per collection
- Same template engine and formatting as global generation
- Enables curated documentation per topic/domain

### 7. Terminal User Interface (`tui/`)

**Purpose**: Interactive browsing and exploration of repository data

**Architecture**:
```rust
pub struct TuiApp {
    store: Box<dyn RepoStore>,
    current_view: ViewType,
    state: AppState,
}

pub enum ViewType {
    RepoList,
    RepoDetail,
    Collections,
    CollectionDetail,
    Stats,
}

pub struct AppState {
    repos: Vec<Repository>,
    collections: Vec<Collection>,
    selected_index: usize,
    search_query: String,
    language_filter: Option<String>,
    relation_filter: Option<Relation>,
}
```

**Features**:
- **RepoList View**: Filterable table with j/k navigation, / search, Enter to detail
- **RepoDetail View**: Full metadata display (stars, description, license, topics, fork parent)
- **Collections View**: List of user-defined groupings with member counts
- **CollectionDetail View**: Collection members with same filtering as RepoList
- **Stats View**: Dashboard with repository statistics (total, active, archived, by language)
- **Filter Cycling**: l/r keys cycle through language and relation type filters
- **Panic Safety**: Custom panic hook restores terminal state on crash
- **Read-Only**: All operations read from store; collection management hints to use CLI

**Key Bindings**:
```
j/k       Navigate up/down
/         Search repositories
Enter     View repository details
Esc       Return to previous view
q         Quit application
1/2/3     Switch between views
l/r       Cycle through filters
```

**Dependencies**:
- `ratatui` (0.28) — Terminal UI framework
- `crossterm` (0.28) — Terminal backend (Windows/Unix)

## 8. Error Handling Strategy (Phase 4)

### Typed Errors Per Crate

Each library crate defines its own error type using `thiserror`:

```rust
// od-core/src/lib.rs
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Config error: {0}")]
    Config(String),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

// od-store/src/lib.rs
#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    
    #[error("Core error: {0}")]
    Core(#[from] od_core::CoreError),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("{0}")]
    Other(String),
}

// od-sync/src/lib.rs
#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error("Core error: {0}")]
    Core(#[from] od_core::CoreError),
    
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Repository not found")]
    RepositoryNotFound,
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
}

// od-validate/src/lib.rs
#[derive(thiserror::Error, Debug)]
pub enum ValidateError {
    #[error("Core error: {0}")]
    Core(#[from] od_core::CoreError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("{0}")]
    Other(String),
}

// od-generate/src/lib.rs
#[derive(thiserror::Error, Debug)]
pub enum GenerateError {
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Core error: {0}")]
    Core(#[from] od_core::CoreError),
}

// od-graph/src/lib.rs
#[derive(thiserror::Error, Debug)]
pub enum GraphError {
    #[error("Core error: {0}")]
    Core(#[from] od_core::CoreError),
    
    #[error("{0}")]
    Other(String),
}
```

### CLI Error Conversion

The CLI layer (`od-cli`) uses `anyhow::Result` and converts typed errors:

```rust
// od-cli/src/commands/sync_cmd.rs
pub async fn execute(args: SyncArgs) -> anyhow::Result<()> {
    let orchestrator = SyncOrchestrator::new(config)
        .map_err(|e| anyhow::anyhow!("Sync setup failed: {}", e))?;
    
    let result = orchestrator.sync_all(repos)
        .await
        .map_err(|e| anyhow::anyhow!("Sync failed: {}", e))?;
    
    Ok(())
}
```

**Rationale**: Library crates use typed errors for composability and error recovery. CLI layer converts to `anyhow::Result` for user-friendly error messages.

## Tracing and Observability (Phase 4)

### Structured Logging

Replaced `env_logger`/`log` with `tracing`/`tracing-subscriber` for structured, hierarchical logging:

```rust
// od-sync/src/orchestrator.rs
use tracing::{info, warn, error, instrument};

#[instrument(skip(self, repos))]
pub async fn sync_all(&self, repos: Vec<Repository>) -> Result<SyncResult> {
    info!(count = repos.len(), "Starting sync");
    
    let mut join_set = JoinSet::new();
    for repo in repos {
        let span = tracing::info_span!("sync_repo", repo_id = %repo.id);
        join_set.spawn(async move {
            let _enter = span.enter();
            self.sync_repository(&repo).await
        });
    }
    
    info!("Sync complete");
    Ok(result)
}
```

### Span Hierarchy

```
sync_all
├── sync_repo (repo_id=owner/name)
│   ├── fetch_from_github
│   ├── merge_sync_data
│   └── update_cache
├── sync_repo (repo_id=owner/name2)
│   ├── fetch_from_github
│   ├── merge_sync_data
│   └── update_cache
└── generate_quality_report
```

### Logging Configuration

```bash
# Standard logging
RUST_LOG=info cargo run -- sync

# Detailed logging
RUST_LOG=debug cargo run -- sync

# JSON output for machine parsing
RUST_LOG=info cargo run -- sync | jq .

# Per-module filtering
RUST_LOG=od_sync=debug,od_core=info cargo run -- sync
```

### Dependencies

- `tracing` (0.1) — Instrumentation framework
- `tracing-subscriber` (0.3) — Logging backend with JSON support

## Design Patterns

### 1. Builder Pattern
```rust
let mut validator = Validator::new();
validator.add_rule(NoDuplicateReposRule);
validator.add_rule(ExternalDataConsistencyRule);
let report = validator.validate(&data);
```

### 2. Strategy Pattern
```rust
pub enum MergeStrategy {
    PreferManual,    // Manual additions override starred data
    PreferStarred,   // Starred data takes precedence
    Strict,          // Reject conflicts
}

let merger = DataMerger::new(strategy);
```

### 3. Trait-Based Polymorphism
```rust
impl ValidationRule for CustomRule {
    fn check(&self, data: &CanonicalData) -> ValidationResult {
        // Custom validation logic
    }
}
```

### 4. Adapter Pattern
```rust
impl DataSourceAdapter for GitHubAdapter {
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository> {
        // GitHub-specific implementation
    }
}
```

### 5. Error Context Pattern
```rust
file.read()
    .context("Failed to read repositories.yml")?;
```

## Performance Characteristics

### Benchmarks (845 repositories)

| Operation | Time | Details |
|-----------|------|---------|
| GitHub API Sync (per repo) | ~2-3s | With rate limiting |
| Cache Hit | ~1ms | Cached repository lookup |
| Parse LIST.md | ~50ms | Regex-based extraction |
| Merge data sources | ~30ms | In-memory operations |
| Validation (9 rules) | ~80ms | Includes E007, E008 |
| Generate 4 files | ~120ms | Template rendering |
| **Total Pipeline** | **~280ms** | End-to-end (cached) |

**Memory Usage**: ~25MB peak during full pipeline

### Scalability

**Current**: 845 repos, 57 web refs, 7 books  
**Tested**: Up to 1000 repos without degradation  
**Bottlenecks**: GitHub API rate limits (5000/hour), template rendering (linear)

**Optimization Strategies**:
- Intelligent caching with ETag support
- Rate limit buffering (500 request buffer)
- Parallel processing ready (tokio async)
- Incremental updates (planned)

## Security Considerations

### Credential Management
- Multi-source support (env, file, keychain)
- Secure file storage (0600 permissions on Unix)
- Token redaction in logs (show first 4 chars + "***REDACTED***")
- Legacy token detection and migration
- No tokens in code or version control

### Input Validation
- All URLs validated with regex
- YAML/JSON deserialization uses safe serde
- No shell command execution from user data
- File paths validated before I/O

### Data Integrity
- Validation prevents malformed data
- Atomic writes for canonical data (planned)
- Version control recommended for data files

## Testing Strategy

### Test Coverage

```
Module                Unit Tests    Integration Tests    Total
─────────────────────────────────────────────────────────────
config/*                  15              3              18
sync/*                     9              -               9
models/*                  15              2              17
parsers/*                  5              1               6
validators/*              15              2              17
generators/*               1              1               2
cross_refs/*               2              1               3
merge/*                    2              1               3
collections/*              8              2              10  (Phase 3)
relations/*                6              2               8  (Phase 3)
concurrent_sync/*          7              3              10  (Phase 3)
Total                     85              18            103
```

**Current Status**: 110 tests passing (up from 103)

### Test Categories
1. **Unit Tests** - Individual function/method testing
2. **Integration Tests** - End-to-end pipeline testing
3. **Mock Tests** - GitHub API simulation

## Extension Points

### Adding New Data Source Adapters

```rust
pub struct CustomAdapter {
    config: AdapterConfig,
}

#[async_trait]
impl DataSourceAdapter for CustomAdapter {
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository> {
        // Custom API integration
    }
    
    async fn check_connection(&self) -> Result<()> {
        // Connection verification
    }
    
    fn source_name(&self) -> &str {
        "custom_source"
    }
}
```

See integration guides:
- [Google Sheets Integration](./INTEGRATION_GUIDE_GOOGLE_SHEETS.md)
- [Jira Integration](./INTEGRATION_GUIDE_JIRA.md)
- [Git Repository Scanner](./INTEGRATION_GUIDE_GIT_REPOSITORY.md)
- [Integration Index](./INTEGRATION_GUIDE_INDEX.md)

### Adding New Validation Rules

```rust
pub struct CustomRule;

impl ValidationRule for CustomRule {
    fn name(&self) -> &str { "custom_rule" }
    fn default_severity(&self) -> Severity { Severity::Warning }
    
    fn check(&self, data: &CanonicalData) -> ValidationResult {
        // Custom validation logic
        ValidationResult::ok()
    }
}

validator.add_rule(CustomRule);
```

### Adding New CLI Commands

```rust
enum Commands {
    // ... existing commands
    NewCommand {
        #[arg(short, long)]
        option: String,
    },
}

match cli.command {
    Commands::NewCommand { option } => {
        // Implementation
    }
}
```

## Dependencies

### Core Dependencies
- **serde** (1.0) - Serialization framework
- **serde_yaml** (0.9) - YAML support
- **serde_json** (1.0) - JSON support
- **clap** (4.5) - CLI argument parsing
- **tera** (1.20) - Template engine
- **petgraph** (0.6) - Graph data structure
- **regex** (1.10) - Pattern matching
- **anyhow** (1.0) - Error handling
- **thiserror** (1.0) - Custom error types
- **chrono** (0.4) - Date/time handling

### Sync Dependencies
- **octocrab** (0.40) - GitHub API client
- **tokio** (1.40) - Async runtime
- **async-trait** (0.1) - Async trait support
- **indicatif** (0.17) - Progress bars
- **toml** (0.8) - Configuration files
- **dirs** (5.0) - Platform directories

## Future Architecture

### Planned Enhancements

1. **Parallel Sync Workers** (Phase 11, Task 60)
   - tokio::task::spawn for concurrent processing
   - Semaphore-based worker limits
   - Thread-safe progress tracking

2. **ETag Conditional Requests** (Phase 11, Task 61)
   - Store ETags in cache
   - If-None-Match headers
   - 304 Not Modified handling

3. **Quality Monitoring** (Phase 9, Tasks 52-54)
   - Data completeness metrics
   - Anomaly detection (star drops, language changes)
   - Structured logging with JSON format

4. **Error Recovery** (Phase 10, Tasks 55-59)
   - Comprehensive sync error types
   - 404 handling (mark as deprecated)
   - Exponential backoff for network errors
   - Repository transfer detection

5. **Plugin System**
   - Custom validation rules as plugins
   - Custom output formats
   - External data sources

6. **Web Interface**
   - REST API with axum
   - Real-time validation
   - Search and query endpoints

## Monitoring and Observability

### Logging Framework
```bash
RUST_LOG=debug cargo run -- sync     # Detailed logging
RUST_LOG=info cargo run -- generate  # Standard logging
```

### Metrics
- Sync progress with real-time progress bars
- Cache hit/miss statistics
- API response times
- Validation report with comprehensive metrics
- Performance timing for all operations

## Backward Compatibility

### Maintained Compatibility
✅ LIST.md format unchanged  
✅ TABLE.md format unchanged  
✅ Section anchors preserved  
✅ URL structures maintained  
✅ Star count representation identical  

### New Features
✨ External data sync with GitHub API  
✨ Secure credential management  
✨ Enhanced validation (E007, E008)  
✨ Unidirectional architecture  
✨ Mermaid diagrams as code  

### Migration Path
- Parse command still available for legacy workflows
- Sync completely replaces manual LIST.md editing
- No breaking changes to output formats

## References

- [Tera Template Documentation](https://keats.github.io/tera/)
- [petgraph Documentation](https://docs.rs/petgraph/)
- [octocrab Documentation](https://docs.rs/octocrab/)
- [tokio Documentation](https://tokio.rs/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Original Stargazer](https://github.com/encodedfox/stargazer)

---

**Last Updated**: 2025-12-11  
**Version**: 0.1.0  
**Rust Edition**: 2021  
**Implementation Status**: Phases 1-5, 7-8 complete (58%)