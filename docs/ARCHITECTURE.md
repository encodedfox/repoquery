# Architecture

## Overview

RepoQuery (binary: `repoquery`) is a Rust CLI for managing GitHub starred repositories. It follows a layered architecture with strict dependency ordering.

```
┌──────────────────────────────────────────────────┐
│                   repoquery                       │
│         CLI binary (clap), commands, output       │
│             output formatters, TUI                │
├──────────────────────────────────────────────────┤
│ rq-store  │  rq-sync   │ rq-validate │  rq-graph  │
│ YAML/SQL  │  adapters  │  9 rules    │  petgraph  │
│ Dual mode │  cache     │  E001-E008  │  cross-ref │
│           │  file lock  │            │            │
├──────────────────────────────────────────────────┤
│                rq-generate                        │
│         Tera templates, markdown output           │
├──────────────────────────────────────────────────┤
│                    rq-core                        │
│  Types, config, models, parsers, merge, activity  │
└──────────────────────────────────────────────────┘
```

## Crate Layering

| Layer | Crate | Dependencies | Responsibility |
|-------|-------|--------------|----------------|
| 1 | `rq-core` | serde, chrono, regex, toml, sha2, hex | Pure types, config, models, parsers, merge logic, activity classification, token hashing |
| 2 | `rq-generate` | rq-core, tera, anyhow | Tera-based markdown generation from canonical data |
| 3 | `rq-store` | rq-core, rusqlite, serde_yml | Trait-based persistence: YAML, SQLite, Dual-mode; FGAT token CRUD |
| 4 | `rq-sync` | rq-core, octocrab, reqwest, tokio, fs2 | Sync orchestrator, GitHub adapters, GraphQL, caching, file lock |
| 5 | `rq-validate` | rq-core, anyhow | Validation framework with 9 pluggable rules |
| 6 | `rq-graph` | rq-core, petgraph | Cross-reference graph and navigation |
| 7 | `repoquery` | All above, clap, ratatui | CLI binary with command handlers, output formatters, TUI |

**Dependency flow** (strict one-way):
```
rq-core ← rq-store, rq-sync, rq-validate, rq-generate, rq-graph ← repoquery
```

## Persistence Layer

### RepoStore Trait

The `RepoStore` trait in `rq-store` defines the storage interface for repositories:

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

### GraphStore Trait (FGAT Tokens)

The `GraphStore` trait in `rq-store` defines the FGAT token management interface:

```
                     ┌──────────────┐
                     │  GraphStore  │
                     │   (trait)    │
                     └──────┬───────┘
                            │
              ┌─────────────┼─────────────────┐
              ▼             ▼                  ▼
        ┌──────────┐ ┌──────────┐      ┌──────────────┐
        │Add FGAT  │ │List FGATs│      │ Delete FGAT  │
        │  token   │ │  tokens  │      │    token     │
        └──────────┘ └──────────┘      └──────────────┘
              │             │                  │
              ▼             ▼                  ▼
        ┌───────────────────────────────────────────┐
        │             SqliteStore                    │
        │  fgat_tokens(id, platform, token_hash,     │
        │    status, requests_used, rate_limit_*,    │
        │    last_used_at, added_at, expires_at,     │
        │    notes)                                  │
        └───────────────────────────────────────────┘
```

FGAT token operations:
- `add_fgat_token()` — INSERT with SHA-256 hashed token
- `list_fgat_tokens()` — SELECT all (raw_token is None, not persisted)
- `update_fgat_token_status()` — UPDATE status (available/exhausted/revoked)
- `delete_fgat_token()` — DELETE from database (hard removal)

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

- All tokens redacted in logs via `CredentialManager::redact()` and `redact_sensitive()`
- FGAT tokens SHA-256 hashed; raw tokens never persisted to SQLite
- Token IDs derived from hash prefix (not raw token material)
- Credential file symlink rejection + TOCTOU protection via `symlink_metadata`
- Path traversal validation on config paths
- SQLite PRAGMA integrity_check on startup
- SQL LIKE wildcard escape in tag filters
- Token format validation on credential setup
- Required scope validation (repo, public_repo) during configure
- HTTP client timeouts enforced (sync: 30s default, configure: 10s)
- GraphQL error responses redacted via `redact_sensitive()`
- File lock prevents concurrent sync operations (`fs2` exclusive lock)
- Hard token deletion (DELETE FROM, not soft-delete)
- `--version` includes security advisory contact
- CI runs `cargo audit` on every push
- Stub adapter errors use `Err()` instead of `panic!()`

## Data Flow

### Sync Flow
```
GitHub API → GraphQL Adapter → Repository struct → SyncCache → RepoStore
                  ↕                              ↕
            ETag/304 handling            Anomaly detection
                  ↕
            File lock (SyncLock)
            Prevents concurrent runs
```

### Query Flow
```
CLI args → RepoFilter → RepoStore::list_repos → in-memory sort/limit → Formatter → stdout
```

### Generation Flow
```
CanonicalData → Tera templates → LIST.md, TABLE.md, ARCHIVE.md
```

## FGAT Token Flow

```
expand token add:
  raw_token ──hash_token()──▶ token_hash (SHA-256 hex)
                                │
                                ▼
  ┌─ FgatPool ─────────────────────────────┐
  │  acquire() → AcquiredToken (RAII)     │
  │  Uses raw_token from memory, or       │
  │  falls back to token_hash for legacy  │
  │  tokens.                              │
  └────────────────────────────────────────┘
                                │
  expand token remove:          │
  delete_fgat_token(id) ───▶ DELETE FROM fgat_tokens
```
