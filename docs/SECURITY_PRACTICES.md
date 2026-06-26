# Security Practices

## Threat Model

- **Asset**: GitHub personal access tokens with `repo` and `read:user` scopes
- **Adversary**: Malicious process on same machine, supply-chain attack on dependencies
- **Risk**: Token exfiltration via logs, credential files, or process memory

## Implemented Controls

| # | Control | Location | Source |
|---|---------|----------|--------|
| 1 | Token format validation on credential setup | `od-core/src/config/credentials.rs` | C-3 |
| 2 | Token scope validation (read:user) | `od-core/src/config/credentials.rs` | C-3 |
| 3 | Log redaction — all tracing:: calls strip tokens | `od-sync/src/client.rs` | C-1, C-4 |
| 4 | Clippy deny-level lints (unwrap_used, panic) | `clippy.toml`, workspace `Cargo.toml` | H-4 |
| 5 | DB integrity check on startup (PRAGMA integrity_check) | `od-store/src/sqlite.rs` | H-1 |
| 6 | Path traversal validation for config paths | `od-core/src/config/mod.rs` | H-2 |
| 7 | Cargo audit in CI | `.github/workflows/ci.yml` | C-2 |
| 8 | Security banner in --version | `repoquery/src/main.rs` | M-1 |

## Token Protection

### Storage

Tokens can be stored via:

1. **OS keychain** (recommended): macOS Keychain, Windows Credential Manager, Linux Secret Service
2. **Environment variable**: `GITHUB_TOKEN` or `REPOQUERY_CREDENTIALS_GITHUB_TOKEN`
3. **Config file**: `~/.config/repoquery/credentials` (file permissions: 600)

### Redaction

All log output is sanitized. Token patterns detected and redacted:

```
ghp_xxx...  → ghp_***REDACTED*** (...)
gho_xxx...  → gho_***REDACTED*** (...)
ghu_xxx...  → ghu_***REDACTED*** (...)
github_pat_xxx...  → github_pat_***REDACTED*** (...)
```

Verified via test `test_credential_redaction`.

## CI/CD Security

- `cargo audit` runs on every push to detect vulnerable dependencies
- `cargo clippy` runs with deny-level lints to prevent unsafe patterns
- Builds blocked on clippy failures
- No secrets in CI configuration

## Reporting

Report security issues to the advisory contact shown in `repoquery --version`:

```
v0.1.0
Report security issues to: https://github.com/encodedfox/repoquery/security
```
