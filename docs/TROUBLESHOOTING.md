# Troubleshooting Guide

Common issues and solutions for OmniDatum.

## Quick Diagnostics

```bash
# Check system status
cargo run -- status

# Validate data
cargo run -- validate

# Check configuration
cargo run -- configure --show

# Test with verbose logging
RUST_LOG=debug cargo run -- sync --verbose
```

## Common Issues

### Build and Installation

#### Compilation Errors

**Problem**: `error: could not compile omnidatum-processor`

**Solutions**:
```bash
# Update Rust toolchain
rustup update stable

# Clean build artifacts
cargo clean
rm Cargo.lock
cargo build

# Check Rust version (need 1.70+)
rustc --version
```

#### Missing Dependencies

**Problem**: `error: failed to resolve dependencies`

**Solutions**:
```bash
# Update dependencies
cargo update

# Clear cache
rm -rf ~/.cargo/registry/cache
cargo build

# Check for dependency conflicts
cargo tree | grep -i conflict
```

### Sync Issues

#### Authentication Failures

**Problem**: `Failed to authenticate with GitHub`

**Solutions**:
```bash
# 1. Verify token is configured
cargo run -- configure --show

# 2. Test token manually
curl -H "Authorization: token YOUR_TOKEN" https://api.github.com/user

# 3. Reconfigure credentials
cargo run -- configure --interactive

# 4. Check token permissions on GitHub
# Visit: https://github.com/settings/tokens
# Ensure: public_repo scope enabled

# 5. Try different credential source
# Edit ~/.config/omnidatum/config.toml
[credentials]
source = "Env"  # Try environment variable

export GITHUB_TOKEN="your_token"
cargo run -- sync
```

#### Rate Limit Exhausted

**Problem**: `Rate limit exhausted. Please wait 3600 seconds`

**Solutions**:
```bash
# 1. Wait for rate limit reset (shown in error)
cargo run -- status --detailed  # Shows reset time

# 2. Increase buffer to sync less per run
# Edit ~/.config/omnidatum/config.toml
[sync]
rate_limit_buffer = 1000  # Stop earlier

# 3. Use longer cache TTL
[sync]
cache_ttl_hours = 48  # Cache for 2 days

# 4. Use selective sync
cargo run -- sync --repos "critical/repos,only"
```

**GitHub Rate Limits**:
- Authenticated: 5,000 requests/hour
- Unauthenticated: 60 requests/hour
- Reset: Top of each hour

#### Network Timeouts

**Problem**: `Network timeout` or `Connection refused`

**Solutions**:
```bash
# 1. Check internet connection
ping api.github.com

# 2. Increase timeout
# Edit ~/.config/omnidatum/config.toml
[sync]
request_timeout_secs = 60  # Default: 30

# 3. Check firewall/proxy settings
curl -v https://api.github.com/

# 4. Retry sync (cache preserves partial progress)
cargo run -- sync
```

#### Cache Corruption

**Problem**: `Failed to load cache` or `Cache parse error`

**Solutions**:
```bash
# 1. Clear cache
cargo run -- sync --clear-cache

# 2. Manually delete cache
rm data/cache/sync_metadata.json

# 3. Rebuild cache
cargo run -- sync --force
```

### Validation Errors

#### E001: Duplicate Repository URLs

**Error**: `Duplicate repository URL found: https://github.com/owner/repo`

**Cause**: Same URL appears multiple times in canonical data

**Fix**:
```bash
# 1. Find duplicates
cargo run -- validate | grep E001

# 2. Edit canonical data
$EDITOR data/canonical/repositories.yml

# 3. Remove or merge duplicates

# 4. Validate again
cargo run -- validate
```

#### E002: Missing License Information

**Error**: `Repository missing license information`

**Cause**: Repository lacks license field

**Fix**:
```bash
# 1. Research license on GitHub
# Visit repo and check LICENSE file

# 2. Update canonical data
# Edit data/canonical/repositories.yml
license: "MIT"
license_spdx: "MIT"

# 3. Or re-sync from GitHub
cargo run -- sync --repos "owner/repo" --force
```

#### E003: Invalid URL Format

**Error**: `Invalid URL format: not-a-url`

**Cause**: Malformed URL in repository data

**Fix**:
```bash
# 1. Identify invalid URL
cargo run -- validate | grep E003

# 2. Correct URL format
# Must be: https://platform.com/owner/repo

# 3. Edit canonical data
$EDITOR data/canonical/repositories.yml
```

#### E004: Broken Cross-Reference

