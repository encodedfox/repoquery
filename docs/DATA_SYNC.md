# Data Synchronization Guide

Complete guide to setting up and using OmniDatum's external data synchronization features.

## Overview

OmniDatum's sync system automatically fetches repository metadata from external sources like GitHub API, keeping your documentation up-to-date without manual editing. This eliminates the need to manually maintain star counts, descriptions, and other repository metadata.

### Benefits

- **Automatic Updates**: Star counts, descriptions, and metadata stay current
- **No Manual Editing**: Eliminates error-prone LIST.md editing
- **Smart Caching**: Reduces API calls with intelligent cache management (24-hour TTL default)
- **Rate Limit Safe**: Automatic rate limit handling with 500-request buffer
- **Selective Sync**: Update specific repositories or sync all
- **Preserved Curation**: Manual notes and classifications preserved during sync

### How It Works

```
External API    →    Sync    →    Canonical Data    →    Generated Docs
(GitHub)            (Cache)      (repositories.yml)     (LIST.md, TABLE.md)

1. Check cache (fresh within 24 hours?)
2. If stale, fetch from GitHub API
3. Merge with existing data (preserve manual fields)
4. Update canonical storage
5. Generate documentation
```

**Key Principle**: Unidirectional flow - generated documents never modify canonical data.

## Prerequisites

### Required
- **Rust**: 1.70+ installed via rustup
- **OmniDatum**: Built from source (`cargo build --release`)
- **GitHub Account**: For API access

### GitHub Personal Access Token

You need a GitHub Personal Access Token with `public_repo` scope:

