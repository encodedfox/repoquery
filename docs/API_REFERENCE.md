# API Reference

## CLI Interface

OmniDatum provides a comprehensive command-line interface built with Clap 4.5. All commands follow the pattern:

```bash
omnidatum-processor <COMMAND> [OPTIONS]
```

### Global Options

```bash
-h, --help     Print help information
-V, --version  Print version information
```


## Sync Commands

### `sync` - Synchronize Repository Metadata

Fetches repository metadata from external sources like GitHub API.

**Usage:**
```bash
omnidatum-processor sync [OPTIONS]
```

**Options:**
```bash
-i, --input <INPUT>          Path to canonical data file [default: data/canonical/repositories.yml]
    --repos <REPOS>          Sync specific repositories (comma-separated owner/name)
    --force                  Force sync even if cached
    --dry-run                Preview what would be synced
-v, --verbose                Verbose output with per-repo details
    --clear-cache            Clear cache before syncing
```

**Examples:**
```bash
# Sync all repositories
cargo run -- sync

# Sync specific repositories
cargo run -- sync --repos "rust-lang/rust,facebook/react"

# Force refresh (ignore cache)
cargo run -- sync --force

# Preview changes
cargo run -- sync --dry-run

# Verbose logging
cargo run -- sync --verbose

# Clear cache first
cargo run -- sync --clear-cache
```

**Output:**
```
🔄 Sync Repository Metadata
🚀 Starting sync...
[00:02:15] ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░ 250/845 ✅ rust-lang/rust

📊 Sync Results:
  Total: 845
  ✅ Synced: 250
  ⚠️  Cached: 595
  ❌ Failed: 0
  ⏱️  Duration: 2m 15s

✅ Sync completed successfully!
```

### `configure` - Setup Configuration

Configures OmniDatum settings and credentials.

**Usage:**
```bash
omnidatum-processor configure [OPTIONS]
```

**Options:**
```bash
-i, --interactive              Interactive mode (prompt for values) [default: true]
    --github-token <TOKEN>     GitHub token (non-interactive)
    --show                     Show current configuration
```

**Examples:**
```bash
# Interactive setup
cargo run -- configure

# Non-interactive
cargo run -- configure --github-token "ghp_your_token_here"

# View configuration
cargo run -- configure --show
```

**Output (--show):**
```
⚙️  OmniDatum Configuration

📋 Current Configuration:

[sync]
  enabled = false
  interval_hours = 24
  parallel_workers = 3
  cache_ttl_hours = 24
  rate_limit_buffer = 500

[credentials]
  source = File
  file_path = "~/.config/omnidatum/credentials"

🔑 GitHub token: ghp_***REDACTED*** (configured)
📁 Config location: "~/.config/omnidatum/config.toml"
```

### `status` - System Health Check

Shows sync status and system health information.

**Usage:**
```bash
omnidatum-processor status [OPTIONS]
```

**Options:**
```bash
--detailed    Show detailed information including rate limits
```

**Examples:**
```bash
# Basic status
cargo run -- status

# Detailed status
cargo run -- status --detailed
```

**Output:**
```
📊 OmniDatum Status

Sync Status:
  Last sync: 2025-12-11 12:30:00 UTC (2 hours ago)
  Repositories cached: 845

Cache:
  Entries: 845
  TTL: 24 hours
  Oldest entry: 12 hours ago

Configuration:
  Sync enabled: false
  Parallel workers: 3
  Rate limit buffer: 500

Credentials:
  ✅ GitHub token: ghp_***REDACTED*** (configured)
```

### `migrate-credentials` - Legacy Token Migration

Migrates credentials from legacy location to secure storage.

**Usage:**
```bash
omnidatum-processor migrate-credentials [OPTIONS]
```

**Options:**
```bash
--from <PATH>        Path to legacy token file
--delete-source      Delete source file after migration
```

**Examples:**
```bash
# Migrate from stargazer
cargo run -- migrate-credentials --from ../stargazer/.github/.token

# Migrate and delete source
cargo run -- migrate-credentials \
  --from ~/.github/.token \
  --delete-source
```

**Output:**
```
🔄 Migrating Credentials
  From: ../stargazer/.github/.token

⚠️  Warning: Source file has insecure permissions (644)
   Consider: chmod 600 ../stargazer/.github/.token

✅ Migration successful!
   Token: ghp_***REDACTED***
   New location: File

💡 Source file preserved at: ../stargazer/.github/.token
   You can safely delete it now.
```

## Commands

### `parse` - Extract Data from Markdown

Parses existing LIST.md and TABLE.md files into canonical YAML format.

**Usage:**
```bash
omnidatum-processor parse [OPTIONS]
```

**Options:**
```bash
-l, --list <LIST>      Path to LIST.md file [default: LIST.md]
-t, --table <TABLE>    Path to TABLE.md file [default: TABLE.md]  
-o, --output <OUTPUT>  Output path for canonical data [default: data/canonical/repositories.yml]
```

