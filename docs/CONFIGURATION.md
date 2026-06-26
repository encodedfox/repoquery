# Configuration

## Config Priority Chain

Values are resolved in this order (highest priority wins):

1. **CLI flags**: `--parallel-workers 5`
2. **Environment variables**: `REPOQUERY_SYNC_PARALLEL_WORKERS=5`
3. **Config file**: `~/.config/repoquery/config.toml`
4. **Default values** (lowest priority)

## Config File

### Location

XDG-compliant discovery order:

1. `$XDG_CONFIG_HOME/repoquery/config.toml`
2. `~/.config/repoquery/config.toml`

Credentials stored separately at `~/.config/repoquery/credentials`.

### Init

```bash
repoquery config init
```

Creates `~/.config/repoquery/config.toml` with defaults.

### Show

```bash
repoquery config show
```

### Set

```bash
repoquery config set sync.parallel_workers 5
repoquery config set storage.mode sqlite
```

### Example

```toml
[storage]
mode = "yaml"                        # yaml | sqlite | dual
yaml_path = "data/canonical/repositories.yml"
sqlite_path = "data/repoquery.db"

[sync]
enabled = true
interval_hours = 24
parallel_workers = 3
cache_ttl_hours = 24
rate_limit_buffer = 500

[credentials]
source = "Env"                       # Env | File | Keychain
file_path = "~/.config/repoquery/credentials"

[github]
token = ""
```

## Environment Variables

All config keys can be overridden with `REPOQUERY_<SECTION>_<KEY>`:

| Variable | Config Key | Default |
|----------|------------|---------|
| `REPOQUERY_STORAGE_MODE` | `storage.mode` | `yaml` |
| `REPOQUERY_SYNC_ENABLED` | `sync.enabled` | `false` |
| `REPOQUERY_SYNC_INTERVAL_HOURS` | `sync.interval_hours` | `24` |
| `REPOQUERY_SYNC_PARALLEL_WORKERS` | `sync.parallel_workers` | `3` |
| `REPOQUERY_SYNC_CACHE_TTL_HOURS` | `sync.cache_ttl_hours` | `24` |
| `REPOQUERY_SYNC_RATE_LIMIT_BUFFER` | `sync.rate_limit_buffer` | `500` |
| `REPOQUERY_CREDENTIALS_SOURCE` | `credentials.source` | `Env` |
| `REPOQUERY_CREDENTIALS_FILE_PATH` | `credentials.file_path` | — |
| `REPOQUERY_GITHUB_TOKEN` | `github.token` | — |
| `GITHUB_TOKEN` | (fallback) | — |

## Credentials

Three credential sources:

| Source | Method | Priority |
|--------|--------|----------|
| `Env` | `GITHUB_TOKEN` or `REPOQUERY_GITHUB_TOKEN` env var | 1 (highest) |
| `File` | Token file at configurable path | 2 |
| `Keychain` | OS keychain (macOS, Windows, Linux) | 3 |

Configure via:
```bash
repoquery configure                     # interactive
repoquery configure --github-token ghp_xxx  # non-interactive
```

## Validation

Config validation rules:

- `parallel_workers`: 1–10
- `cache_ttl_hours`: 1–168
- `rate_limit_buffer`: 0–1000
- `storage.mode`: one of `yaml`, `sqlite`, `dual`
