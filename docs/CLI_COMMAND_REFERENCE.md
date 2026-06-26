# CLI Command Reference

## Overview

```
repoquery <COMMAND> [options]
```

## Global Options

| Flag | Description |
|------|-------------|
| `-h`, `--help` | Print help information |
| `-V`, `--version` | Print version and security contact |

## Commands

### `parse` — Parse existing LIST.md and TABLE.md into canonical format

```
repoquery parse [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-l`, `--list` | `LIST.md` | Path to LIST.md file |
| `-t`, `--table` | `TABLE.md` | Path to TABLE.md file |
| `-o`, `--output` | `data/canonical/repositories.yml` | Output path for canonical data |

---

### `validate` — Validate canonical data

```
repoquery validate [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-i`, `--input` | `data/canonical/repositories.yml` | Path to canonical data file |
| `-o`, `--output` | `data/cache/validation_report.json` | Output path for validation report |
| `--check-external-consistency` | `true` | Check external data consistency |

---

### `generate` — Generate markdown documents from canonical data

```
repoquery generate [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-i`, `--input` | `data/canonical/repositories.yml` | Path to canonical data file |
| `--include-archived` | `true` | Include archived repos in separate files |
| `--validate` | `false` | Validate data before generation |
| `--validate-sync-data` | `true` | Validate sync data integrity |
| `--platforms` | `false` | Include multi-platform info |
| `--collection` | — | Generate for a specific collection |

---

### `merge` — Merge manual additions into canonical data

```
repoquery merge [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-b`, `--base` | `data/canonical/repositories.yml` | Path to base canonical data |
| `-m`, `--manual` | `data/canonical/manual_additions.yml` | Path to manual additions |
| `-o`, `--output` | `data/canonical/repositories.yml` | Output path for merged data |

---

### `stats` — Show statistics about repository data

```
repoquery stats [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-i`, `--input` | `data/canonical/repositories.yml` | Path to canonical data file |
| `--enhanced-json` | `false` | Output as enhanced JSON |
| `--diff` | — | Compare with previous statistics file |

---

### `configure` — Configure settings and credentials

```
repoquery configure [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-i`, `--interactive` | `true` | Interactive mode |
| `--github-token` | — | GitHub token (non-interactive) |
| `--show` | `false` | Show current configuration |

---

### `migrate-credentials` — Migrate credentials from legacy location

```
repoquery migrate-credentials [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--from` | — | Path to legacy token file |
| `--delete-source` | `false` | Delete source after migration |

---

### `sync` — Sync repository metadata from external sources

```
repoquery sync [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--repos` | — | Specific repos (comma-separated owner/name) |
| `--force` | `false` | Force sync even if cached |
| `--dry-run` | `false` | Show what would be synced |
| `-v`, `--verbose` | `false` | Detailed output |
| `--clear-cache` | `false` | Clear cache before syncing |
| `-i`, `--input` | `data/canonical/repositories.yml` | Path to canonical data |
| `--relations` | — | Relation types to sync (starred,owned,forked,watching) |
| `--check-forks` | `false` | Check fork status (ahead/behind) |

---

### `status` — Show sync status and system health

```
repoquery status [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--detailed` | `false` | Show detailed information |

---

### `import` — Import data between store formats

```
repoquery import [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--from` | `data/canonical/repositories.yml` | Source path |
| `--to` | `data/repoquery.db` | Destination path |

---

### `export` — Export data between store formats

```
repoquery export [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--from` | `data/repoquery.db` | Source path |
| `--to` | `data/canonical/repositories.yml` | Destination path |

---

### `collections` — Manage repository collections

#### Subcommands

##### `list` — List all collections
```
repoquery collections list
```

##### `create` — Create a new collection
```
repoquery collections create --name <name> [--description <desc>]
```

##### `show` — Show collection details
```
repoquery collections show --id <id>
```

##### `add` — Add a repository to a collection
```
repoquery collections add --collection <id> --repo <owner/name>
```

##### `remove` — Remove a repository from a collection
```
repoquery collections remove --collection <id> --repo <owner/name>
```

##### `delete` — Delete a collection
```
repoquery collections delete --id <id>
```

