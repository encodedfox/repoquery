# OmniDatum Update Runbook

This runbook documents the complete workflow for updating the OmniDatum repository documentation, including starred repositories, manual additions, web references, and books.

## Table of Contents

- [Overview](#overview)
- [Complete Update Workflow](#complete-update-workflow)
- [Adding Manual Projects](#adding-manual-projects)
- [Updating Web References](#updating-web-references)
- [Updating Books](#updating-books)
- [Cross-Reference System](#cross-reference-system)
- [Troubleshooting](#troubleshooting)
- [Maintenance Schedule](#maintenance-schedule)

## Overview

The OmniDatum repository uses a Rust-based processor to maintain synchronized documentation across multiple formats:

- **LIST.md** - Bullet list format of active repositories
- **TABLE.md** - Table format of active repositories  
- **ARCHIVE.md** - Bullet list of archived/inactive repositories
- **ARCHIVE_TABLE.md** - Table format of archived repositories
- **README.md** - Overview with curated selections and references

### Data Sources

1. **GitHub Stars** - Primary source (839 repositories)
2. **Manual Additions** - `data/canonical/manual_additions.yml`
3. **Web References** - `data/canonical/web_references.yml`
4. **Books** - `data/canonical/books.yml`

### Generated Files

All output files are generated from canonical YAML files in `data/canonical/`:
- `repositories.yml` - Parsed from LIST.md or generated from GitHub API
- `merged_data.yml` - Combined data from all sources
- Validation reports in `data/cache/validation_report.json`

## Complete Update Workflow

### Step 1: Parse Existing Data

If starting from existing LIST.md:

```bash
# Parse LIST.md into canonical format
cargo run -- parse --list LIST.md --output data/canonical/repositories.yml

# Verify parsing
cargo run -- stats --input data/canonical/repositories.yml
```

### Step 2: Add/Update Manual Entries

Edit `data/canonical/manual_additions.yml` to add projects not in GitHub stars:

```yaml
manual_projects:
  - id: "manual-project-name"
    name: "Project Name"
    description: "Project description"
    platforms:
      - platform: "github"
        url: "https://github.com/owner/repo"
        status: "active"
        is_primary: true
    metadata:
      primary_language: "Go"
      license: "MIT License"
      stars: 1000
    classification:
      categories: ["category"]
      readme_sections: ["GitHub Projects"]
    curator_notes: "Why this project is significant"
```

### Step 3: Update Web References & Books

Edit `data/canonical/web_references.yml` and `data/canonical/books.yml` as needed (see sections below).

### Step 4: Merge All Data Sources

```bash
# Merge starred repos + manual additions + web refs + books
cargo run -- merge \
  --base data/canonical/repositories.yml \
  --manual data/canonical/manual_additions.yml \
  --output data/canonical/merged_data.yml

# This automatically includes web_references.yml and books.yml from the same directory
```

### Step 5: Validate Merged Data

```bash
# Run validation checks
cargo run -- validate \
  --input data/canonical/merged_data.yml \
  --output data/cache/validation_report.json

# Review validation report
cat data/cache/validation_report.json | jq '.summary'
```

**Expected Output:**
- 0 errors (required to proceed)
- Warnings are acceptable (missing licenses, stale references)
- Info messages provide optimization suggestions

### Step 6: Generate All Documents

```bash
# Generate LIST.md, TABLE.md, ARCHIVE.md, ARCHIVE_TABLE.md
cargo run -- generate \
  --input data/canonical/merged_data.yml \
  --include-archived true
```

### Step 7: Verify Generated Files

```bash
# Check file generation
ls -lh LIST.md TABLE.md ARCHIVE.md ARCHIVE_TABLE.md

# Verify repository counts
grep -c "^- \[" LIST.md  # Should match active count from stats
grep -c "^- \[" ARCHIVE.md  # Should match archived count
```

### Step 8: Review and Commit

```bash
# Review changes
git diff LIST.md TABLE.md ARCHIVE.md ARCHIVE_TABLE.md

# Commit with descriptive message
git add data/canonical/*.yml LIST.md TABLE.md ARCHIVE.md ARCHIVE_TABLE.md
git commit -m "Update repository lists - $(date +%Y-%m-%d)"
```

## Adding Manual Projects

Manual projects are those not in your GitHub stars but worthy of inclusion.

### When to Add Manually

- Self-hosted alternatives to commercial services
- Codeberg/GitLab projects (platform diversity)
- Architecturally significant projects
- Projects referenced in Web References/Books

### Step-by-Step Process

1. **Research the Project**
   ```bash
   # Get project info
   curl https://api.github.com/repos/owner/repo | jq '{
     name: .name,
     description: .description,
     language: .language,
     license: .license.spdx_id,
     stars: .stargazers_count,
     archived: .archived
   }'
   ```

2. **Add to manual_additions.yml**
   ```yaml
   manual_projects:
     - id: "manual-project-slug"  # Unique ID
       name: "Project Name"
       description: "Clear, concise description"
       platforms:
         - platform: "github"  # or "codeberg", "gitlab"
           url: "https://github.com/owner/repo"
           status: "active"  # or "archived"
           is_primary: true
           last_verified: "2024-12-10"
       metadata:
         primary_language: "Language"
         license: "License Name"
         stars: 0  # Optional for non-GitHub
       classification:
         categories: ["category1", "category2"]
         readme_sections: ["GitHub Projects"]  # If README-worthy
       curator_notes: "Justification for inclusion"
   ```

3. **Validate Addition**
   ```bash
   # Re-run merge and validate
   cargo run -- merge --base data/canonical/repositories.yml \
     --manual data/canonical/manual_additions.yml \
     --output data/canonical/merged_data.yml
   
   cargo run -- validate --input data/canonical/merged_data.yml
   ```

4. **Verify in Generated Output**
   ```bash
   # Generate and check
   cargo run -- generate --input data/canonical/merged_data.yml
   
   # Search for your addition
   grep -n "Project Name" LIST.md
   ```

### Manual Addition Checklist

- [ ] Project has clear purpose and value
- [ ] URL is accessible and verified
- [ ] License information is accurate
- [ ] Description is concise (< 100 characters preferred)
- [ ] Classification categories are appropriate
- [ ] Curator notes explain inclusion rationale
- [ ] No duplicates exist in starred repos

## Updating Web References

Web references are curated links to documentation, articles, and learning resources.

### Structure

```yaml
references:
  - id: "unique-ref-id"
    title: "Reference Title"
    url: "https://example.com/resource"
    author: "Author Name"  # Optional
    category: "Primary Category"
    subcategory: "Subcategory"  # Optional
    content_type: "article"  # article, documentation, book, video, catalog, tutorial, paper
    difficulty: "intermediate"  # introductory, intermediate, advanced
    publication_date: "2024"  # Optional
    last_verified: "2024-12-10"
    related_references: []  # IDs of related references
    related_repos: ["owner/repo"]  # Full names of related repositories
    tags: ["tag1", "tag2"]
    status: "active"  # active, proposed, deprecated
```

### Adding New References

1. **Identify Gap or New Resource**
   - Review starred repos for emerging topics
   - Check if existing references are outdated
   - Consider topics with 20+ repos but no references

2. **Add to web_references.yml**
   ```yaml
   references:
     - id: "new-ref-2024"
       title: "Modern Approach to [Topic]"
       url: "https://example.com/article"
       category: "Architecture"  # Match existing categories
       subcategory: "Microservices"
       content_type: "article"
       difficulty: "intermediate"
       last_verified: "2024-12-10"
       related_repos: ["repo1", "repo2"]  # Link to starred repos
       tags: ["modern", "best-practices"]
       status: "active"
   ```

3. **Link to Related Repositories**
   - Find repos in `merged_data.yml` that implement concepts
   - Use `full_name` format: `owner/repo`
   - Cross-reference system will auto-generate bidirectional links

### Updating Existing References

1. **Mark Stale References**
   ```bash
   # Find references not verified in 2 years
   cargo run -- validate --input data/canonical/merged_data.yml | \
     jq '.issues[] | select(.message | contains("stale"))'
   ```

2. **Update or Deprecate**
   ```yaml
   # Update last_verified if content still relevant
   last_verified: "2024-12-10"
   
   # Or deprecate if outdated
   status: "deprecated"
   curator_notes: "Replaced by [new reference]"
   ```

### AI/ML References Modernization

For AI/ML topics, ensure coverage of:
- Foundation Models & LLMs
- Vector Databases
- RAG (Retrieval Augmented Generation)
- Model Evaluation & Benchmarking
- Training & Fine-tuning
- Agents & Orchestration
- MLOps

See `docs/aiml_modernization_proposal.md` for detailed guidance.

## Updating Books

Books provide foundational knowledge for topics represented in starred repositories.

### Structure

```yaml
books:
  - id: "book-unique-id"
    title: "Book Title"
    subtitle: "Subtitle if applicable"
    authors: ["Author One", "Author Two"]
    category: "Primary Category"
    subcategory: "Specific Topic"
    isbn: "978-0-00-000000-0"  # Optional
    publication_year: 2024
    edition: "1st"  # Optional
    related_books: []  # IDs of related books
    expansion_topics: ["topic1", "topic2"]  # Topics for future expansion
    related_web_references: []  # IDs of related web refs
    related_repos: ["owner/repo"]  # Repos that implement book concepts
```

### Adding New Books

1. **Identify Topic Gaps**
   ```bash
   # Find topics with many repos but no books
   cargo run -- stats --input data/canonical/merged_data.yml | \
     grep "By Language"
   ```

2. **Research Recommendations**
   - Check referenced repos for documentation recommendations
   - Look for books cited in Web References
   - Consider classic texts for foundational topics

3. **Add to books.yml**
   ```yaml
   books:
     - id: "new-book-2024"
       title: "Comprehensive Book Title"
       authors: ["Author Name"]
       category: "Category"
       subcategory: "Specific Focus"
       publication_year: 2024
       related_repos: ["repo1", "repo2"]  # Link implementations
       expansion_topics: ["related-topic"]
   ```

### Book Categories

Primary categories should align with repository themes:
- Architecture
- Distributed Systems
- Data Engineering
- Security
- DevOps
- Functional Programming
- Systems Programming

## Cross-Reference System

The cross-reference system creates bidirectional links between:
- Web References ↔ Repositories
- Books ↔ Repositories
- README Sections ↔ Repositories

### How It Works

1. **Graph Construction**
   - Built from `merged_data.yml`
   - Nodes: Repositories, Web References, Books, README Sections
   - Edges: References, Implements, Alternative

2. **Link Generation**
   - Web Reference lists `related_repos`
   - System finds repos and creates links
   - Generated links appear in README.md

3. **Querying Cross-References**
   ```rust
   // In code
   let graph = CrossRefGraph::build(&data)?;
   let repos = graph.repos_for_section("Web References/Architecture");
   let sections = graph.sections_for_repo("owner/repo");
   ```

### Maintaining Cross-References

1. **Add Related Repos to References**
   ```yaml
   # In web_references.yml
   related_repos:
     - "owner/repo1"  # Use full_name format
     - "owner/repo2"
   ```

2. **Validate Cross-References**
   ```bash
   # Validation checks for broken links
   cargo run -- validate --input data/canonical/merged_data.yml | \
     jq '.issues[] | select(.code == "E004")'
   ```

3. **Update README Sections**
   - Generated sections use HTML comment markers:
     ```html
     <!-- AUTO-GENERATED:START -->
     (generated content)
     <!-- AUTO-GENERATED:END -->
     ```
   - Manual content outside markers is preserved

## Troubleshooting

### Common Issues

#### Issue: Validation Fails with Duplicate Repos

**Symptom:** Error `E001: Duplicate repository URLs found`

**Solution:**
```bash
# Find duplicates
cargo run -- validate --input data/canonical/merged_data.yml | \
  jq '.issues[] | select(.code == "E001")'

# Remove from manual_additions.yml if already in starred repos
# Or update ID if it's a legitimate different project
```

#### Issue: Missing Licenses Warning

**Symptom:** Warning `E002: Repository missing license information`

**Solution:**
```bash
# Check actual repo for license
curl https://api.github.com/repos/owner/repo | jq '.license'

# Update repositories.yml or re-parse LIST.md to refresh
```

#### Issue: Invalid URL Format

**Symptom:** Error `E003: Invalid URL format detected`

**Solution:**
- Ensure all URLs start with `https://`
- Check for typos in domain names
- Verify platform URLs match pattern (github.com, codeberg.org, etc.)

#### Issue: Stale Web References

**Symptom:** Info `Stale content detected (> 2 years)`

**Solution:**
```yaml
# Update last_verified in web_references.yml
last_verified: "2024-12-10"

# Or mark as deprecated if outdated
status: "deprecated"
```

#### Issue: Template Rendering Fails

**Symptom:** `Failed to render template`

**Solution:**
```bash
# Check template syntax
cat data/templates/list.md.tera

# Ensure all required variables are in context
# Check src/generators/markdown.rs for variable names
```

#### Issue: Merge Conflicts Between Sources

**Symptom:** Manual project conflicts with starred repo

**Solution:**
- `PreferManual` strategy (default) keeps manual version
- Or remove from `manual_additions.yml` to use starred version
- Check merge output for conflict report

### Debugging Commands

```bash
# Verbose output
RUST_LOG=debug cargo run -- validate --input data/canonical/merged_data.yml

# Check statistics
cargo run -- stats --input data/canonical/merged_data.yml

# Test single repo parsing
cargo run -- parse --list LIST.md --output /tmp/test.yml
cargo run -- stats --input /tmp/test.yml
```

### Getting Help

1. Check validation report details: `data/cache/validation_report.json`
2. Review logs in terminal output
3. Consult `README_PROCESSOR.md` for processor documentation
4. Check `IMPLEMENTATION_STATUS.md` for feature status

## Maintenance Schedule

### Weekly
- [ ] Review validation warnings
- [ ] Check for new highly-starred repos in your GitHub stars
- [ ] Verify no broken URLs (automated check)

### Monthly
- [ ] Run full update workflow
- [ ] Review and update stale web references
- [ ] Check for deprecated projects
- [ ] Update star counts

### Quarterly
- [ ] Review AI/ML references for currency
- [ ] Evaluate new book recommendations
- [ ] Clean up archived projects (< 10 stars, > 2 years inactive)
- [ ] Update README with new cross-references

### Annually
- [ ] Major reorganization if needed
- [ ] Archive old technologies
- [ ] Add new trending topics
- [ ] Update all documentation

## Quick Reference

### Essential Commands

```bash
# Full update workflow
cargo run -- parse --list LIST.md --output data/canonical/repositories.yml
cargo run -- merge --base data/canonical/repositories.yml \
  --manual data/canonical/manual_additions.yml \
  --output data/canonical/merged_data.yml
cargo run -- validate --input data/canonical/merged_data.yml
cargo run -- generate --input data/canonical/merged_data.yml

# Quick stats
cargo run -- stats --input data/canonical/merged_data.yml

# Test validation
cargo run -- validate --input data/canonical/merged_data.yml | jq '.summary'
```

### File Locations

- Canonical data: `data/canonical/*.yml`
- Templates: `data/templates/*.tera`
- Cache: `data/cache/`
- Generated: `LIST.md`, `TABLE.md`, `ARCHIVE.md`, `ARCHIVE_TABLE.md`
- Documentation: `docs/`

### Data Format Reference

- Repository: `src/models/repository.rs`
- Manual Project: `src/models/manual.rs`
- Web Reference: `src/models/reference.rs`
- Book: `src/models/book.rs`

---

**Last Updated:** 2024-12-10
**Processor Version:** 0.1.0