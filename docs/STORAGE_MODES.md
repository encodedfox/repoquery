# Storage Modes

repoquery supports three storage modes via the `storage.mode` configuration option:

## YAML Mode (default)

**Config**: `storage.mode = "yaml"`

- Canonical YAML file is the single source of truth
- Data is read from and written to `repositories.yml`
- SQLite database (if present) is read-only cache for queries
- Best for: version control, manual editing, portability

## SQLite Mode

**Config**: `storage.mode = "sqlite"`

- SQLite database is the primary store
- YAML files are export-only or disabled
- Enables fast filtered queries with SQL
- Best for: large datasets, frequent queries, analysis

## Dual Mode

**Config**: `storage.mode = "dual"`

- Both YAML and SQLite stores are updated in sync
- YAML remains the canonical format
- SQLite automatically seeded from YAML on first use (when YAML exists but DB doesn't)
- Best for: migration, backup, development

## Store Selection

The binary automatically determines store type from the file extension:

| Extension | Store Type |
|-----------|------------|
| `.db`, `.sqlite` | SqliteStore |
| `.yml`, `.yaml` | YamlStore |

## Configuration

In `~/.config/repoquery/config.toml`:

```toml
[storage]
mode = "yaml"        # yaml | sqlite | dual
yaml_path = "data/canonical/repositories.yml"
sqlite_path = "data/repoquery.db"
```

Environment variable override: `REPOQUERY_STORAGE_MODE=sqlite`

## Import/Export

Convert between formats at any time:

```bash
# YAML → SQLite
repoquery import --from data/canonical/repositories.yml --to data/repoquery.db

# SQLite → YAML
repoquery export --from data/repoquery.db --to data/canonical/repositories.yml
```
