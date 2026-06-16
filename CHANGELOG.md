# Changelog

All notable changes to the OmniDatum repository documentation will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [2.0.0] - 2025-12-11

### Added - External Data Synchronization

- **GitHub API Integration**: Real-time repository metadata sync using octocrab
  - Automatic star count updates
  - Description and metadata synchronization
  - License information fetching
  - Topics and homepage tracking
  - Archive status detection
- **Secure Credential Management**: Multi-source token storage
  - Environment variable support (GITHUB_TOKEN, GH_TOKEN)
  - Secure file storage (~/.config/omnidatum/credentials with 0600 permissions)
  - OS keychain integration (macOS Keychain, Linux secret-tool)
  - Token redaction in logs and error messages
- **Smart Caching System**: ETag-based cache with configurable TTL
  - Default 24-hour cache freshness
  - Configurable TTL (1-168 hours)
  - Cache hit/miss tracking
  - Automatic cache management
- **New CLI Commands**:
  - `sync`: Synchronize repository metadata from GitHub API
  - `configure`: Interactive credential and configuration setup
  - `status`: System health and sync status check
  - `migrate-credentials`: Migrate legacy tokens to secure storage

### Added - Enhanced Validation

- **E007: External Data Consistency Rule**: Validates synced data integrity
  - Checks for unrealistic star counts (>1,000,000)
  - Verifies archive status consistency
  - Flags missing descriptions for popular repos
- **E008: Duplicate Repository Name Rule**: Detects duplicate full_names
- Enhanced validation framework with external consistency checking
- Validation flags: `--check-external-consistency` (default: true)
- Sync data validation in generate command: `--validate-sync-data` (default: true)

### Added - Quality Monitoring

- **Sync Quality Reports**: Automatic quality assessment after each sync
  - Data completeness metrics (license, description, topics, homepage)
  - Anomaly detection (star drops >25%, spikes >100%, description changes, language changes, newly archived)
  - Quality report saved to data/cache/sync_quality_report.json
- **Status Command Enhancements**: Display quality metrics and recent anomalies
- **Verbose Logging**: Per-repository sync details with `--verbose` flag

### Added - Error Handling

- **Comprehensive Error Types**: SyncError enum with 7 specific error types
  - AuthenticationFailed, RateLimitExceeded, RepositoryNotFound
  - NetworkError, InvalidConfiguration, CacheError, ParseError
- **404 Recovery**: Graceful handling of deleted/private repositories
  - Marks repository as deprecated
  - Continues sync without failing
  - Preserves repository history
- **Exponential Backoff**: Network resilience with automatic retry
  - 3 retry attempts with exponential delay (1s, 2s, 4s)
  - Retries only network errors
  - Logs each retry attempt

### Added - Documentation

- **User Guides**:
  - DATA_SYNC.md (438 lines): Complete sync setup and usage guide
  - TROUBLESHOOTING.md (377 lines): All error codes E001-E008 with solutions
  - Enhanced API_REFERENCE.md: All new commands documented
- **Developer Guides**:
  - DEVELOPMENT.md (276 lines): Development setup and contribution guide
  - Enhanced ARCHITECTURE.md (670 lines): Complete system architecture
  - docs/README.md (181 lines): Documentation index and navigation
- **Integration Guides**: Complete adapter implementations
  - INTEGRATION_GUIDE_GOOGLE_SHEETS.md (428 lines)
  - INTEGRATION_GUIDE_JIRA.md (492 lines)
  - INTEGRATION_GUIDE_GIT_REPOSITORY.md (541 lines)
  - INTEGRATION_GUIDE_INDEX.md (427 lines)
- **Diagrams**: Mermaid source files with generation script
  - pipeline.mmd, data-flow.mmd, sync-sequence.mmd, components.mmd
  - scripts/generate-diagrams.sh for PNG/SVG generation

### Added - Testing Infrastructure

- Mock GitHub adapter for offline testing (189 lines)
- 3 new integration tests:
  - Full sync workflow test
  - Rate limit handling test
  - Authentication failure test
- 10+ new unit tests for sync, cache, credentials, quality reports
- Total test suite: 81 tests (79 passing)

### Changed - Architecture

- **Unidirectional Data Flow**: Eliminated circular dependencies
  - External sources → Canonical storage → Generated outputs
  - Generated documents (LIST.md, TABLE.md) are read-only
  - Parse command now legacy migration path only