1. Visit: https://github.com/settings/tokens/new
2. Token description: "OmniDatum Sync"
3. Select scopes: **`public_repo`** (read-only access to public repos)
4. Generate token and copy it (you won't see it again!)

**Token Formats**:
- Classic tokens: `ghp_...` (40 characters)
- Fine-grained tokens: `github_pat_...` (93 characters)

## Credential Setup

### Option 1: Interactive Configuration (Recommended)

```bash
cargo run -- configure --interactive
```

This will:
1. Prompt for your GitHub token
2. Store it securely in `~/.config/omnidatum/credentials` (0600 permissions on Unix)
3. Create default configuration

### Option 2: Environment Variable

```bash
# Linux/macOS
export GITHUB_TOKEN="your_token_here"
cargo run -- sync

# Windows PowerShell
$env:GITHUB_TOKEN="your_token_here"
cargo run -- sync
```

### Option 3: Non-Interactive Configuration

```bash
cargo run -- configure --github-token "your_token_here"
```

### Option 4: OS Keychain (macOS/Linux)

```bash
# macOS
security add-generic-password -a omnidatum -s github_token -w "your_token_here"

# Linux
secret-tool store --label="OmniDatum GitHub Token" service omnidatum username github

# Configure OmniDatum to use keychain
# Edit ~/.config/omnidatum/config.toml:
[credentials]
source = "Keychain"
```

### Verify Configuration

```bash
cargo run -- configure --show
```

Output:
```
📋 Current Configuration:
[sync]
  enabled = false
  cache_ttl_hours = 24
  rate_limit_buffer = 500

🔑 GitHub token: ghp_***REDACTED*** (configured)
```

### Migrate Legacy Token

If you have a token in `../stargazer/.github/.token`:

```bash
cargo run -- migrate-credentials --from ../stargazer/.github/.token --delete-source
```

## Configuration

### Config File Location

- **Linux/macOS**: `~/.config/omnidatum/config.toml`
- **Windows**: `%APPDATA%\omnidatum\config.toml`

### Default Configuration

```toml
[sync]
enabled = false                # Enable automatic sync
interval_hours = 24           # Sync every 24 hours
parallel_workers = 3          # Concurrent API requests (1-10)
cache_ttl_hours = 24          # Cache freshness (1-168 hours)
rate_limit_buffer = 500       # Safety buffer (0-1000)
request_timeout_secs = 30     # API request timeout

[credentials]
source = "File"               # Options: Env, File, Keychain
file_path = "~/.config/omnidatum/credentials"

[validation]
rules = ["E001", "E002", "E003", "E004", "E005", "E006", "E007", "E008"]

[generation]
include_archived = true
platform_info = false
stats_detail_level = "Standard"
```

### Customizing Configuration

Edit the config file or use configure command:

```bash
# View current config
cargo run -- configure --show

# After editing config file
cargo run -- sync  # Uses new settings
```

## Usage

### Basic Sync Commands

#### Initial Sync (First Time)

```bash
# Check status first
cargo run -- status

# Configure credentials
cargo run -- configure --interactive

# Initial sync (may take time for large repos)
cargo run -- sync

# Verify results
cargo run -- status
```

#### Regular Sync

```bash
# Sync all repositories
cargo run -- sync

# Force refresh (ignore cache)
cargo run -- sync --force

# Dry run (preview changes)
cargo run -- sync --dry-run
```

#### Selective Sync

```bash
# Sync specific repositories
cargo run -- sync --repos "rust-lang/rust,facebook/react,microsoft/vscode"

# Preview selective sync
cargo run -- sync --repos "rust-lang/rust" --dry-run
```

#### Verbose Mode

```bash
# Detailed logging
cargo run -- sync --verbose

# Shows per-repository:
# - Cache hit/miss
# - API response time
# - Star count changes
# - Rate limit status
```

### Advanced Options

#### Clear Cache

```bash
# Clear cache before syncing
cargo run -- sync --clear-cache

# Check cache status
cargo run -- status
```

#### Check Status

```bash
# Basic status
cargo run -- status

# Detailed status (checks rate limits)
cargo run -- status --detailed
```

Output:
```
📊 OmniDatum Status

Sync Status:
  Last sync: 2025-12-11 12:30:00 UTC (2 hours ago)
  Repositories cached: 845

Cache:
  Entries: 845
  TTL: 24 hours

Configuration:
  Sync enabled: false
  Parallel workers: 3
  Rate limit buffer: 500

Credentials:
  ✅ GitHub token: ghp_***REDACTED*** (configured)
```

## Scheduling Automated Syncs

### Linux/macOS: Cron

```bash
# Edit crontab
crontab -e

# Add line for daily sync at 2 AM
0 2 * * * cd /path/to/omnidatum && /path/to/omnidatum-processor sync

# Or with logging
0 2 * * * cd /path/to/omnidatum && /path/to/omnidatum-processor sync >> /var/log/omnidatum-sync.log 2>&1
```

### Linux: systemd Timer

Create `/etc/systemd/system/omnidatum-sync.service`:
```ini
[Unit]
Description=OmniDatum Repository Sync
After=network.target

[Service]
Type=oneshot
User=your-user
WorkingDirectory=/path/to/omnidatum
ExecStart=/path/to/omnidatum-processor sync
Environment="GITHUB_TOKEN=your_token"

[Install]
WantedBy=multi-user.target
```

Create `/etc/systemd/system/omnidatum-sync.timer`:
```ini
[Unit]
Description=Daily OmniDatum Sync
Requires=omnidatum-sync.service

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable and start:
```bash
sudo systemctl enable omnidatum-sync.timer
sudo systemctl start omnidatum-sync.timer
sudo systemctl status omnidatum-sync.timer
```

### Windows: Task Scheduler

```powershell
# Create scheduled task
$action = New-ScheduledTaskAction -Execute "C:\path\to\omnidatum-processor.exe" -Argument "sync" -WorkingDirectory "C:\path\to\omnidatum"
$trigger = New-ScheduledTaskTrigger -Daily -At 2am
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable
Register-ScheduledTask -Action $action -Trigger $trigger -Settings $settings -TaskName "OmniDatum Sync" -Description "Daily repository metadata sync"
```

## Troubleshooting

### Authentication Failures

**Problem**: `Error: Failed to authenticate with GitHub`

**Solutions**:
```bash
# 1. Verify token is configured
cargo run -- configure --show

# 2. Test token manually
curl -H "Authorization: token YOUR_TOKEN" https://api.github.com/user

# 3. Reconfigure
cargo run -- configure --interactive

# 4. Check token permissions
# Visit https://github.com/settings/tokens
# Ensure public_repo scope is enabled
```

### Rate Limiting

**Problem**: `Rate limit exhausted. Please wait X seconds`

**Solutions**:
```bash
# 1. Check current rate limit status
cargo run -- status --detailed

# 2. Wait for rate limit reset (shown in error message)

# 3. Increase rate limit buffer in config
# Edit ~/.config/omnidatum/config.toml:
[sync]
rate_limit_buffer = 1000  # Larger buffer, syncs fewer repos per run

# 4. Use cache more aggressively
[sync]
cache_ttl_hours = 48  # Cache for 2 days instead of 1
```

**GitHub Rate Limits**:
- **Authenticated**: 5,000 requests/hour
- **Unauthenticated**: 60 requests/hour
- **Buffer**: OmniDatum stops at 500 remaining by default

### Network Errors

**Problem**: `Network timeout` or `Connection refused`

**Solutions**:
```bash
# 1. Check internet connection
ping api.github.com

# 2. Increase timeout in config
[sync]
request_timeout_secs = 60  # Default is 30

# 3. Try again (automatic retry not yet implemented)
cargo run -- sync
```

### Partial Sync Recovery

**Problem**: Sync failed partway through

**Solution**:
```bash
# Cache is automatically saved during sync
# Just run sync again - cached repos will be skipped
cargo run -- sync

# Or force refresh all
cargo run -- sync --force
```

### Cache Issues

**Problem**: Stale cache or corrupted data

**Solutions**:
```bash
# 1. Check cache status
cargo run -- status

# 2. Clear cache
cargo run -- sync --clear-cache

# 3. Force refresh without clearing
cargo run -- sync --force

# 4. Manually delete cache
rm data/cache/sync_metadata.json
```

### Validation Errors After Sync

**Problem**: Validation fails with E007 or E008 errors

**Solutions**:
```bash
# 1. Run validation to see details
cargo run -- validate --check-external-consistency

# 2. Review specific errors
cat data/cache/validation_report.json | jq '.issues[] | select(.code == "E007")'

# 3. Re-sync problematic repos
cargo run -- sync --repos "owner/repo" --force

# 4. Skip sync validation during generation (not recommended)
cargo run -- generate --validate-sync-data=false
```

## Security Considerations

### Token Permissions

**Minimum Required**:
- `public_repo` - Read-only access to public repositories

**NOT Required**:
- Private repo access
- Write permissions
- Admin permissions
- Workflow permissions

### Token Security

**Best Practices**:
1. Use fine-grained tokens with minimal permissions
2. Set token expiration (90 days recommended)
3. Rotate tokens regularly
4. Never commit tokens to version control
5. Use OS keychain when possible

**Storage Security**:
- Environment variables: Secure but must be set per-session
- File storage: 0600 permissions enforced on Unix
- OS keychain: Most secure, requires initial setup

### Token Rotation

```bash
# 1. Generate new token on GitHub
# 2. Update configuration
cargo run -- configure --github-token "new_token_here"

# 3. Verify new token works
cargo run -- status

# 4. Revoke old token on GitHub
```

## Performance Tips

### Optimize Sync Speed

1. **Use Parallel Workers** (requires future implementation):
```toml
[sync]
parallel_workers = 5  # Up to 10 supported
```

2. **Increase Cache TTL** (reduce API calls):
```toml
[sync]
cache_ttl_hours = 48  # Cache for 2 days
```

3. **Selective Sync** (only what changed):
```bash
cargo run -- sync --repos "recently-updated-repos"
```

### Monitor Performance

```bash
# Time sync operation
time cargo run --release -- sync

# Verbose mode for per-repo timing
cargo run -- sync --verbose
```

Expected timing:
- First sync (no cache): ~2-3s per repository
- Cached sync: ~1ms per repository
- Rate limit check: Every 100 repos

## Data Flow

### Architecture

```
External Source              Canonical Storage           Generated Output
──────────────              ──────────────────          ────────────────

GitHub API                  repositories.yml            LIST.md
  ├─ Stars                    ├─ Synced metadata        (read-only)
  ├─ Description              ├─ Manual curation
  ├─ License                  └─ Quality metrics        TABLE.md
  ├─ Topics                                             (read-only)
  ├─ Homepage
  ├─ Archive status
  └─ Last commit

         ↓                           ↓                        ↓
    Sync (cache)              Single source              Generate
```

**Important**: Generated LIST.md and TABLE.md are **output only** - they never feed back into the system. This prevents circular dependencies.

### What Gets Updated

**Synced from GitHub API**:
- ✅ Description
- ✅ Star count
- ✅ License information
- ✅ Topics/tags
- ✅ Homepage URL
- ✅ Archive status
- ✅ Last commit date

**Preserved (Manual Curation)**:
- ✅ `manually_curated` flag
- ✅ `curator_notes`
- ✅ `classification.significance_notes`
- ✅ `classification.readme_inclusion_reason`
- ✅ Custom categories

## Common Workflows

### Daily Development Workflow

```bash
# Morning: sync latest changes
cargo run -- sync

# Validate data quality
cargo run -- validate

# Generate documentation
cargo run -- generate

# Commit changes
git add data/canonical/repositories.yml LIST.md TABLE.md
git commit -m "chore: daily sync and regenerate docs"
```

### First-Time Setup

```bash
# 1. Configure credentials
cargo run -- configure --interactive

# 2. Initial sync (may take 10-15 minutes for 800+ repos)
cargo run -- sync

# 3. Validate results
cargo run -- validate

# 4. Generate documentation
cargo run -- generate

# 5. Review changes
git diff LIST.md
```

### Weekly Maintenance

```bash
# 1. Force refresh all (ignore cache)
cargo run -- sync --force

# 2. Check for validation issues
cargo run -- validate --check-external-consistency

# 3. Review quality report
cat data/cache/validation_report.json | jq '.summary'

# 4. Regenerate all docs
cargo run -- generate
```

### CI/CD Integration

```yaml
# .github/workflows/sync.yml
name: Daily Sync

on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM daily
  workflow_dispatch:  # Manual trigger

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build
        run: cargo build --release
      
      - name: Sync repositories
        env:
          GITHUB_TOKEN: ${{ secrets.OMNIDATUM_GITHUB_TOKEN }}
        run: cargo run --release -- sync
      
      - name: Validate
        run: cargo run --release -- validate
      
      - name: Generate docs
        run: cargo run --release -- generate
      
      - name: Commit changes
        run: |
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add data/canonical/ LIST.md TABLE.md
          git diff --staged --quiet || git commit -m "chore: automated sync [skip ci]"
          git push
```

## Error Reference

### E007: External Data Inconsistent

**Cause**: Synced data doesn't match expected patterns

**Examples**:
- Stars > 1,000,000 (unrealistic)
- Archive status mismatch between platform and local
- Missing description for popular repo (>1000 stars)

**Fix**:
```bash
# Re-sync the repository
cargo run -- sync --repos "owner/repo" --force

# Or review manually
$EDITOR data/canonical/repositories.yml
```

### E008: Duplicate Repository Name

**Cause**: Same full_name exists multiple times

**Fix**:
```bash
# Find duplicates
cargo run -- validate | grep E008

# Review and merge manually
$EDITOR data/canonical/repositories.yml
```

## Advanced Topics

### Selective Sync Examples

```bash
# Sync recently updated repos
cargo run -- sync --repos "rust-lang/rust,tokio-rs/tokio"

# Preview what would sync
cargo run -- sync --dry-run

# Sync with full logging
RUST_LOG=debug cargo run -- sync --verbose
```

### Cache Management

```bash
# View cache status
cargo run -- status

# Cache location
ls -la data/cache/sync_metadata.json

# Cache structure
cat data/cache/sync_metadata.json | jq '.' | head -20
```

### Rate Limit Management

```bash
# Check current rate limit
cargo run -- sync --dry-run
# (Shows rate limit in output)

# Adjust buffer in config
[sync]
rate_limit_buffer = 1000  # Stop when 1000 requests remain
```

### Performance Tuning

```bash
# Increase cache TTL (fewer API calls)
[sync]
cache_ttl_hours = 72  # 3 days

# Reduce timeout (faster failures)
[sync]
request_timeout_secs = 10

# Future: Parallel workers
[sync]
parallel_workers = 5  # Planned feature
```

## Comparison: Manual vs Sync

### Before (Manual Process)
1. Browse GitHub repo
2. Copy star count
3. Edit LIST.md manually
4. Risk: Typos, outdated data, inconsistent formatting
5. Time: ~1 minute per repository

### After (Automated Sync)
1. Run `cargo run -- sync`
2. Automatic: Fetches all metadata
3. Benefit: Always current, no manual errors
4. Time: ~2-3 seconds per repository (first sync), ~1ms (cached)

### Migration Path

```bash
# 1. One-time: Initial sync replaces manual editing
cargo run -- sync

# 2. Ongoing: Update specific repos as needed
cargo run -- sync --repos "owner/repo"

# 3. Legacy: Parse command still available if needed
cargo run -- parse  # Extracts from LIST.md (legacy)
```

## FAQ

### Q: How often should I sync?

**A**: Depends on your needs:
- **Daily**: Most users (via cron/systemd)
- **Weekly**: Less active projects
- **On-demand**: Before major documentation updates
- **Continuous**: CI/CD integration

### Q: What happens if sync fails partway?

**A**: 
- Partial progress is saved to cache
- Re-running sync continues from cache
- No data corruption - atomic writes (planned)

### Q: Can I sync private repositories?

**A**: Not yet supported. Current implementation only syncs public repositories. Private repo support requires:
- Token with `repo` scope (broader permissions)
- Enhanced privacy handling
- (Planned for future release)

### Q: How do I sync from non-GitHub sources?

**A**: See integration guides:
- [Google Sheets](INTEGRATION_GUIDE_GOOGLE_SHEETS.md)
- [Jira](INTEGRATION_GUIDE_JIRA.md)
- [Git repositories](INTEGRATION_GUIDE_GIT_REPOSITORY.md)
- [Custom sources](INTEGRATION_GUIDE_INDEX.md)

### Q: Does sync modify LIST.md directly?

**A**: No! Sync updates `data/canonical/repositories.yml` (canonical storage). You then run `generate` to create LIST.md. This ensures clean data flow.

### Q: What if a repository is deleted or made private?

**A**: Future implementation (Task 56) will:
- Detect 404 responses
- Mark repository as deprecated
- Preserve in archive
- Continue sync without failing

## Support

### Getting Help

- **Documentation**: Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- **Error Codes**: See [API_REFERENCE.md](API_REFERENCE.md)
- **Architecture**: Review [ARCHITECTURE.md](ARCHITECTURE.md)
- **Issues**: File bug reports with full error output

### Reporting Issues

Include:
1. Command that failed
2. Full error message
3. Config (with token redacted)
4. Output of `cargo run -- status`

---

**Last Updated**: 2025-12-11  
**Applies to**: OmniDatum v0.1.0+  
**Related**: [ARCHITECTURE.md](ARCHITECTURE.md), [API_REFERENCE.md](API_REFERENCE.md), [TROUBLESHOOTING.md](TROUBLESHOOTING.md)