**Error**: `Web reference links to non-existent repository`

**Cause**: Web reference or book links to repository not in dataset

**Fix**:
```bash
# 1. Identify broken link
cargo run -- validate | grep E004

# 2. Option A: Add missing repository
# Star on GitHub, then sync
cargo run -- sync --repos "missing/repo"

# 2. Option B: Remove broken link
$EDITOR data/canonical/web_references.yml
# Remove from related_repos
```

#### E005: Platform Migration Incomplete

**Error**: `Repository mentions migration but lacks secondary platform URL`

**Cause**: Description mentions migration but no secondary platform configured

**Fix**:
```bash
# 1. Research migration
# Check repo for Codeberg/GitLab URL

# 2. Add secondary platform
# Edit data/canonical/repositories.yml
platforms:
  - platform: "GitHub"
    status: "Migrated"
  - platform: "Codeberg"
    url: "https://codeberg.org/owner/repo"
    status: "Active"
    is_primary: true
```

#### E006: Missing Metadata

**Error**: `Repository missing description or owner`

**Cause**: Required metadata fields empty

**Fix**:
```bash
# Re-sync from GitHub
cargo run -- sync --repos "owner/repo" --force

# Or manually add
# Edit data/canonical/repositories.yml
description: "Project description"
owner: "owner-name"
```

#### E007: External Data Inconsistent

**Error**: `External data inconsistency detected`

**Causes**:
- Stars > 1,000,000 (unrealistic)
- Archive status mismatch
- Missing description for popular repo

**Fix**:
```bash
# 1. View details
cargo run -- validate --check-external-consistency

# 2. Re-sync repository
cargo run -- sync --repos "owner/repo" --force

# 3. Manual correction if needed
$EDITOR data/canonical/repositories.yml
```

#### E008: Duplicate Repository Name

**Error**: `Duplicate repository name detected: owner/repo`

**Cause**: Same full_name exists multiple times (different platforms)

**Fix**:
```bash
# 1. Find duplicates
cargo run -- validate | grep E008

# 2. Review in data file
grep -n "full_name: owner/repo" data/canonical/repositories.yml

# 3. Merge or differentiate
# Either merge entries or specify different platforms
```

### Generation Errors

#### Template Not Found

**Problem**: `Template 'list.md.tera' not found`

**Solutions**:
```bash
# 1. Check template directory
ls -la data/templates/

# 2. Verify templates exist
ls data/templates/*.tera

# 3. Specify template directory
cargo run -- generate --template-dir /path/to/templates

# 4. Restore from Git
git checkout data/templates/
```

#### Template Syntax Error

**Problem**: `Template parse error at line X`

**Solutions**:
```bash
# 1. Validate template syntax
# Tera uses Jinja2-style syntax

# 2. Common issues:
# - Unclosed tags: {% if %}...{% endif %}
# - Wrong variable: {{ wrong }} → {{ correct }}
# - Missing filters: {{ value | default(value="N/A") }}

# 3. Test template
cargo run -- generate --verbose
```

#### Output File Permissions

**Problem**: `Permission denied writing to LIST.md`

**Solutions**:
```bash
# 1. Check file permissions
ls -la LIST.md

# 2. Make writable
chmod u+w LIST.md

# 3. Check directory permissions
ls -ld .

# 4. Run as appropriate user
sudo chown $USER:$USER .
```

### Configuration Issues

#### Config File Not Found

**Problem**: `Failed to load configuration`

**Solutions**:
```bash
# 1. Create default config
cargo run -- configure --interactive

# 2. Check config location
cargo run -- configure --show

# 3. Manually create config directory
mkdir -p ~/.config/omnidatum

# 4. Use environment variables instead
export GITHUB_TOKEN="your_token"
cargo run -- sync
```

#### Invalid Configuration

**Problem**: `Configuration validation failed`

**Solutions**:
```bash
# Check which values are invalid
# Valid ranges:
# - parallel_workers: 1-10
# - cache_ttl_hours: 1-168
# - rate_limit_buffer: 0-1000

# Edit config
$EDITOR ~/.config/omnidatum/config.toml

# Verify
cargo run -- configure --show
```

#### Credential File Permissions

**Problem**: `Insecure credential file permissions`

**Solutions**:
```bash
# Fix permissions (Unix)
chmod 600 ~/.config/omnidatum/credentials

# Verify
ls -la ~/.config/omnidatum/credentials
# Should show: -rw------- (600)

# On Windows, use file properties to restrict access
```

### Data Issues

#### YAML Parse Errors

**Problem**: `YAML parse error at line X`