**Examples:**
```bash
# Parse default files
cargo run -- parse

# Parse custom files
cargo run -- parse --list custom_list.md --output repos.yml

# Parse with full paths
cargo run -- parse \
  --list /path/to/LIST.md \
  --table /path/to/TABLE.md \
  --output /path/to/output.yml
```

**Output Format:**
```yaml
repositories:
  - id: "gin-gonic/gin"
    platforms:
      - platform: "GitHub"
        url: "https://github.com/gin-gonic/gin"
        status: "Active"
    metadata:
      name: "Gin"
      description: "High-performance HTTP web framework for Go"
      owner: "gin-gonic"
      license: "MIT"
    classification:
      language: "Go"
      topics: ["web", "framework", "http"]
    quality_metrics:
      stars: 87313
      quality_score: 95
```

### `validate` - Quality Assurance

Validates canonical data against built-in and custom rules.

**Usage:**
```bash
omnidatum-processor validate [OPTIONS]
```

**Options:**
```bash
-i, --input <INPUT>                    Path to canonical data file [default: data/canonical/repositories.yml]
-o, --output <OUTPUT>                  Output path for validation report [default: data/cache/validation_report.json]
    --check-external-consistency       Check external data consistency (E007, E008) [default: true]
```

**Examples:**
```bash
# Validate default file
cargo run -- validate

# Validate with custom paths
cargo run -- validate \
  --input data/canonical/merged_data.yml \
  --output reports/validation.json

# Validate with detailed logging
RUST_LOG=debug cargo run -- validate
```

**Validation Rules:**

| Code | Rule | Severity | Description |
|------|------|----------|-------------|
| E001 | NoDuplicateRepos | Error | Prevents duplicate repository URLs |
| E002 | MissingLicense | Warning | Flags repos without license info |
| E003 | ValidUrls | Error | Validates URL formats |
| E004 | ReadmeCrossReference | Warning | Checks cross-reference integrity |
| E005 | PlatformMigration | Warning | Verifies migration completeness |
| E006 | MissingMetadata | Info | Identifies missing descriptions |
| E007 | ExternalDataConsistent | Error/Warning | Validates synced data consistency |
| E008 | DuplicateRepositoryName | Error | Detects duplicate full_names |
| - | StaleContent | Info | Flags content >2 years old |

**Output Format:**
```json
{
  "summary": {
    "total_repositories": 845,
    "errors": 0,
    "warnings": 177,
    "info": 21,
    "pass_rate": 100.0
  },
  "issues": [
    {
      "rule": "MissingLicense",
      "severity": "Warning",
      "message": "Repository missing license information",
      "repository_id": "example/repo",
      "details": {
        "url": "https://github.com/example/repo"
      }
    }
  ],
  "statistics": {
    "by_language": {"Go": 276, "Python": 124},
    "by_platform": {"GitHub": 844, "Codeberg": 1},
    "by_status": {"Active": 797, "Archived": 48}
  }
}
```

### `generate` - Create Output Files

Generates markdown documentation from canonical data using Tera templates.

**Usage:**
```bash
omnidatum-processor generate [OPTIONS]
```

**Options:**
```bash
-i, --input <INPUT>     Path to canonical data file [default: data/canonical/repositories.yml]
-o, --output <OUTPUT>   Output directory [default: current directory]
--format <FORMAT>       Output format [default: all] [possible values: list, table, archive, all]
--include-stats         Include statistics footer
```

**Examples:**
```bash
# Generate all formats
cargo run -- generate

# Generate specific format
cargo run -- generate --format list

# Generate with statistics
cargo run -- generate --include-stats

# Generate to custom directory
cargo run -- generate --output /path/to/docs/
```

**Generated Files:**

| File | Format | Content |
|------|--------|---------|
| `LIST.md` | Bullet list | Active repositories grouped by language |
| `TABLE.md` | Table | Active repositories in tabular format |
| `ARCHIVE.md` | Bullet list | Archived repositories |
| `ARCHIVE_TABLE.md` | Table | Archived repositories in tabular format |

### `merge` - Combine Data Sources

Merges multiple data sources into unified canonical format.

**Usage:**
```bash
omnidatum-processor merge [OPTIONS]
```

**Options:**
```bash
-b, --base <BASE>         Base repositories file [default: data/canonical/repositories.yml]
-m, --manual <MANUAL>     Manual additions file [default: data/canonical/manual_additions.yml]
-w, --web-refs <WEB>      Web references file [default: data/canonical/web_references.yml]
-k, --books <BOOKS>       Books file [default: data/canonical/books.yml]
-o, --output <OUTPUT>     Output file [default: data/canonical/merged_data.yml]
-s, --strategy <STRATEGY> Merge strategy [default: prefer-manual] [possible values: prefer-manual, prefer-starred, strict]
```

**Merge Strategies:**

- **prefer-manual**: Manual additions override starred data (default)
- **prefer-starred**: Starred data takes precedence
- **strict**: Reject conflicts, require manual resolution

**Examples:**
```bash
# Merge with default strategy
cargo run -- merge

# Merge with strict conflict resolution
cargo run -- merge --strategy strict

# Merge custom files
cargo run -- merge \
  --base repos.yml \
  --manual additions.yml \
  --output combined.yml
```