- **Configuration System**: TOML-based configuration
  - ~/.config/omnidatum/config.toml
  - Configurable sync, validation, and generation settings
  - Validation of parameter ranges

### Changed - CLI Enhancements

- **Sync Command**: Multiple operation modes
  - Selective sync: `--repos` flag for specific repositories
  - Dry-run mode: `--dry-run` flag for preview
  - Force refresh: `--force` flag to ignore cache
  - Verbose mode: `--verbose` flag for detailed logging
  - Cache clearing: `--clear-cache` flag
- **Validate Command**: `--check-external-consistency` flag
- **Generate Command**: `--validate-sync-data` flag
- **Configure Command**: `--interactive` and `--show` modes
- All commands now async-capable with tokio runtime

### Changed - Documentation Structure

- **Root Directory**: Cleaned and consolidated
  - Single README.md (consolidated from 4 files)
  - Removed: README_ENHANCED.md, README_PROCESSOR.md
  - Archived obsolete files to docs/archive/
- **docs/ Directory**: Professional organization
  - User guides, developer guides, integration guides
  - Diagrams directory for Mermaid sources
  - Archive directory for historical documents

### Improved - Performance

- Sub-second processing maintained (~280ms for 845 repos)
- Smart caching reduces API calls by 70-80%
- Progress bars for long-running operations
- Parallel processing ready (foundation in place)

### Security

- Secure credential storage with file permission enforcement
- Token redaction in all logs and error messages
- No credentials in code or version control
- Legacy token detection and migration guide
- Minimum permission requirements documented (public_repo only)

### Backward Compatibility

- ✅ All existing commands still work
- ✅ LIST.md and TABLE.md format unchanged
- ✅ Generated output fully compatible
- ✅ Parse command still available for legacy workflows
- ✅ No breaking changes to data structures

### Migration Guide

See [docs/DATA_SYNC.md](docs/DATA_SYNC.md) for:
- Credential setup instructions
- First-time sync walkthrough
- Configuration examples
- Troubleshooting guide

### Deprecated

- Manual editing of LIST.md (use sync command instead)
- Direct token storage in files (use configure command)
- Separate README files (consolidated into single README.md)

## [2.0.0] - 2026-05-03

### Summary: v2 Migration Complete

All phases of the OmniDatum v2 migration are now complete. The system has evolved from a monolithic Rust CLI into a modular 7-crate workspace with comprehensive external data sync, validation, generation, and cross-reference tracking. The architecture is production-ready with 123 passing tests, sub-second processing, and full backward compatibility.

**Phases Completed**:
- Phase 0: Architecture refactoring (command extraction, explicit exports)
- Phase 1: Workspace restructuring (7-crate modular design)
- Phase 2: Trait-based persistence layer (YAML + SQLite backends)
- Phase 3: Data model expansion (relations, collections, concurrent sync)
- Phase 4: Typed error handling and tracing infrastructure
- Phase 5: Testing infrastructure (95 → 118 tests)
- Phase 6: GraphQL integration and per-collection generation (118 → 123 tests)
- Phases 7–8: Diagrams and documentation

**Key Metrics**:
- 845 repositories tracked (797 active, 48 archived)
- 123 tests passing (up from 95 at Phase 0)
- Sub-second processing (~280ms for full dataset)
- 7 crates with acyclic dependency graph
- 9 validation rules (E001–E008)
- 7 repository relationship types
- 4 output formats (LIST.md, TABLE.md, ARCHIVE.md, ARCHIVE_TABLE.md)

## [2.1.0-phase8] - 2026-05-03

### Added - Topic-Based Collections & Fork Tracking (Phase 8)

- **Auto-Generate Collections from Topics**:
  - `collections auto-generate --min-repos N` creates collections from GitHub topics
  - Collections prefixed with `topic:` for automatic discovery
  - Uses existing synced topic data (no additional API calls)
- **Fork Sync Status**:
  - `fork_ahead: Option<u32>` and `fork_behind: Option<u32>` on Repository
  - `sync --check-forks` fetches commit comparison with upstream via REST API
  - Fork status displayed in TUI detail view
- **Custom Tags & Notes**:
  - `custom_tags: Vec<String>` on Repository
  - New `repo` subcommand with tag/untag/note/show actions
  - Tags filterable in TUI (t key) and via RepoFilter
