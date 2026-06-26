# RepoQuery

[![License: CC0](https://img.shields.io/badge/License-CC0%201.0-lightgrey.svg)](https://creativecommons.org/publicdomain/zero/1.0/)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

High-performance Rust CLI for managing, synchronizing, and generating documentation from GitHub starred repositories. Processes 845+ repositories with concurrent sync, multi-format generation, and extensible validation.

## Install

```bash
# Clone and build
git clone <repository-url>
cd repoquery
cargo build --release

# Binary is at target/release/repoquery
```

## Quick Start

```bash
# 1. Configure GitHub credentials
cargo run -p repoquery -- configure

# 2. Sync starred repos from GitHub
cargo run -p repoquery -- sync

# 3. Import into SQLite for fast queries
cargo run -p repoquery -- import --from data/canonical/repositories.yml --to data/repoquery.db

# 4. Generate documentation
cargo run -p repoquery -- generate

# 5. Launch interactive TUI
cargo run -p repoquery -- tui
```

## Features

### Sync
- Concurrent sync with bounded worker pool (default: 3)
- GraphQL bulk fetch for efficient API usage
- Multi-relation support: starred, owned, forked, watching, org member, contributed
- Fork sync status tracking (commits ahead/behind upstream)
- ETag-based caching with configurable TTL
- Dry-run mode and selective repository sync

### Storage
- SQLite + YAML backends via trait-based persistence layer
- Import/export between formats
- Canonical YAML as single source of truth

### Collections
- User-defined repository groupings with CRUD operations
- Auto-generate collections from GitHub topics
- Per-collection markdown generation

### Custom Tags & Notes
- Per-repository tagging system
- Custom notes and metadata
- Tag-based filtering in TUI

### Validation
- 9 extensible rules (E001–E008)
- External data consistency checks
- Validation reports in JSON

### Generation
- Tera template engine for customizable output
- Per-collection markdown generation
- LIST.md, TABLE.md, ARCHIVE.md, ARCHIVE_TABLE.md

### Cross-Reference Graph
- Bidirectional relationship tracking (petgraph)
- Repository dependency navigation

### Quality Scoring
- Automated assessment based on stars, activity, metadata completeness

### Interactive TUI
- Browse repositories and collections with keyboard navigation
- Real-time filtering and search
- Tag filter cycling and fork status display
- Built with ratatui and crossterm

### Query & Search
- Multi-filter repository queries (language, stars, owner, license, topic)
- Full-text search across names, descriptions, and topics
- Multiple output formats: table, JSON, Markdown, CSV
- Configurable sorting and limits

### Activity Monitoring
- Classify repos as Active, Maintained, Stale, or Abandoned
- Configurable time thresholds
- Trending detection by star growth rate
- ASCII histogram charts

### Security Hardening
- Token redaction in all log output
- SQLite integrity checks on startup
- Path traversal protection
- Cargo audit in CI pipeline
- Clippy deny-level lints
- FGAT token hashing (SHA-256, raw tokens never persisted to SQLite)
- Credential file symlink rejection + TOCTOU protection
- SQL LIKE wildcard escape in tag filters
- HTTP client timeout enforcement (sync: 30s, configure: 10s)
- Concurrent sync file lock protection (fs2 exclusive lock)
- Required GitHub token scope validation (repo, public_repo)
- GraphQL error message redaction via `redact_sensitive()`

### Shell Completions & Man Page
- Generate completions for bash, zsh, fish, and powershell
- Man page generation via `repoquery man`

### Structured Logging
- Tracing-based logging with configurable levels
- Debug output for troubleshooting

## CLI Commands

| Command | Description |
|---------|-------------|
| `parse` | Parse existing LIST.md and TABLE.md into canonical format |
| `validate` | Validate canonical data (9 rules) |
| `generate` | Generate markdown documents from canonical data |
| `merge` | Merge manual additions into canonical data |
| `stats` | Show statistics about repository data |
| `configure` | Configure RepoQuery settings and credentials |
| `migrate-credentials` | Migrate credentials from legacy location |
| `sync` | Sync repository metadata from external sources |
| `status` | Show sync status and system health |
| `import` | Import data between store formats (YAML ↔ SQLite) |
| `export` | Export data between store formats (SQLite ↔ YAML) |
| `collections` | Manage repository collections (list, create, show, add, remove, delete, auto-generate) |
| `tui` | Interactive terminal UI for browsing and managing repositories |
| `config` | Manage configuration (init, show, set) |
| `activity` | Analyze repository activity (active, stale, abandoned, trending) |
| `query` | Query repositories with filters (list, search, show, topics, languages) |
| `repo` | Manage individual repository metadata (tag, untag, note, show) |
| `completions` | Generate shell completions (bash, zsh, fish, powershell) |
| `man` | Generate man page |

## Architecture

7-crate workspace with strict one-way dependency flow:

```
od-core       → Pure types, config, validators, parsers, merge
od-store      → Trait-based persistence (YAML, SQLite)
od-sync       → Sync orchestrator, adapters, cache, progress
od-validate   → Validation framework and rules
od-generate   → Tera-based markdown generation
od-graph      → petgraph cross-reference graph + navigator
repoquery     → Binary with 14 command handlers
```

**Dependency flow**: od-core ← od-store, od-sync, od-validate, od-generate, od-graph ← repoquery

## Configuration

### GitHub Credentials

Set up credentials via one of three methods:

1. **Environment variable** (highest priority):
   ```bash
   export GITHUB_TOKEN=ghp_xxxxxxxxxxxx
   ```

2. **Config file** (`~/.config/repoquery/repoquery.toml`):
   ```toml
   [github]
   token = "ghp_xxxxxxxxxxxx"
   ```

3. **OS keychain** (macOS Keychain, Windows Credential Manager, Linux Secret Service):
   ```bash
   cargo run -p repoquery -- configure
   ```

Run `cargo run -p repoquery -- configure --show` to view current configuration.

## Documentation

- **[Architecture](./docs/ARCHITECTURE.md)** — System design and module overview
- **[CLI Command Reference](./docs/CLI_COMMAND_REFERENCE.md)** — Full command tree and options
- **[Configuration](./docs/CONFIGURATION.md)** — Config file, env vars, credential setup
- **[Storage Modes](./docs/STORAGE_MODES.md)** — YAML vs SQLite vs Dual mode
- **[Security Practices](./docs/SECURITY_PRACTICES.md)** — Token protection, audit, linting
- **[Data Sync Guide](./docs/DATA_SYNC.md)** — Complete sync setup and usage
- **[API Reference](./docs/API_REFERENCE.md)** — CLI commands and library API
- **[Development](./docs/DEVELOPMENT.md)** — Contributing and development setup
- **[Troubleshooting](./docs/TROUBLESHOOTING.md)** — Common issues and solutions

## Project Status

- **Repositories**: 845 tracked (797 active, 48 archived)
- **Tests**: 191 passing
- **Performance**: Sub-second processing for full dataset
- **Phases**: 0–8 + security hardening, config overhaul, query commands, activity monitoring, testing infrastructure, documentation (6 phases ported from RepoSQL specs)

## License

This project is released under the **CC0 1.0 Universal** license, placing it in the public domain.

[![CC0](https://licensebuttons.net/p/zero/1.0/88x31.png)](https://creativecommons.org/publicdomain/zero/1.0/)

You can copy, modify, distribute and perform the work, even for commercial purposes, all without asking permission. See [LICENSE](LICENSE) for the full legal text.