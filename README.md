# OmniDatum

[![License: CC0](https://img.shields.io/badge/License-CC0%201.0-lightgrey.svg)](https://creativecommons.org/publicdomain/zero/1.0/)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

High-performance Rust CLI for managing, synchronizing, and generating documentation from GitHub starred repositories. Processes 845+ repositories with concurrent sync, multi-format generation, and extensible validation.

## Install

```bash
# Clone and build
git clone <repository-url>
cd omnidatum
cargo build --release

# Binary is at target/release/omnidatum-processor
```

## Quick Start

```bash
# 1. Configure GitHub credentials
cargo run -p od-cli -- configure

# 2. Sync starred repos from GitHub
cargo run -p od-cli -- sync

# 3. Import into SQLite for fast queries
cargo run -p od-cli -- import --from data/canonical/repositories.yml --to data/omnidatum.db

# 4. Generate documentation
cargo run -p od-cli -- generate

# 5. Launch interactive TUI
cargo run -p od-cli -- tui
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
| `configure` | Configure OmniDatum settings and credentials |
| `migrate-credentials` | Migrate credentials from legacy location |
| `sync` | Sync repository metadata from external sources |
| `status` | Show sync status and system health |
| `import` | Import data from one store format to another |
| `export` | Export data from one store format to another |
| `collections` | Manage repository collections (list, create, show, add, remove, delete, auto-generate) |
| `tui` | Interactive terminal UI for browsing and managing repositories |
| `repo` | Manage individual repository metadata (tag, untag, note, show) |

## Architecture

7-crate workspace for modularity and reusability:

```
od-core       → Pure types, config, validators, parsers, merge
od-store      → Trait-based persistence (YAML, SQLite)
od-sync       → Sync orchestrator, adapters, cache, progress
od-validate   → Validation framework and rules
od-generate   → Tera-based markdown generation
od-graph      → petgraph cross-reference graph + navigator
od-cli        → Binary with 14 command handlers
```

**Dependency flow**: od-core ← od-store, od-sync, od-validate, od-generate, od-graph ← od-cli

## Configuration

### GitHub Credentials

Set up credentials via one of three methods:

1. **Environment variable** (highest priority):
   ```bash
   export GITHUB_TOKEN=ghp_xxxxxxxxxxxx
   ```

2. **Config file** (`~/.config/omnidatum/omnidatum.toml`):
   ```toml
   [github]
   token = "ghp_xxxxxxxxxxxx"
   ```

3. **OS keychain** (macOS Keychain, Windows Credential Manager, Linux Secret Service):
   ```bash
   cargo run -p od-cli -- configure
   ```

Run `cargo run -p od-cli -- configure --show` to view current configuration.

## Documentation

- **[Architecture](./docs/ARCHITECTURE.md)** — System design and module overview
- **[Data Sync Guide](./docs/DATA_SYNC.md)** — Complete sync setup and usage
- **[API Reference](./docs/API_REFERENCE.md)** — CLI commands and library API
- **[Development](./docs/DEVELOPMENT.md)** — Contributing and development setup
- **[Troubleshooting](./docs/TROUBLESHOOTING.md)** — Common issues and solutions

## Project Status

- **Repositories**: 845 tracked (797 active, 48 archived)
- **Tests**: 127 passing
- **Performance**: Sub-second processing for full dataset
- **Phases**: 0–8 complete (workspace, core, store, sync, validate, generate, graph, CLI, relations, collections, tags, TUI)

## License

This project is released under the **CC0 1.0 Universal** license, placing it in the public domain.

[![CC0](https://licensebuttons.net/p/zero/1.0/88x31.png)](https://creativecommons.org/publicdomain/zero/1.0/)

You can copy, modify, distribute and perform the work, even for commercial purposes, all without asking permission. See [LICENSE](LICENSE) for the full legal text.