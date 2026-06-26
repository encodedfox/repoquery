# Security Practices

## Threat Model

- **Asset**: GitHub personal access tokens (classic and fine-grained), repository metadata
- **Adversary**: Malicious process on same machine, supply-chain attack on dependencies, network observer
- **Risks**:
  - Token exfiltration via logs, credential files, or process memory
  - SQL injection via crafted tag/seed values in LIKE clauses
  - TOCTOU races on credential file reads
  - Symlink-based credential file substitution attacks
  - Concurrent sync corruption (write-write races on canonical data)
  - GraphQL error responses leaking sensitive data

## Implemented Controls

| # | Control | Location | Source |
|---|---------|----------|--------|
| 1 | Token format validation on credential setup | `rq-core/src/config/credentials.rs` | C-3 |
| 2 | Token scope validation (repo, public_repo) | `repoquery/src/commands/configure.rs` | Phase 1.9 |
| 3 | Log redaction — all tracing:: calls strip tokens | `rq-sync/src/client.rs` | C-1, C-4 |
| 4 | Clippy deny-level lints (unwrap_used, panic) | `clippy.toml`, workspace `Cargo.toml` | H-4 |
| 5 | DB integrity check on startup (PRAGMA integrity_check) | `rq-store/src/sqlite.rs` | H-1 |
| 6 | Path traversal validation for config paths | `rq-core/src/config/mod.rs` | H-2 |
| 7 | Cargo audit in CI | `.github/workflows/ci.yml` | C-2 |
| 8 | Security banner in --version | `repoquery/src/main.rs` | M-1 |
| 9 | FGAT token hashing (SHA-256) — raw tokens never persisted | `rq-core/src/models/seed.rs` | Phase 1.1 |
| 10 | UUID-based token IDs derived from hash prefix | `rq-core/src/models/seed.rs` | Phase 1.2 |
| 11 | Credential file symlink rejection + TOCTOU defense | `rq-core/src/config/credentials.rs` | Phase 1.4 |
| 12 | SQL LIKE wildcard escape in tag filter | `rq-store/src/sqlite.rs` | Phase 1.6 |
| 13 | HTTP client timeout enforcement (sync + configure) | `rq-sync/src/adapters/graphql.rs`, `repoquery/src/commands/configure.rs` | Phase 1.7, 1.8 |
| 14 | Required token scope enforcement (repo, public_repo) | `repoquery/src/commands/configure.rs` | Phase 1.9 |
| 15 | GraphQL error message redaction via `redact_sensitive()` | `rq-sync/src/adapters/graphql.rs` | Phase 2.9 |
| 16 | Hard token deletion (`delete_fgat_token`) | `rq-store/src/traits.rs`, `rq-store/src/sqlite.rs` | Phase 3.10 |
| 17 | File lock preventing concurrent sync operations | `rq-sync/src/orchestrator.rs` (fs2) | Phase 3.11 |
| 18 | Stub adapter error returns (no panics in production) | `rq-sync/src/adapters/gitlab.rs`, `codeberg.rs` | Phase 1.5 |

## Token Protection

### Storage

Tokens can be stored via:

1. **OS keychain** (recommended): macOS Keychain, Windows Credential Manager, Linux Secret Service
2. **Environment variable**: `GITHUB_TOKEN` or `REPOQUERY_CREDENTIALS_GITHUB_TOKEN`
3. **Config file**: `~/.config/repoquery/credentials` (file permissions: 600, symlink rejected)

### FGAT Token Lifecycle

Fine-Grained Access Tokens follow a strict lifecycle:

1. **Add**: Raw token is SHA-256 hashed. Only the hash is persisted to SQLite.
   - `FgatToken.token_hash` = hex(SHA-256(raw_token))
   - `FgatToken.raw_token` is held in memory only (`#[serde(skip)]`)
   - Token ID = first 8 hex chars of the hash (UUID derived, not token-prefix)
2. **Use**: `FgatPool::acquire()` prefers `raw_token` (in-memory) with fallback to `token_hash`
3. **Exhaust**: Status set to `"exhausted"` via `update_fgat_token_status()`
4. **Delete**: Row removed from SQLite via `delete_fgat_token()` (hard delete, not soft-delete)

### Redaction

All log output is sanitized. Token patterns detected and redacted:

```
ghp_xxx...  → ghp_***REDACTED*** (...)
gho_xxx...  → gho_***REDACTED*** (...)
ghu_xxx...  → ghu_***REDACTED*** (...)
github_pat_xxx...  → github_pat_***REDACTED*** (...)
```

GraphQL API error responses are also redacted before being included in error messages.

### Credential File Security

- Symlinks are rejected (`symlink_metadata` — does not follow symlinks)
- Unix file permissions checked (must be 0600 or stricter)
- TOCTOU minimized: permission check and read use the same `symlink_metadata` result

## Network Hardening

### HTTP Timeouts

All HTTP clients enforce configurable timeouts:

- **Sync adapter**: `request_timeout_secs` from `SyncConfig` (default: 30s)
- **Configure validation**: 10s fixed timeout
- Configurable via `REPOQUERY_SYNC_REQUEST_TIMEOUT_SECS` env var or `config set sync.request_timeout_secs`

### Concurrent Sync Protection

An exclusive file lock (`fs2::FileExt::try_lock_exclusive`) prevents concurrent sync operations:

- Lock file: `{canonical_path}.lock` (e.g., `data/canonical/repositories.yml.lock`)
- RAII guard `SyncLock` releases on drop (including panic)
- Applies to all sync entry points: `sync_all`, `sync_by_relation`, `sync_org_repos`, `sync_specific`

### SQL Injection Prevention

Tag filter values are escaped before LIKE pattern matching:

- `\` → `\\`, `%` → `\%`, `_` → `\_`
- `ESCAPE '\'` clause added to the LIKE expression

## CI/CD Security

- `cargo audit` runs on every push to detect vulnerable dependencies
- `cargo clippy` runs with deny-level lints to prevent unsafe patterns
- Builds blocked on clippy failures
- No secrets in CI configuration

## Dependency Audit: RSA (Marvin Attack)

The `rsa` crate (v0.9.10) is a **transitive** dependency:

```
octocrab → jsonwebtoken → rsa
```

- **Advisory**: RUSTSEC-2023-0071 (CVE-2023-33285) — Marvin Attack timing side-channel
- **Status**: No patched version available; `rsa` crate is unmaintained
- **Exposure**: We do NOT use RSA directly. It is pulled in by `jsonwebtoken` for JWT verification only.
- **Mitigation**: Advisory explicitly allowed in `deny.toml`. No newer advisory exists for `rsa` as of 2026-06.

## Reporting

Report security issues to the advisory contact shown in `repoquery --version`:

```
v0.1.0
Report security issues to: https://github.com/encodedfox/repoquery/security
```
