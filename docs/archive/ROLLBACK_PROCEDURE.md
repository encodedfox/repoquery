# Rollback Procedure

This document describes how to rollback to previous versions of the OmniDatum documentation if needed.

## Version Control Setup

### Initialize Git Repository (If Not Already Done)

```bash
# Initialize repository
git init

# Add .gitignore
cat > .gitignore << 'EOF'
# Rust build artifacts
target/
Cargo.lock

# IDE
.vscode/
.idea/

# Cache files
data/cache/
*.log

# OS files
.DS_Store
EOF

# Initial commit
git add .
git commit -m "Initial commit - v1.0.0-baseline"

# Tag baseline
git tag -a v1.0.0-baseline -m "OmniDatum baseline with Rust processor"
```

## Baseline Backup

### Current Baseline Files (v1.0.0-baseline)

**Date**: 2024-12-10  
**Commit**: (to be tagged)

**Critical Files**:
- `LIST.md` (141KB, 797 active repos)
- `TABLE.md` (146KB, 797 active repos)
- `ARCHIVE.md` (11KB, 48 archived repos)
- `ARCHIVE_TABLE.md` (12KB, 48 archived repos)
- `README.md` (6KB)
- `data/canonical/*.yml` (all YAML data files)

### Creating Manual Backup

If git is not available, create manual backup:

