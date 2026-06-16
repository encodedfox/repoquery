# Development Guide

Complete guide for developers contributing to OmniDatum.

## Development Environment Setup

### Prerequisites

- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **Git**: For version control
- **GitHub Token**: For sync development/testing (see [DATA_SYNC.md](DATA_SYNC.md))

### Initial Setup

```bash
# Clone repository
git clone <repository-url>
cd omnidatum

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build project
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- validate
```

### IDE Setup

**VS Code** (Recommended):
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.check.command": "clippy",
  "[rust]": {
    "editor.formatOnSave": true
  }
}
```

**IntelliJ IDEA**:
- Install Rust plugin
- Enable format on save
- Configure Clippy for warnings

## Building and Running

### Development Build

```bash
# Fast compilation, includes debug symbols
cargo build

# Run directly
cargo run -- --help

# Run specific command
cargo run -- sync --dry-run
```

### Release Build

```bash
# Optimized binary
cargo build --release

# Run release binary
./target/release/omnidatum-processor --help
```

### Build Profiles

| Profile | Optimization | Debug Info | Use Case |
|---------|--------------|------------|----------|
| dev | 0 | Full | Development |
| release | 3 | None | Production |
| test | 0 | Full | Testing |

## Testing

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_sync_workflow

# With output
cargo test -- --nocapture

# With logging
RUST_LOG=debug cargo test

# Serial execution (for env tests)
cargo test -- --test-threads=1
```

### Test Structure

```
tests/
├── integration_test.rs    # E2E tests
├── mock_adapter.rs        # Mock GitHub adapter
└── test_data/            # Test fixtures

src/
├── config/
│   ├── settings.rs       # Config tests
│   └── credentials.rs    # Credential tests
├── sync/
│   ├── cache.rs          # Cache tests
│   └── progress.rs       # Progress tests
└── validators/
    └── external_data_rules.rs  # Validation tests
```

### Test Categories

**Unit Tests**: Test individual functions/methods
```rust
#[test]
fn test_parse_github_repo() {
    let repo = create_test_repo("rust-lang/rust");
    let result = parse_github_repo(&repo);
    assert!(result.is_ok());
}
```

**Integration Tests**: Test complete workflows
```rust
#[tokio::test]
async fn test_sync_workflow() {
    // Setup, execute, verify
}
```

**Mock Tests**: Test with simulated external services
```rust
let mut mock = MockGitHubAdapter::new();
mock.add_response("owner/repo", test_repo);
let result = mock.fetch_repository("owner/repo").await;
```

### Writing Tests

```rust
// Good test structure
#[test]
fn test_feature_name() {
    // Arrange
    let input = create_test_data();
    
    // Act
    let result = function_under_test(input);
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_value);
}
```

## Code Style

### Rust Conventions

```rust
// Use descriptive names
pub struct SyncOrchestrator {  // Good
pub struct SO {                 // Bad

// Document public APIs
/// Synchronize repository metadata from GitHub API
pub async fn sync_all(&mut self) -> Result<SyncResult> {

// Use Result for fallible operations
pub fn load_config() -> Result<Config> {  // Good
pub fn load_config() -> Config {          // Bad (panics on error)

// Prefer iterators over loops
repos.iter()
    .filter(|r| r.is_active())
    .map(|r| r.metadata.stars)
    .sum()
```

### Formatting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Format on save (recommended)
# Configure in your IDE
```

### Linting

```bash
# Run Clippy
cargo clippy

# Fix automatically
cargo clippy --fix

# Strict mode
cargo clippy -- -D warnings
```

## Project Structure

### Module Organization

```
src/
├── config/         # Configuration management
├── sync/           # External data sync
├── models/         # Data structures
├── validators/     # Quality rules
├── parsers/        # Data extraction
├── generators/     # Output creation
├── cross_refs/     # Relationship tracking
├── merge.rs        # Data integration
└── readme_updater.rs
```

### Adding New Modules

1. Create module file: `src/new_module.rs`
2. Add to `src/lib.rs`: `pub mod new_module;`
3. Export public types: `pub use new_module::PublicType;`
4. Add tests: `#[cfg(test)] mod tests { ... }`
5. Document module: `//! Module documentation`

## Contributing

### Contribution Workflow

1. **Fork and Clone**
```bash
git clone <your-fork-url>
cd omnidatum
git remote add upstream <original-repo-url>
```

2. **Create Feature Branch**
```bash
git checkout -b feature/my-feature
```

3. **Make Changes**
```bash
# Edit code
# Add tests
# Update documentation
```

4. **Verify Quality**
```bash
cargo test
cargo fmt
cargo clippy
```

5. **Commit**
```bash
git add .
git commit -m "feat: add my feature"
```

6. **Push and PR**
```bash
git push origin feature/my-feature
# Open pull request on GitHub
```

### Commit Message Convention

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `test`: Adding tests
- `refactor`: Code restructuring
- `perf`: Performance improvement
- `chore`: Maintenance tasks

**Examples**:
```
feat(sync): add selective repository sync
fix(validation): correct E007 star count threshold
docs(api): add sync command examples
test(integration): add rate limit handling test
```

## Code Review Guidelines

### For Reviewers

✅ **Check**:
- Code follows Rust conventions
- Tests cover new functionality
- Documentation updated
- No breaking changes (or documented)
- Performance acceptable
- Error handling robust

### For Contributors

