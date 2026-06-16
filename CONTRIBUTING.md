# Contributing to OmniDatum Processor

Thank you for your interest in contributing to the OmniDatum Processor! This document provides guidelines and information for contributors.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Adding New Features](#adding-new-features)
- [Documentation](#documentation)

## Getting Started

### Prerequisites

- Rust 1.70 or later (2021 edition)
- Cargo (comes with Rust)
- Git
- Text editor with Rust support (VS Code + rust-analyzer recommended)

### Clone and Build

```bash
git clone <repository-url>
cd omnidatum
cargo build
cargo test
```

### Verify Installation

```bash
cargo run -- stats --input data/canonical/merged_data.yml
```

## Development Setup

### Recommended Tools

```bash
# Install rust-analyzer for IDE support
rustup component add rust-analyzer

# Install clippy for linting
rustup component add clippy

# Install rustfmt for formatting
rustup component add rustfmt
```

### Running in Development Mode

```bash
# Fast development builds
cargo build

# Run with logging
RUST_LOG=debug cargo run -- validate --input data/canonical/merged_data.yml

# Watch mode (requires cargo-watch)
cargo install cargo-watch
cargo watch -x test
```

## Code Style

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt
```

### Linting

All code must pass `clippy` without warnings:

```bash
cargo clippy -- -D warnings
```

### Naming Conventions

**Files and Modules**:
- Use `snake_case` for file names: `list_parser.rs`, `cross_refs/mod.rs`
- Module names follow file names

**Types**:
- `PascalCase` for structs, enums, traits: `Repository`, `ValidationRule`
- Enum variants: `PascalCase`

**Functions and Variables**:
- `snake_case` for functions and variables: `calculate_statistics()`, `repo_count`
- Methods start with verbs: `parse_file()`, `generate_list()`

**Constants**:
- `SCREAMING_SNAKE_CASE`: `DEFAULT_STALE_DAYS`

### Documentation

All public items must have doc comments:

```rust
/// Calculate quality score for a repository
///
/// Quality score ranges from 0-100 based on:
/// - Star count (base score)
/// - Activity (commit recency)
/// - License presence
/// - Archive status
///
/// # Examples
///
/// ```
/// let score = repo.calculate_quality_score();
/// assert!(score <= 100);
/// ```
pub fn calculate_quality_score(&self) -> u8 {
    // implementation
}
```

### Error Handling

Use descriptive error messages with context:

```rust
// ❌ Bad
file.read()?;

// ✅ Good
file.read()
    .context(format!("Failed to read file: {}", path.display()))?;
```

## Testing Requirements

### Test Coverage

- **New features**: Must include tests
- **Bug fixes**: Add regression test
- **Minimum coverage**: 80% for new code
- **Integration tests**: For multi-component features

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = create_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test integration_test
```

## Pull Request Process

### Before Submitting

1. ✅ Code formatted: `cargo fmt`
2. ✅ Lints pass: `cargo clippy`
3. ✅ Tests pass: `cargo test`
4. ✅ Documentation updated
5. ✅ CHANGELOG.md entry added

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing performed

## Related Issues
Fixes #123

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No new warnings introduced
```

### Review Process

1. Automated checks run (formatting, linting, tests)
2. Code review by maintainer
3. Address feedback
4. Approval and merge

## Adding New Features

### Validation Rules

See [ARCHITECTURE.md](ARCHITECTURE.md#adding-new-validation-rules) for detailed guide.

**Steps**:
1. Create rule struct in `src/validators/rules.rs`
2. Implement `ValidationRule` trait
3. Add unit tests for the rule
4. Register in `src/main.rs` validate command
5. Document in README_PROCESSOR.md

### Data Models

**Steps**:
1. Define struct in appropriate `src/models/*.rs` file
2. Add `#[derive(Serialize, Deserialize)]`
3. Add field to `CanonicalData` if needed
4. Update merge logic if needed
5. Add unit tests for serialization/deserialization
6. Update templates if affects output

### CLI Commands

**Steps**:
1. Add variant to `Commands` enum in `src/main.rs`
2. Define arguments with clap attributes
3. Implement match arm in `main()`
4. Add help documentation
5. Add integration test if complex
6. Update UPDATE_RUNBOOK.md

## Documentation

### Required Documentation

**For New Features**:
- Doc comments in code
- Entry in CHANGELOG.md
- Update README_PROCESSOR.md
- Update UPDATE_RUNBOOK.md if affects workflow
- Add examples if non-trivial

**For Bug Fixes**:
- Comment explaining fix
- Entry in CHANGELOG.md
- Add regression test

### Documentation Style

- Use markdown for all documentation
- Include code examples
- Link to related documentation
- Keep examples up-to-date

## Code Review Checklist

### For Reviewers

- [ ] Code follows Rust idioms
- [ ] Error handling is appropriate
- [ ] No unwrap() in production code (use ? operator)
- [ ] Tests are comprehensive
- [ ] Documentation is clear
- [ ] Performance impact considered
- [ ] Backward compatibility maintained

### For Contributors

- [ ] Self-reviewed code
- [ ] Removed debug prints
- [ ] No commented-out code
- [ ] Meaningful commit messages
- [ ] Branch up-to-date with main

## Common Development Tasks

### Adding a Repository Field

```rust
// 1. Add to RepositoryMetadata struct
pub struct RepositoryMetadata {
    // ... existing fields
    pub new_field: Option<String>,
}

// 2. Update parser to extract field
// 3. Update templates if needed for display
// 4. Add tests
// 5. Increment schema_version if breaking change
```

### Modifying Template Output

```bash
# 1. Edit template
vim data/templates/list.md.tera

# 2. Test generation
cargo run -- generate --input data/canonical/merged_data.yml

# 3. Verify output
head -50 LIST.md

# 4. No code changes needed!
```

### Adding Validation Rule

```rust
// 1. Define rule in src/validators/rules.rs
pub struct MyCustomRule {
    threshold: u32,
}

// 2. Implement trait
impl ValidationRule for MyCustomRule {
    fn name(&self) -> &str { "my_custom_rule" }
    fn default_severity(&self) -> Severity { Severity::Warning }
    
    fn check(&self, data: &CanonicalData) -> ValidationResult {
        let issues = Vec::new();
        // Check logic here
        ValidationResult::multiple(issues)
    }
}

// 3. Add test
#[cfg(test)]
mod tests {
    #[test]
    fn test_my_custom_rule() {
        // Test logic
    }
}

// 4. Register in main.rs
validator.add_rule(MyCustomRule { threshold: 100 });
```

## Project Structure Conventions

### When to Create New Modules

- **New data type**: Add to `models/`
- **New validation**: Add to `validators/rules.rs`
- **New parser**: Add to `parsers/`
- **New output format**: Add to `generators/`
- **New CLI command**: Add to `main.rs` Commands enum

### When to Update Existing

- **Enhance data model**: Update existing model struct
- **Fix bug**: Update affected module
- **Improve validation**: Update existing rule

## Release Process

### Version Numbering

Follow Semantic Versioning (MAJOR.MINOR.PATCH):

- **MAJOR**: Breaking changes to output format or API
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Release Checklist

1. [ ] Update version in Cargo.toml
2. [ ] Update CHANGELOG.md
3. [ ] Run full test suite: `cargo test`
4. [ ] Build release: `cargo build --release`
5. [ ] Test release binary
6. [ ] Create git tag: `git tag v0.2.0`
7. [ ] Push tag: `git push origin v0.2.0`
8. [ ] Document breaking changes if any

## Getting Help

### Resources

- **Architecture**: See [ARCHITECTURE.md](ARCHITECTURE.md)
- **Usage**: See [UPDATE_RUNBOOK.md](UPDATE_RUNBOOK.md)
- **Processor Docs**: See [README_PROCESSOR.md](README_PROCESSOR.md)
- **Issues**: Check existing issues for known problems

### Asking Questions

- Open an issue with "Question:" prefix
- Include relevant code snippets
- Describe what you've tried
- Share error messages if applicable

## Code of Conduct

### Expected Behavior

- Be respectful and constructive
- Focus on the technical merits
- Help others learn
- Accept feedback gracefully

### Unacceptable Behavior

- Personal attacks
- Harassment
- Discriminatory language
- Publishing others' private information

## Recognition

Contributors will be:
- Listed in CHANGELOG.md for significant contributions
- Credited in commit messages
- Acknowledged in release notes

Thank you for contributing to OmniDatum Processor!

---

**Maintainer**: Repository owner  
**Last Updated**: 2024-12-10