```bash
# Create backup directory
mkdir -p backups/v1.0.0-baseline

# Copy critical files
cp LIST.md backups/v1.0.0-baseline/
cp TABLE.md backups/v1.0.0-baseline/
cp ARCHIVE.md backups/v1.0.0-baseline/
cp ARCHIVE_TABLE.md backups/v1.0.0-baseline/
cp README.md backups/v1.0.0-baseline/
cp -r data/canonical backups/v1.0.0-baseline/

# Create backup manifest
cat > backups/v1.0.0-baseline/MANIFEST.md << 'EOF'
# Backup Manifest - v1.0.0-baseline

**Created**: 2024-12-10
**Files**:
- LIST.md (797 active repos)
- TABLE.md (797 active repos)  
- ARCHIVE.md (48 archived repos)
- ARCHIVE_TABLE.md (48 archived repos)
- README.md (original with manual content)
- data/canonical/*.yml (all data files)

**Restore Command**:
```bash
cp -r backups/v1.0.0-baseline/* .
```
EOF

# Compress backup (optional)
tar -czf backups/v1.0.0-baseline.tar.gz backups/v1.0.0-baseline/
```

## Rollback Scenarios

### Scenario 1: Rollback Generated Files Only

**When**: Generated files (LIST.md, TABLE.md) have issues but data is fine

```bash
# Using git
git checkout v1.0.0-baseline -- LIST.md TABLE.md ARCHIVE.md ARCHIVE_TABLE.md

# Using backup
cp backups/v1.0.0-baseline/LIST.md .
cp backups/v1.0.0-baseline/TABLE.md .
cp backups/v1.0.0-baseline/ARCHIVE.md .
cp backups/v1.0.0-baseline/ARCHIVE_TABLE.md .

# Regenerate from data if needed
cargo run -- generate --input data/canonical/merged_data.yml
```

### Scenario 2: Rollback Data Files

**When**: Canonical data files corrupted or merged incorrectly

```bash
# Using git
git checkout v1.0.0-baseline -- data/canonical/

# Using backup
cp -r backups/v1.0.0-baseline/canonical/ data/

# Re-merge and regenerate
cargo run -- merge --base data/canonical/repositories.yml \
  --manual data/canonical/manual_additions.yml \
  --output data/canonical/merged_data.yml

cargo run -- generate --input data/canonical/merged_data.yml
```

### Scenario 3: Complete Rollback

**When**: Major issues, need to restore entire baseline

```bash
# Using git
git checkout v1.0.0-baseline

# Using backup
cp -r backups/v1.0.0-baseline/* .

# Verify restoration
ls -lh LIST.md TABLE.md ARCHIVE.md ARCHIVE_TABLE.md
cargo run -- stats --input data/canonical/merged_data.yml
```

### Scenario 4: Rollback Single File

**When**: Specific file has issues

```bash
# Using git
git log --oneline -- LIST.md
git checkout <commit-hash> -- LIST.md

# Using backup
cp backups/v1.0.0-baseline/LIST.md .
```

## Verification After Rollback

### Check File Integrity

```bash
# Verify file sizes
ls -lh LIST.md TABLE.md

# Count repositories
grep -c "^- \[" LIST.md  # Should be 797
grep -c "^- \[" ARCHIVE.md  # Should be 48

# Verify format
head -50 LIST.md
tail -20 LIST.md  # Check statistics footer
```

### Validate Data

```bash
# Run validation
cargo run -- validate --input data/canonical/merged_data.yml

# Check statistics
cargo run -- stats --input data/canonical/merged_data.yml

# Should show:
# - Total: 845 repos
# - Active: 797
# - Archived: 48
# - No errors
```

### Verify Generation Pipeline

```bash
# Test regeneration
cargo run -- merge --base data/canonical/repositories.yml \
  --manual data/canonical/manual_additions.yml \
  --output /tmp/test_merge.yml

cargo run -- generate --input /tmp/test_merge.yml

# Compare with baseline
diff LIST.md /tmp/LIST.md || echo "Differences found"
```

## Backup Schedule

### Recommended Backup Frequency

**Before Major Changes**:
- Before merging new data sources
- Before updating 50+ repositories
- Before restructuring web references
- Before CLI changes

**Regular Schedule**:
- Weekly: Backup generated files
- Monthly: Backup entire data/ directory
- Quarterly: Create compressed archive

### Automated Backup Script

```bash
#!/bin/bash
# backup.sh - Create timestamped backup

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_DIR="backups/$TIMESTAMP"

mkdir -p "$BACKUP_DIR"

echo "Creating backup: $BACKUP_DIR"

# Copy files
cp LIST.md "$BACKUP_DIR/"
cp TABLE.md "$BACKUP_DIR/"
cp ARCHIVE.md "$BACKUP_DIR/"
cp ARCHIVE_TABLE.md "$BACKUP_DIR/"
cp README.md "$BACKUP_DIR/"
cp -r data/canonical "$BACKUP_DIR/"

# Create manifest
cat > "$BACKUP_DIR/MANIFEST.md" << EOF
# Backup Manifest

**Created**: $(date)
**Files**: See directory listing

\`\`\`bash
# Restore this backup:
cp -r $BACKUP_DIR/* .
\`\`\`
EOF

echo "Backup complete: $BACKUP_DIR"

# Compress
tar -czf "$BACKUP_DIR.tar.gz" "$BACKUP_DIR"
echo "Compressed: $BACKUP_DIR.tar.gz"
```

## Version History

### v1.0.0-baseline (2024-12-10)

**State**:
- 845 total repositories (797 active, 48 archived)
- 7 books
- 57 web references
- 6 manual projects
- Rust processor v0.1.0
- 38 passing tests
- 0 validation errors

**Files**:
- Canonical data in YAML format
- Generated LIST/TABLE/ARCHIVE files
- Complete documentation suite

### Pre-v1.0.0 (Before 2024-12-10)

**State**:
- Original stargazer (bash-based)
- Manual maintenance
- No structured data
- No validation system

## Recovery Scenarios

### Lost Data Files

**Symptom**: `data/canonical/*.yml` files deleted or corrupted

**Recovery**:
```bash
# Option 1: Restore from backup
cp -r backups/v1.0.0-baseline/canonical/ data/

# Option 2: Re-parse from markdown (if LIST.md still valid)
cargo run -- parse --list LIST.md --output data/canonical/repositories.yml

# Verify
cargo run -- stats --input data/canonical/repositories.yml
```

### Lost Generated Files

**Symptom**: LIST.md, TABLE.md deleted

**Recovery**:
```bash
# Regenerate from canonical data
cargo run -- generate --input data/canonical/merged_data.yml

# Verify
diff LIST.md backups/v1.0.0-baseline/LIST.md
```

### Corrupted Templates

**Symptom**: Generation fails with template errors

**Recovery**:
```bash
# Restore templates from backup
cp -r backups/v1.0.0-baseline/templates/ data/

# Or restore from git
git checkout v1.0.0-baseline -- data/templates/

# Test generation
cargo run -- generate --input data/canonical/merged_data.yml
```

## Disaster Recovery

### Complete Data Loss

If all local files are lost but you have:

1. **GitHub Stars** (intact):
   - Use stargazer to fetch fresh data
   - Re-parse and rebuild canonical data
   - Manual additions will need recreation

2. **Remote Git Repository**:
   ```bash
   git clone <remote-url>
   cd omnidatum
   cargo build
   ```

3. **Backup Archive**:
   ```bash
   tar -xzf backups/v1.0.0-baseline.tar.gz
   cp -r backups/v1.0.0-baseline/* .
   ```

## Best Practices

### Before Making Changes

1. ✅ Create backup: `./backup.sh`
2. ✅ Commit current state: `git commit -am "Pre-change snapshot"`
3. ✅ Tag if milestone: `git tag v1.0.1`
4. ✅ Document intent: Update CHANGELOG.md

### After Making Changes

1. ✅ Validate: `cargo run -- validate`
2. ✅ Test: `cargo test`
3. ✅ Verify output: Check generated files
4. ✅ Commit: `git commit -am "Descriptive message"`
5. ✅ Keep backup for 30 days

### Backup Retention

- **Daily backups**: Keep 7 days
- **Weekly backups**: Keep 4 weeks
- **Monthly backups**: Keep 12 months
- **Baseline versions**: Keep indefinitely

## Troubleshooting

### Cannot Restore from Backup

**Issue**: Backup directory not found

**Solution**:
1. Check backups/ directory exists
2. Look for .tar.gz archives
3. Check git tags: `git tag -l`
4. Check git history: `git log --oneline`

### Restored Files Don't Work

**Issue**: Restored files fail validation

**Solution**:
1. Verify backup integrity: Check file sizes
2. Re-run merge: May need to rebuild merged_data.yml
3. Check schema version compatibility
4. Regenerate from canonical data

### Git Not Initialized

**Issue**: No version control available

**Solution**:
1. Initialize now: `git init`
2. Add all files: `git add .`
3. Commit baseline: `git commit -m "Baseline commit"`
4. Tag: `git tag v1.0.0-baseline`
5. Create manual backups going forward

## Contacts

For rollback assistance:
- Check [UPDATE_RUNBOOK.md](UPDATE_RUNBOOK.md) for normal procedures
- See [ARCHITECTURE.md](ARCHITECTURE.md) for system understanding
- Review [CHANGELOG.md](CHANGELOG.md) for version history

---

**Last Updated**: 2024-12-10  
**Processor Version**: 0.1.0