**Solutions**:
```bash
# 1. Validate YAML syntax
# Use online YAML validator or:
python3 -c "import yaml; yaml.safe_load(open('data/canonical/repositories.yml'))"

# 2. Common issues:
# - Incorrect indentation (use 2 spaces)
# - Missing quotes for special characters
# - Invalid date format (use YYYY-MM-DD)

# 3. Restore from backup
cp data/canonical/repositories.yml.backup data/canonical/repositories.yml
```

#### Corrupted Data File

**Problem**: Repository data appears corrupted

**Solutions**:
```bash
# 1. Check Git history
git log --oneline data/canonical/repositories.yml

# 2. Restore from Git
git checkout HEAD~1 data/canonical/repositories.yml

# 3. Re-sync from GitHub
cargo run -- sync --force

# 4. Validate after restoration
cargo run -- validate
```

## Error Code Reference

### Complete Error Codes (E001-E008)

| Code | Name | Severity | Fix |
|------|------|----------|-----|
| **E001** | Duplicate Repos | Error | Remove duplicate URLs |
| **E002** | Missing License | Warning | Add license or re-sync |
| **E003** | Invalid URL | Error | Correct URL format |
| **E004** | Broken Cross-Ref | Warning | Add missing repo or remove link |
| **E005** | Platform Migration | Warning | Add secondary platform URL |
| **E006** | Missing Metadata | Info | Add description/owner |
| **E007** | External Data Inconsistent | Error/Warning | Re-sync repository |
| **E008** | Duplicate Repo Name | Error | Merge or differentiate entries |

### Error Resolution Matrix

| Error | Auto-Fixable | Manual Required | Re-sync Fixes |
|-------|--------------|-----------------|---------------|
| E001 | No | Yes | No |
| E002 | No | Yes | Yes |
| E003 | No | Yes | No |
| E004 | No | Yes | Maybe |
| E005 | No | Yes | No |
| E006 | No | Yes | Yes |
| E007 | Maybe | Maybe | Yes |
| E008 | No | Yes | No |

## Performance Issues

### Slow Sync

**Problem**: Sync takes too long

**Solutions**:
```bash
# 1. Check cache usage
cargo run -- status
# Should show high cache hit rate

# 2. Use selective sync
cargo run -- sync --repos "frequently-updated-repos"

# 3. Increase cache TTL
[sync]
cache_ttl_hours = 48

# 4. Check network speed
curl -w "@curl-format.txt" -o /dev/null -s https://api.github.com/
```

### High Memory Usage

**Problem**: Process uses too much memory

**Solutions**:
```bash
# 1. Use release build
cargo build --release

# 2. Process in batches (future feature)
cargo run -- sync --batch-size 100

# 3. Monitor memory
/usr/bin/time -v cargo run --release -- generate
```

### Slow Generation

**Problem**: Document generation is slow

**Solutions**:
```bash
# 1. Use release build
cargo build --release

# 2. Profile to find bottleneck
cargo flamegraph -- generate

# 3. Simplify templates
# Remove complex Tera logic from templates

# 4. Disable statistics
cargo run -- generate --no-stats
```

## Debug Mode

### Enable Detailed Logging

```bash
# Set log level
export RUST_LOG=omnidatum_processor=debug

# Module-specific logging
export RUST_LOG=omnidatum_processor::sync=trace

# Log to file
cargo run -- sync 2>&1 | tee sync.log

# With timestamps
RUST_LOG=omnidatum_processor=debug cargo run -- sync 2>&1 | ts
```

### Debugging Specific Components

```bash
# Sync operations
RUST_LOG=omnidatum_processor::sync=debug cargo run -- sync --verbose

# Validation
RUST_LOG=omnidatum_processor::validators=debug cargo run -- validate

# Template generation
RUST_LOG=omnidatum_processor::generators=debug cargo run -- generate
```

## Data Recovery

### Restore from Backup

```bash
# Git history
git log --oneline data/canonical/

# Restore from commit
git checkout <commit-hash> data/canonical/repositories.yml

# Create backup before changes
cp data/canonical/repositories.yml{,.backup}
```

### Rebuild from Scratch

```bash
# 1. Backup current data
cp -r data/canonical data/canonical.backup

# 2. Parse from original LIST.md (if available)
cargo run -- parse

# 3. Or re-sync everything
cargo run -- sync --force --clear-cache

# 4. Merge manual additions
cargo run -- merge

# 5. Validate
cargo run -- validate

# 6. Generate
cargo run -- generate
```

## Getting Help