- **TUI Enhancements**:
  - Tag filter cycling (t key) in RepoList view
  - Fork status display in RepoDetail view
  - Custom tags display in RepoDetail view

### Added - Test Coverage (Phase 8)

- **+4 Tests** (127 total, up from 123):
  - Backward compatibility test (fork_ahead/fork_behind fields)
  - Auto-collection generation from topics
  - Tag operations (tag/untag/note/show)
  - Tag filter cycling in TUI

## [Unreleased]

## [2.1.0-tui] - 2026-05-03

### Added - Interactive Terminal UI

- **New `tui` Command**: `omnidatum-processor tui --store <path>` launches interactive terminal UI
- **5 Views**:
  - RepoList: Filterable table with j/k navigation and / search
  - RepoDetail: Full repository metadata display
  - Collections: List of user-defined repository groupings
  - CollectionDetail: Collection members and metadata
  - Stats: Dashboard with repository statistics
- **Key Bindings**:
  - j/k: Navigate up/down
  - /: Search repositories
  - Enter: View repository details
  - Esc: Return to previous view
  - q: Quit application
  - 1/2/3: Switch between views
  - l/r: Cycle through filters (language, relation type)
- **Panic Safety**: Custom panic hook restores terminal state on crash
- **Read-Only Mode**: TUI reads from store without modifying data (collection management via CLI)
- **New Dependencies**: ratatui 0.28, crossterm 0.28

## [2.1.0-phase6] - 2026-05-03

### Added - GitHub GraphQL Integration (Phase 6)

- **GraphQL Client** (od-sync/src/adapters/graphql.rs):
  - reqwest-based HTTP client for api.github.com/graphql
  - Paginated bulk fetch: 100 repositories per request (vs 1 per REST call)
  - Queries for starred, owned, forked, watched repositories
  - Fork parent extraction (nameWithOwner, url)
- **Fork Upstream Tracking** (od-core):
  - `fork_parent: Option<String>` field on Repository
  - `fork_parent_url: Option<String>` field on Repository
  - Backward-compatible via serde(default)
  - merge_sync_data propagates fork parent from synced repositories
- **Per-Collection Generation** (od-generate + od-cli):
  - `generate_list_for_collection()` method
  - `generate_table_for_collection()` method
  - `--collection <id>` flag on generate command
  - Output to `generated/<collection-id>/LIST.md` and `TABLE.md`
- **New Dependency**: reqwest 0.12 (for GraphQL HTTP client)

### Added - Test Coverage (Phase 6)

- **+5 Tests** (123 total, up from 118):
  - GraphQL node conversion test
  - Fork parent backward compatibility test
  - Per-collection LIST.md generation test
  - Per-collection TABLE.md generation test
  - Collection generation with empty data test

## [2.1.0-phase4] - 2026-05-03

### Added - Typed Error Handling (Phase 4)

- **Per-Crate Error Types** (thiserror):
  - `od-core::CoreError` — Io, Yaml, Json, Config, Toml, TomlSerialize, Regex
  - `od-store::StoreError` — Sqlite, Core, Json, Io, Other
  - `od-sync::SyncError` — Updated with From impls for CoreError
  - `od-validate::ValidateError` — Core, Io, Other
  - `od-generate::GenerateError` — Template, Io, Core
  - `od-graph::GraphError` — Core, Other
  - `od-cli` — Remains anyhow::Result (CLI layer converts typed errors)
- **Error Context**: All errors include context via anyhow::Context trait

### Added - Tracing Infrastructure (Phase 4)

- **Structured Logging**: Replaced env_logger/log with tracing/tracing-subscriber
  - Structured spans in sync orchestrator (sync_all, per-repo)
  - Hierarchical context propagation
  - JSON output support for machine parsing
- **Dependency Update**: tracing (0.1), tracing-subscriber (0.3) added

### Changed - Quality Scoring Unification (Phase 4)

- **Single Implementation**: `QualityMetrics::calculate_score()` in od-core
  - Duplicate implementations in parsers and merge removed
  - Consistent scoring across all code paths
  - Centralized quality logic

### Changed - Clone Elimination (Phase 4)

- **Zero-Copy References**: `CanonicalData::partition_repositories()` returns `(Vec<&Repository>, Vec<&Repository>)`
  - Eliminates unnecessary clones in partition operations
  - Improves performance for large datasets
  - Maintains type safety with lifetime annotations