### `stats` - Display Statistics

Shows comprehensive statistics about the repository dataset.

**Usage:**
```bash
omnidatum-processor stats [OPTIONS]
```

**Options:**
```bash
-i, --input <INPUT>    Path to canonical data file [default: data/canonical/repositories.yml]
-f, --format <FORMAT>  Output format [default: table] [possible values: table, json, yaml]
--detailed             Show detailed breakdown
```

**Examples:**
```bash
# Show basic statistics
cargo run -- stats

# Show detailed breakdown
cargo run -- stats --detailed

# Output as JSON
cargo run -- stats --format json

# Analyze specific file
cargo run -- stats --input data/canonical/merged_data.yml
```

**Statistics Output:**
```
Repository Statistics
═══════════════════════

Total Repositories: 845
├── Active: 797 (94.3%)
└── Archived: 48 (5.7%)

Top Languages:
├── Go: 276 (32.7%)
├── Python: 124 (14.7%)
├── Rust: 80 (9.5%)
├── JavaScript: 65 (7.7%)
└── TypeScript: 45 (5.3%)

Platform Distribution:
├── GitHub: 844 (99.9%)
└── Codeberg: 1 (0.1%)

Quality Metrics:
├── High Quality (80-100): 443 (52.4%)
├── Medium Quality (60-79): 254 (30.1%)
└── Lower Quality (0-59): 148 (17.5%)
```

## Library API

For programmatic usage, OmniDatum exposes a Rust library API.

### Core Types

```rust
use omnidatum_processor::{CanonicalData, Repository, ValidationResult};

// Load canonical data
let data = CanonicalData::from_file("data/canonical/merged_data.yml")?;

// Access repositories
for repo in &data.repositories {
    println!("{}: {}", repo.id, repo.metadata.description);
}
```

### Validation API

```rust
use omnidatum_processor::{Validator, ValidationRule};

// Create validator with built-in rules
let mut validator = Validator::new();
validator.add_default_rules();

// Add custom rule
validator.add_rule(Box::new(CustomRule::new()));

// Validate data
let report = validator.validate(&data)?;
println!("Errors: {}, Warnings: {}", report.error_count(), report.warning_count());
```

### Generation API

```rust
use omnidatum_processor::MarkdownGenerator;

// Create generator
let generator = MarkdownGenerator::new("data/templates")?;

// Generate specific format
let list_content = generator.generate_list(&data, true)?; // with stats
let table_content = generator.generate_table(&data, false)?; // without stats

// Write to files
std::fs::write("LIST.md", list_content)?;
std::fs::write("TABLE.md", table_content)?;
```

### Cross-Reference API

```rust
use omnidatum_processor::{CrossReferenceGraph, NodeType};

// Build cross-reference graph
let graph = CrossReferenceGraph::from_canonical(&data)?;

// Query relationships
let related_repos = graph.find_related_repositories("web-frameworks")?;
let sections = graph.find_sections_for_repository("gin-gonic/gin")?;

// Generate navigation
let nav_links = graph.generate_navigation_links(&data)?;
```

## Error Handling

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Parse error |
| 3 | Validation error (with --strict) |
| 4 | File I/O error |
| 5 | Template error |

### Error Messages

All error messages follow the format:
```
Error: <error-type>: <description>
  Caused by: <root-cause>
  
  Help: <suggested-action>
```

**Example:**
```
Error: ValidationError: Repository validation failed
  Caused by: Duplicate repository URL found: https://github.com/gin-gonic/gin
  
  Help: Remove duplicate entries or use --strategy strict to require manual resolution
```

## Configuration

### Environment Variables

```bash
# Logging level
export RUST_LOG=omnidatum_processor=debug

# Backtrace on panic
export RUST_BACKTRACE=1

# Template directory override
export OMNIDATUM_TEMPLATES_DIR=/custom/templates

# Cache directory override  
export OMNIDATUM_CACHE_DIR=/custom/cache
```

### Configuration File (Planned)

Future versions will support `omnidatum.toml`:

```toml
[validation]
strict_mode = false
rules = ["all"]
ignore_warnings = false

[generation]
include_stats = true
template_dir = "data/templates"
output_dir = "."

[merge]
default_strategy = "prefer-manual"
conflict_resolution = "interactive"
```

## Performance Tuning

### Optimization Flags

```bash
# Release build for maximum performance
cargo build --release

# Profile-guided optimization (requires nightly)
cargo build --release -Z build-std

# Link-time optimization
RUSTFLAGS="-C lto=fat" cargo build --release
```

### Memory Usage

```bash
# Monitor memory usage
/usr/bin/time -v cargo run --release -- generate

# Reduce memory usage for large datasets
cargo run --release -- generate --streaming
```

### Parallel Processing

```bash
# Enable parallel validation (default)
cargo run -- validate --parallel

# Disable for debugging
cargo run -- validate --no-parallel
```

---

*API Reference - Last updated: 2025-12-10*