### Before Asking for Help

Gather this information:
1. **Command** that failed
2. **Full error message**
3. **System info**: `uname -a` and `cargo --version`
4. **Config**: `cargo run -- configure --show` (redact token)
5. **Status**: `cargo run -- status`
6. **Logs**: `RUST_LOG=debug cargo run -- <command> 2>&1 | tee debug.log`

### Where to Get Help

- **Documentation**: Check [README.md](../README.md), [DATA_SYNC.md](DATA_SYNC.md), [API_REFERENCE.md](API_REFERENCE.md)
- **Issues**: Search existing issues on GitHub
- **Discussions**: Ask in GitHub Discussions
- **Stack Overflow**: Tag with `omnidatum` and `rust`

### Filing Bug Reports

Include:
```markdown
## Environment
- OS: [e.g., macOS 14.0, Ubuntu 22.04]
- Rust version: [output of `rustc --version`]
- OmniDatum version: [output of `cargo run -- --version`]

## Command
```bash
cargo run -- sync --repos "owner/repo"
```

## Error
```
[paste full error output]
```

## Configuration
```toml
[paste config with tokens redacted]
```

## Steps to Reproduce
1. Run command X
2. Observe error Y
3. Expected result Z
```

## FAQ

### Q: Why do some tests fail?

**A**: Two tests (`test_load_from_env`, `test_load_from_env_missing`) fail when run in parallel due to environment variable conflicts. Run with `--test-threads=1` to avoid this:
```bash
cargo test -- --test-threads=1
```

### Q: How do I reset everything?

**A**: Complete reset:
```bash
# 1. Clear cache
rm -rf data/cache/

# 2. Reset config
rm -rf ~/.config/omnidatum/

# 3. Restore data from Git
git checkout data/canonical/

# 4. Reconfigure
cargo run -- configure

# 5. Sync
cargo run -- sync
```

### Q: Can I run sync without GitHub token?

**A**: No, GitHub API requires authentication. Without a token:
- Rate limit: 60 requests/hour (vs 5,000 authenticated)
- No access to some repository metadata
- Risk of hitting rate limit quickly

### Q: What if a repository is deleted?

**A**: Currently returns error. Future implementation (Task 56) will:
- Detect 404 responses
- Mark repository as deprecated
- Preserve in archive
- Continue sync

### Q: How do I contribute a fix?

**A**: See [DEVELOPMENT.md](DEVELOPMENT.md) and [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

## Platform-Specific Issues

### macOS

**Keychain Access**:
```bash
# Grant terminal access to keychain
security unlock-keychain

# Store token
security add-generic-password -a omnidatum -s github_token -w "token"

# Retrieve token
security find-generic-password -a omnidatum -s github_token -w
```

### Linux

**Secret Service**:
```bash
# Install secret-tool
sudo apt install libsecret-tools  # Debian/Ubuntu
sudo dnf install libsecret        # Fedora

# Store token
secret-tool store --label="OmniDatum GitHub Token" service omnidatum username github

# Retrieve token
secret-tool lookup service omnidatum username github
```

### Windows

**Credential Manager**:
```powershell
# Store in Windows Credential Manager
cmdkey /generic:omnidatum_github /user:token /pass:your_token_here

# Or use file-based storage
cargo run -- configure --interactive
```

## Advanced Troubleshooting

### Memory Leaks

```bash
# Profile with valgrind (Linux)
valgrind --leak-check=full cargo run -- generate

# Profile with Instruments (macOS)
instruments -t Leaks cargo run -- generate
```

### Performance Profiling

```bash
# CPU profiling
cargo flamegraph -- generate

# Memory profiling
cargo build --release
heaptrack ./target/release/omnidatum-processor generate
```

### Network Debugging

```bash
# Trace API calls
RUST_LOG=octocrab=trace cargo run -- sync --verbose

# Monitor with tcpdump
sudo tcpdump -i any host api.github.com

# Monitor with Wireshark
# Filter: ip.host == api.github.com
```

## Still Need Help?

If you've tried the solutions above and still have issues:

1. **Search existing issues**: https://github.com/owner/omnidatum/issues
2. **Open new issue**: Include all diagnostic information
3. **Ask in discussions**: For general questions
4. **Check documentation**: May have been recently updated

---

**Last Updated**: 2025-12-11  
**Covers**: OmniDatum v0.1.0+  
**Related**: [DATA_SYNC.md](DATA_SYNC.md), [DEVELOPMENT.md](DEVELOPMENT.md), [API_REFERENCE.md](API_REFERENCE.md)