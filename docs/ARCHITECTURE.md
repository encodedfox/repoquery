# Architecture

## Overview

RepoQuery (binary: `repoquery`) is a Rust CLI for managing GitHub starred repositories. It follows a layered architecture with strict dependency ordering.

```
┌──────────────────────────────────────────────────┐
│                   repoquery                       │
│         CLI binary (clap), commands, output       │
│             output formatters, TUI                │
├──────────────────────────────────────────────────┤
│  od-store  │  od-sync  │  od-validate  │ od-graph │
│  YAML/SQL  │  adapters │   9 rules     │ petgraph │
│  Dual mode │  cache    │  E001-E008    │ cross-ref│
├──────────────────────────────────────────────────┤
│               od-generate                         │
│         Tera templates, markdown output           │
├──────────────────────────────────────────────────┤
│                   od-core                         │
│  Types, config, models, parsers, merge, activity  │
└──────────────────────────────────────────────────┘
```

## Crate Layering

| Layer | Crate | Dependencies | Responsibility |
|-------|-------|--------------|----------------|
| 1 | `od-core` | serde, chrono, regex, toml | Pure types, config, models, parsers, merge logic, activity classification |
| 2 | `od-generate` | od-core, tera, anyhow | Tera-based markdown generation from canonical data |
| 3 | `od-store` | od-core, rusqlite, serde_yml | Trait-based persistence: YAML, SQLite, Dual-mode |
| 4 | `od-sync` | od-core, octocrab, reqwest, tokio | Sync orchestrator, GitHub adapters, GraphQL, caching |
| 5 | `od-validate` | od-core, anyhow | Validation framework with 9 pluggable rules |
| 6 | `od-graph` | od-core, petgraph | Cross-reference graph and navigation |
| 7 | `repoquery` | All above, clap, ratatui | CLI binary with command handlers, output formatters, TUI |

**Dependency flow** (strict one-way):
```
od-core ← od-store, od-sync, od-validate, od-generate, od-graph ← repoquery
```

## Persistence Layer

The `RepoStore` trait in `od-store` defines the storage interface:

```
                    ┌──────────────┐
                    │  RepoStore   │
                    │   (trait)    │
                    └──────┬───────┘
             ┌─────────────┼─────────────┐
             ▼             ▼             ▼
       ┌──────────┐ ┌──────────┐ ┌──────────┐
       │YamlStore │ │SqliteStore│ │DualStore │
       │  .yml    │ │  .db     │ │both      │
       └──────────┘ └──────────┘ └──────────┘
```

- **YamlStore**: Reads/writes canonical YAML files. Single source of truth.
- **SqliteStore**: SQLite-backed with PRAGMA integrity checks on startup.
- **DualStore**: Writes to both stores in sync. Seeds SQLite from YAML on first use.

## Key Design Decisions

### Storage Modes (configurable via `storage.mode`)

| Mode | Behavior |
|------|----------|
| `yaml` (default) | Canonical YAML as single source of truth; SQLite is for queries only |
| `sqlite` | SQLite is primary; YAML is export-only |
| `dual` | Both stores kept in sync; YAML remains canonical |

### Activity Classification

```
Now - last_push:
  < 3 months  → Active
  3-12 months → Maintained
  12-24 months → Stale
  > 24 months → Abandoned
```

Thresholds configurable at runtime via CLI flags.

### Config Priority Chain

1. Default values (lowest priority)
2. Config file: `$XDG_CONFIG_HOME/repoquery/config.toml` or `~/.config/repoquery/config.toml`
3. Environment variable: `REPOQUERY_<SECTION>_<KEY>` (e.g., `REPOQUERY_SYNC_PARALLEL_WORKERS`)
4. CLI flag: `--<key>` (highest priority)

### Security

- All tokens redacted in logs via `CredentialManager::redact()`
- Path traversal validation on config paths
- SQLite PRAGMA integrity_check on startup
- Token format validation on credential setup
- `--version` includes security advisory contact
- CI runs `cargo audit` on every push

## Data Flow

### Sync Flow
```
GitHub API → GraphQL Adapter → Repository struct → SyncCache → RepoStore
                  ↕                              ↕
            ETag/304 handling            Anomaly detection
```

### Query Flow
```
CLI args → RepoFilter → RepoStore::list_repos → in-memory sort/limit → Formatter → stdout
```

### Generation Flow
```
CanonicalData → Tera templates → LIST.md, TABLE.md, ARCHIVE.md
```