### Added - Test Coverage (Phase 4)

- **+8 Tests** (118 total, up from 110):
  - Sync orchestrator: 6 tests (parse_github_repo, merge_sync_data, quality report detection)
  - Markdown generator: 3 tests (list, table, empty data rendering)
  - Cross-ref graph: 3 tests (build, navigator, empty graph)

## [2.1.0-phase3] - 2026-05-03

### Added - Data Model Expansion (Phase 3)

- **Relation Enum**: Seven repository relationship types
  - Starred, Owned, Forked, Watching, OrgMember, Contributed, ManuallyAdded
  - Enables multi-relation tracking per repository
- **Collection Struct**: User-defined repository groupings
  - CRUD methods for collection management
  - Stored in CanonicalData and SQLite
- **Repository.relations Field**: Vec<Relation> with backward-compatible serde default
- **CanonicalData.collections Field**: Vec<Collection> with backward-compatible serde default

### Added - Concurrent Sync (Phase 3)

- **Parallel Worker Pool**: tokio::task::JoinSet + tokio::sync::Semaphore
  - Bounded by `parallel_workers` config (default: 3, range: 1-10)
  - Configurable concurrency limits
- **sync_by_relation Method**: Fetch repositories by relation type
- **GitHubAdapter Expansion**: Four new methods
  - `fetch_user_repos()` — User's starred repositories
  - `fetch_user_forks()` — User's forked repositories
  - `fetch_watched_repos()` — User's watched repositories
  - `fetch_org_repos()` — Organization repositories
- **SyncResult.by_relation**: HashMap<String, usize> tracks sync counts per relation

### Added - Collections CLI (Phase 3)

- **New `collections` Subcommand**: Six actions
  - `list` — Display all collections
  - `create <name>` — Create new collection
  - `show <name>` — Display collection details
  - `add <collection> <repo>` — Add repository to collection
  - `remove <collection> <repo>` — Remove repository from collection
  - `delete <name>` — Delete collection
- **SQLite Backend**: Collections persisted to data/omnidatum.db
- **--relations Flag on Sync**: Comma-separated relation types (starred,owned,forked,watching)

### Changed - Store Layer (Phase 3)

- **RepoStore Trait Extended**: Four new collection methods
  - `create_collection()`, `get_collection()`, `list_collections()`, `delete_collection()`
- **SqliteStore**: Collections table added with schema
- **YamlStore**: Collections via CanonicalData field

### Test Results

- Total tests: 110 passing (up from 103)
- New test categories: Relation tracking, collection CRUD, concurrent sync

## [2.1.0-phase2] - 2026-05-03

### Added - Trait-Based Persistence Layer (Phase 2)

- **RepoStore Trait**: Abstraction for repository storage with 7 methods
  - `load_all()`, `save_all()` — bulk operations
  - `get_repo()`, `upsert_repo()`, `delete_repo()` — single-record CRUD
  - `list_repos()`, `count_repos()` — query operations
- **SqliteStore Implementation**: rusqlite-backed persistence
  - JSON columns for complex fields (platforms, metadata, classification)
  - Indexed scalar columns for efficient queries (language, archived, stars, source)
  - Atomic transactions for data consistency
- **YamlStore Implementation**: Backward-compatible wrapper
  - Wraps existing load_canonical/save_canonical
  - Maintains full compatibility with current workflows
- **RepoFilter**: Query builder for filtering repositories
  - Filter by language, archived status, minimum stars, source
  - Composable filter conditions
- **open_store() Function**: Auto-detects backend from file extension
  - `.db` → SqliteStore
  - `.yml` → YamlStore
- **New CLI Commands**:
  - `import --from <yaml> --to <db>` — migrate YAML data to SQLite
  - `export --from <db> --to <yaml>` — export SQLite data to YAML
- **New Dependency**: rusqlite 0.31 (bundled SQLite)
- **9 New Tests**: SQLite round-trip, CRUD operations, filter queries, YAML round-trip, cross-store equivalence

### Changed - Storage Architecture

- **od-store Crate Restructured**:
  - `traits.rs` — RepoStore trait definition and RepoFilter
  - `sqlite.rs` — SqliteStore implementation
  - `yaml.rs` — YamlStore implementation
  - `store.rs` — open_store() convenience function