📋 **Before Submitting PR**:
- [ ] Tests pass (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No Clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Commit messages follow convention

## Debugging

### Enable Logging

```bash
# Debug level
export RUST_LOG=omnidatum_processor=debug
cargo run -- sync

# Trace level (very verbose)
export RUST_LOG=omnidatum_processor=trace
cargo run -- validate

# Module-specific
export RUST_LOG=omnidatum_processor::sync=debug
```

### Debugging Tools

```bash
# Backtrace on panic
export RUST_BACKTRACE=1
cargo run -- generate

# Full backtrace
export RUST_BACKTRACE=full

# GDB debugging
rust-gdb target/debug/omnidatum-processor
```

### Common Debug Tasks

```bash
# Check data structure
cat data/canonical/repositories.yml | head -50

# Validate JSON output
cat data/cache/validation_report.json | jq '.'

# Test specific rule
cargo test test_external_data_consistency -- --nocapture

# Profile performance
cargo build --release
time ./target/release/omnidatum-processor generate
```

## Curation Guidelines

For curators maintaining the repository content:

### When to Add Manual Projects

Add when repository meets criteria:
- ⭐️ >2000 stars + architectural significance
- 🏗️ Implements important pattern
- 🌐 Platform diversity (Codeberg, GitLab)
- 📚 Referenced in books or web references
- 🎯 Strategic knowledge management value

### When to Add Web References

Add when content is:
- 📖 Foundational concept with 5+ repo examples
- 🆕 Emerging technology with 20+ repos
- 🔗 Frequently cited in starred repos
- 📊 Gap identified via analysis
- ✅ Authority/quality of source

### When to Add Books

Add when book covers:
- 📚 Topic has 20+ starred repos
- 🎓 Foundational knowledge required
- 🔍 Referenced in web refs or repos
- ⭐ Classic or authoritative text
- 🆕 Modern coverage of emerging topics

### When to Archive Repos

Archive when repository is:
- 📦 <10 stars AND >2 years inactive
- ⛔ Explicitly archived by maintainer
- 🔄 Migrated with no activity on original
- ⚠️ Deprecated technology

## Community Feedback Process

### Feedback Channels

1. **GitHub Issues**: Bug reports, feature requests, documentation improvements
2. **GitHub Discussions**: General questions, ideas, community engagement
3. **Pull Requests**: Direct contributions

### Response Timeline

| Priority | Response Time | Resolution Time |
|----------|--------------|-----------------|
| P0 - Critical | 24 hours | 1 week |
| P1 - High | 3 days | 2 weeks |
| P2 - Medium | 1 week | 1 month |
| P3 - Low | 2 weeks | Next quarter |

### Evaluation Criteria

For each feedback item:
1. **Impact**: How many users affected?
2. **Effort**: Implementation complexity?
3. **Alignment**: Fits project goals?
4. **Value**: Cost-benefit ratio?

### Recognition

Contributors are recognized through:
- CHANGELOG.md credits
- GitHub contributor stats
- Release notes mentions
- Special thanks in major releases

## Release Process

### Version Planning

**Patch Releases (X.Y.Z)**:
- Bug fixes
- Documentation updates
- No feature additions

**Minor Releases (X.Y.0)**:
- New features
- Enhancements
- Quarterly releases

**Major Releases (X.0.0)**:
- Breaking changes
- Major architecture updates
- Annual or less

### Release Checklist

- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version bumped in Cargo.toml
- [ ] Git tag created
- [ ] GitHub release published

## Performance Optimization

### Profiling

```bash
# Build with profiling symbols
cargo build --release --profile=profiling

# Profile with perf (Linux)
perf record ./target/release/omnidatum-processor generate
perf report

# Profile with Instruments (macOS)
instruments -t Time\ Profiler ./target/release/omnidatum-processor generate
```

### Benchmarking

```bash
# Time operations
time cargo run --release -- sync

# Memory profiling
/usr/bin/time -v cargo run --release -- generate

# Compare before/after
hyperfine "cargo run --release -- generate" \
          "cargo run --release -- generate --new-feature"
```

## Troubleshooting Development Issues

### Build Errors

```bash
# Update dependencies
cargo update

# Clean build
cargo clean
cargo build

# Check for dependency conflicts
cargo tree
```

### Test Failures

```bash
# Run single test with output
cargo test test_name -- --nocapture --test-threads=1

# Update test snapshots (if using insta)
cargo insta review

# Check for race conditions
cargo test -- --test-threads=1
```

### IDE Issues

```bash
# Rebuild rust-analyzer index
rm -rf target/debug/.fingerprint
cargo clean
cargo build
```

## Documentation Standards

### Code Documentation

```rust
//! Module-level documentation
//!
//! Explains the module's purpose and main concepts.

/// Function documentation
///
/// # Arguments
/// * `param` - Description
///
/// # Returns
/// Description of return value
///
/// # Errors
/// Conditions that cause errors
///
/// # Examples
/// ```
/// let result = function(param);
/// ```
pub fn function(param: Type) -> Result<Output> {
    // Implementation
}
```

### User Documentation

- Use clear, concise language
- Include code examples
- Add screenshots for UI features
- Link to related documentation
- Test all commands/examples

## Resources

### Learning Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)

### Project Resources
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture
- [API_REFERENCE.md](API_REFERENCE.md) - CLI and API reference
- [DATA_SYNC.md](DATA_SYNC.md) - Sync setup and usage
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues

### Related Projects
- [octocrab](https://docs.rs/octocrab/) - GitHub API client
- [Tera](https://keats.github.io/tera/) - Template engine
- [petgraph](https://docs.rs/petgraph/) - Graph data structure

## Getting Help

- **Questions**: Open a GitHub Discussion
- **Bugs**: File an issue with reproduction steps
- **Features**: Propose in GitHub Issues
- **Chat**: Check for Discord/Slack links in README

---

**Last Updated**: 2025-12-11  
**Version**: 0.1.0  
**For**: Contributors and maintainers