##### `auto-generate` — Auto-generate collections from GitHub topics
```
repoquery collections auto-generate [--min-repos 3] [--store path]
```

---

### `tui` — Interactive terminal UI

```
repoquery tui [--store path]
```

Keyboard-driven TUI for browsing and managing repositories. Built with ratatui and crossterm.

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |

---

### `config` — Manage configuration

#### Subcommands

##### `init` — Initialize default configuration
```
repoquery config init
```

##### `show` — Show current configuration
```
repoquery config show
```

##### `set` — Set a configuration value
```
repoquery config set <key> <value>
```

---

### `activity` — Analyze repository activity

#### Subcommands

##### `overview` — Activity breakdown for all repositories
```
repoquery activity overview [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--active-months` | `3` | Active threshold in months |
| `--stale-months` | `12` | Stale threshold in months |
| `--chart` | — | Show histogram chart |
| `--format` | `table` | Output format (table, json, md, csv) |

##### `stale` — List stale/abandoned repositories
```
repoquery activity stale [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--stale-threshold` | `12` | Stale threshold in months |
| `--format` | `table` | Output format |

##### `active` — List active/maintained repositories
```
repoquery activity active [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--active-threshold` | `3` | Active threshold in months |
| `--format` | `table` | Output format |

##### `trending` — Repositories with fastest star growth
```
repoquery activity trending [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--since` | `90` | Number of days to consider |
| `--limit` | `20` | Maximum results |

---

### `query` — Query repositories with filters

#### Subcommands

##### `list` — Filtered repository list
```
repoquery query list [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--language` | — | Filter by primary language |
| `--min-stars` | — | Minimum star count |
| `--max-stars` | — | Maximum star count |
| `--owner` | — | Filter by owner |
| `--license` | — | Filter by SPDX license identifier |
| `--topic` | — | Filter by topic |
| `--source` | — | Filter by source (github/gitlab/codeberg) |
| `--sort` | `stars` | Sort field (stars, name, updated, quality) |
| `--order` | `desc` | Sort order (asc, desc) |
| `--limit` | — | Maximum results |
| `--format` | `table` | Output format (table, json, md, csv) |

##### `search` — Full-text search
```
repoquery query search <query> [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--limit` | — | Maximum results |
| `--sort` | `stars` | Sort field |
| `--format` | `table` | Output format |

##### `show` — Show repository details
```
repoquery query show <owner/repo> [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--format` | `table` | Output format (table, json, md) |

##### `topics` — List all topics with repo counts
```
repoquery query topics [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--min-repos` | `1` | Minimum repos with a topic |
| `--limit` | — | Maximum topics to show |

##### `languages` — List all languages with repo counts
```
repoquery query languages [options]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--store` | `data/repoquery.db` | Path to data store |
| `--min-repos` | `1` | Minimum repos with a language |
| `--limit` | — | Maximum languages to show |

---

### `repo` — Manage individual repository metadata

#### Subcommands

##### `tag` — Add a tag to a repository
```
repoquery repo tag --repo <owner/name> --tag <name> [--store path]
```

##### `untag` — Remove a tag from a repository
```
repoquery repo untag --repo <owner/name> --tag <name> [--store path]
```

##### `note` — Set a curator note
```
repoquery repo note --repo <owner/name> --text <note> [--store path]
```

##### `show` — Show repository details
```
repoquery repo show --repo <owner/name> [--store path]
```

## Output Formats

| Format | Flag | Description |
|--------|------|-------------|
| Table | `--format table` | Terminal-formatted table with column alignment and truncation |
| JSON | `--format json` | Standard JSON array |
| Markdown | `--format md` | GitHub-flavored Markdown table |
| CSV | `--format csv` | RFC 4180 CSV with headers |

## Activity Classification

| Status | Criteria | Description |
|--------|----------|-------------|
| Active | Last commit < 3 months | Regular updates |
| Maintained | Last commit 3–12 months | Periodic maintenance |
| Stale | Last commit 12–24 months | Infrequent or no updates |
| Abandoned | Last commit > 24 months | No longer maintained |
| Unknown | No date available | Insufficient metadata |

Thresholds configurable via `--active-months` and `--stale-months` flags.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Runtime error (IO, network, config) |
| 2 | CLI usage error (invalid arguments) |