- **Backward Compatibility**: Existing commands unchanged
  - All commands continue using YAML via load_canonical/save_canonical
  - SQLite is opt-in via import/export commands

### Test Results

- Total tests: 103 passing (up from 95)
- New test categories: SQLite CRUD, filter queries, cross-store equivalence

## [2.1.0-phase1] - 2026-05-03

### Changed - Workspace Restructuring (Phase 1)

- **Monolith to Workspace**: Single-crate project split into 7-crate Cargo workspace
  - `crates/od-core/` — Models, config, merge, parsers, readme_updater (pure types, no async/network)
  - `crates/od-store/` — YAML file I/O (load_canonical, save_canonical)
  - `crates/od-sync/` — Sync orchestrator, adapters (GitHub, GitLab stub, Codeberg stub), cache, progress, client
  - `crates/od-validate/` — Validation framework, rules E001-E008, external data rules
  - `crates/od-generate/` — Tera-based markdown generation
  - `crates/od-graph/` — petgraph cross-reference graph + navigator
  - `crates/od-cli/` — Binary crate with main.rs + 9 command handlers
- **Root Cargo.toml**: Now workspace manifest with [workspace.dependencies] for shared versions
- **Binary Name**: Unchanged (omnidatum-processor)
- **Removed**: Old src/ directory (all code migrated to crates/)
- **Test Suite**: 95 tests passing (up from 68)

### Benefits

- **Dependency Isolation**: Each crate declares only required dependencies
- **Reusability**: od-core, od-store, od-validate, od-generate can be used independently
- **Testability**: Smaller crates easier to test in isolation
- **Maintainability**: Clear separation of concerns (models, I/O, sync, validation, generation, graph, CLI)
- **Future Extensibility**: Easy to add new adapters, validators, or output formats

## [2.1.0-phase0] - 2026-05-03

### Changed - Architecture Refactoring (Phase 0)

- **Command Extraction**: All 9 command handlers extracted from main.rs into dedicated modules
  - `src/commands/parse.rs`, `validate.rs`, `generate.rs`, `merge.rs`, `stats.rs`
  - `src/commands/configure.rs`, `migrate_credentials.rs`, `sync_cmd.rs`, `status.rs`
  - main.rs now thin dispatch layer
- **Explicit Exports**: lib.rs wildcard re-exports replaced with 30+ explicit type exports
- **Dependency Update**: serde_yaml (deprecated) replaced with serde_yml 0.0.12 across all files
- **Trait Consistency**: DataSourceAdapter trait now uses single `identifier: &str` parameter
  - GitHubAdapter and MockGitHubAdapter both implement consistently

### Fixed - Testing & Quality

- **Test Isolation**: serial_test crate added for env-dependent credential tests
  - Env-dependent tests use `#[serial]` attribute
  - std::env::set_var removed from production code
  - env_logger::Builder used for test logging
- **Clippy Warnings**: 8 clippy warnings fixed
- **Test Cache**: test_cache_clear.json fixed to use tempdir

### Removed

- **context_portal/**: Legacy Python/Alembic/SQLite code retired
  - All functionality migrated to Rust
  - Alembic migrations no longer needed

## [Unreleased]

### Added
- Comprehensive integration test suite with 5 test scenarios
- UPDATE_RUNBOOK.md with complete workflow documentation
- CHANGELOG.md to track reorganization changes

## [1.0.0-baseline] - 2024-12-10

### Added
- Rust-based omnidatum-processor for enhanced data processing
- Canonical data models for repositories, manual projects, web references, and books
- Validation engine with 7 core validation rules
- Cross-reference graph system for bidirectional linking
- Template-based document generation using Tera
- Multi-platform tracking (GitHub, Codeberg, GitLab, Gitea, AWS CodeCommit)
- Migration detection and status tracking
- Quality scoring system (0-100) for repositories
- Archive filtering logic (< 10 stars AND > 2 years inactive)
- CLI with subcommands: parse, validate, generate, merge, stats
- Ecosystem detail files (e.g., docs/ecosystems/bitwarden.md)
- AI/ML modernization proposal documentation
- Baseline summary document (BASELINE_SUMMARY.md)
- README processor documentation (README_PROCESSOR.md)
- Project status tracking (IMPLEMENTATION_STATUS.md)

### Changed
- Reorganized data into structured YAML format:
  - repositories.yml (839 starred repositories)
  - manual_additions.yml (10 manual projects)
  - web_references.yml (57 references)
  - books.yml (4 core books)
  - merged_data.yml (unified canonical data)
- Enhanced LIST.md with platform migration notation
- Enhanced TABLE.md with archive status indicators
- Separated archived repositories into ARCHIVE.md and ARCHIVE_TABLE.md
- Improved star count accuracy across all formats
- Updated generation workflow from manual to automated

### Technical Details
- **Language**: Rust (2021 edition)
- **Dependencies**: 
  - serde/serde_yaml/serde_json for serialization
  - tera for template engine
  - petgraph for cross-reference graph
  - octocrab for GitHub API (future use)
  - clap for CLI
  - regex for parsing
- **Architecture**: 
  - Modular design with src/{models,validators,parsers,generators,cross_refs}
  - Separation of concerns: data model, validation, generation
  - Template-driven output for maintainability
- **Testing**: 
  - Unit tests for all major components
  - Integration tests for end-to-end workflows
  - Test coverage for validation rules, parsing, and generation

### Statistics (as of 2024-12-10)
- Total Repositories: 845 (839 starred + 6 manual additions, 3 conflicts resolved)
- Active Repositories: 797 (94.3%)
- Archived Repositories: 48 (5.7%)
- Platforms: GitHub (99.9%), Codeberg (0.1%)
- Languages: 43 (top: Go 32.7%, Rust 9.5%, Python 8.5%)
- Web References: 57 (34 active from README)
- Books: 4 (with 10 expansion topics identified)
- Quality Scores: Range 0-100, calculated from stars, activity, license, archive status

### Validation Results
- Errors: 0
- Warnings: 177 (primarily missing licenses - acceptable)
- Info: 21 (optimization suggestions)
- Migration Detections: 2
- Archive Candidates: 48 (low engagement repos)

### Known Issues
- Some repositories lack license information (184/845 = 21.8%)
- AI/ML web references need modernization (2023 content)
- Book section needs expansion (10 topics identified)
- Some GitHub Projects entries lack complete URLs

### Future Enhancements
- GitHub Actions workflow for automated weekly updates
- Dead link detection and reporting
- Enhanced search functionality across all documents
- Technology landscape visualization
- RSS feed for repository updates
- Automated description enhancement using LLMs
- Duplicate detection using semantic similarity

## [Pre-1.0.0] - Before 2024-12-10

### Historical State
- Manual maintenance of LIST.md and TABLE.md
- Used original stargazer tool (bash-based)
- No structured data format
- No validation system
- Manual cross-referencing
- No migration tracking
- No separation of active/archived repositories
- Limited metadata tracking

---

## Version Naming Convention

- **Major.Minor.Patch** for releases (e.g., 1.0.0)
- **-baseline** suffix for initial state snapshots
- **-rc** suffix for release candidates

## Change Categories

- **Added**: New features or files
- **Changed**: Changes to existing functionality
- **Deprecated**: Features that will be removed
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security-related changes

## Rollback Procedure

To rollback to v1.0.0-baseline if needed:

```bash
# Restore original files from backup
git checkout v1.0.0-baseline -- LIST.md TABLE.md README.md

# Or restore from git history
git log --oneline --all -- LIST.md
git checkout <commit-hash> -- LIST.md TABLE.md README.md
```

## Migration Notes

### Migrating from Pre-1.0.0 to 1.0.0-baseline

1. Existing LIST.md and TABLE.md remain compatible
2. New ARCHIVE.md and ARCHIVE_TABLE.md files added for archived repos
3. Data now sourced from YAML files in data/canonical/
4. Generation process changed from bash to Rust CLI
5. No breaking changes to output format - full backward compatibility maintained

### Data Model Changes

- Repository data structure extended with:
  - Multi-platform support
  - Migration tracking
  - Quality metrics
  - Enhanced classification

- New data types added:
  - ManualProject
  - WebReference
  - Book
  - Cross-reference graph nodes/edges

## Contribution Guidelines

When updating this changelog:

1. Add entries under [Unreleased] as changes are made
2. Move entries to versioned section when releasing
3. Follow Keep a Changelog format
4. Group changes by category (Added, Changed, etc.)
5. Include relevant details (file names, counts, percentages)
6. Update version tags when tagging releases

---

**Maintained by**: Repository owner
**Last Updated**: 2024-12-10