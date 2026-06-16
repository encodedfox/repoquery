# OmniDatum Documentation

Welcome to the OmniDatum documentation. This directory contains comprehensive guides for users and developers.

## Documentation Philosophy

OmniDatum documentation is organized by audience and purpose:
- **User Guides**: For end-users setting up and using OmniDatum
- **Developer Guides**: For contributors and developers extending the system
- **Integration Guides**: For adding new data sources
- **Reference**: Technical specifications and API documentation

## Getting Started

### For New Users
1. Start with the [main README](../README.md) for project overview and quick start
2. Read [DATA_SYNC.md](DATA_SYNC.md) to set up GitHub sync
3. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md) if you encounter issues

### For Developers
1. Review [ARCHITECTURE.md](ARCHITECTURE.md) to understand system design
2. Read [DEVELOPMENT.md](DEVELOPMENT.md) for development setup
3. Check [API_REFERENCE.md](API_REFERENCE.md) for CLI and library APIs
4. Explore [Integration Guides](#integration-guides) to add new data sources

### For Contributors
1. See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines
2. Review [DEVELOPMENT.md](DEVELOPMENT.md) for coding standards
3. Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common development issues

## User Documentation

### Setup and Usage
- **[DATA_SYNC.md](DATA_SYNC.md)** - Complete guide to setting up and using external data synchronization
  - Credential configuration (environment, file, keychain)
  - Sync commands and options
  - Scheduling automated syncs
  - Troubleshooting sync issues

### Reference
- **[API_REFERENCE.md](API_REFERENCE.md)** - Complete CLI command reference
  - All commands with examples
  - Flag descriptions
  - Usage patterns

- **[TROUBLESHOOTING.md](TROUBLESHOOTING.md)** - Common issues and solutions
  - Error code reference (E001-E008)
  - Authentication issues
  - Rate limit handling
  - Cache problems
  - Validation failures

## Developer Documentation

### Architecture and Design
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Complete system architecture
  - Design philosophy
  - Component architecture
  - Data flow (unidirectional)
  - Module descriptions
  - Performance characteristics
  - Extension points

### Development
- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development guide
  - Development environment setup
  - Build instructions
  - Testing strategy
  - Code style and conventions
  - Contribution workflow

## Integration Guides

Comprehensive guides for extending OmniDatum with new data sources:

- **[INTEGRATION_GUIDE_INDEX.md](INTEGRATION_GUIDE_INDEX.md)** - Overview and integration template
  - DataSourceAdapter trait reference
  - Integration patterns
  - Best practices

- **[INTEGRATION_GUIDE_GOOGLE_SHEETS.md](INTEGRATION_GUIDE_GOOGLE_SHEETS.md)** - Google Sheets integration
  - Complete GoogleSheetsAdapter implementation
  - Service account authentication
  - Sheet format specification
  - Configuration examples

- **[INTEGRATION_GUIDE_JIRA.md](INTEGRATION_GUIDE_JIRA.md)** - Jira integration
  - Complete JiraAdapter implementation
  - API authentication
  - Custom field configuration
  - Issue-to-Repository conversion

- **[INTEGRATION_GUIDE_GIT_REPOSITORY.md](INTEGRATION_GUIDE_GIT_REPOSITORY.md)** - Git repository scanner
  - Complete GitLocalAdapter implementation
  - Repository discovery
  - Metadata extraction
  - Language detection

## Diagrams

Visual representations of system architecture (Mermaid source files):

- **[diagrams/pipeline.mmd](diagrams/pipeline.mmd)** - Processing pipeline flow
- **[diagrams/data-flow.mmd](diagrams/data-flow.mmd)** - Unidirectional data flow
- **[diagrams/sync-sequence.mmd](diagrams/sync-sequence.mmd)** - GitHub API sync sequence
- **[diagrams/components.mmd](diagrams/components.mmd)** - Module architecture

To generate PNG/SVG from Mermaid sources:
```bash
./scripts/generate-diagrams.sh
```

## Specialized Topics

### Ecosystem Documentation
- **[ecosystems/bitwarden.md](ecosystems/bitwarden.md)** - Bitwarden ecosystem analysis

### Future Proposals
- **[aiml_modernization_proposal.md](aiml_modernization_proposal.md)** - AI/ML reference modernization proposal

## Historical Documentation

See **[archive/](archive/)** for historical documents including:
- Project summaries and status reports
- Baseline assessments
- Update runbooks and procedures

These are preserved for reference but no longer actively maintained.

## Contributing to Documentation

### Adding New Documentation

1. **User guides**: Add to docs/ root with descriptive filename
2. **Integration guides**: Follow the pattern in INTEGRATION_GUIDE_INDEX.md
3. **Diagrams**: Add Mermaid source to diagrams/, run generation script
4. **Architecture changes**: Update ARCHITECTURE.md

### Documentation Standards

- Use clear, concise language
- Include code examples for technical content
- Add links to related documentation
- Test all commands and code examples
- Update this README when adding new documents

### File Naming Conventions

- Use UPPERCASE for major guides (ARCHITECTURE.md, API_REFERENCE.md)
- Use Title_Case for specific topics (Data_Sync.md becomes DATA_SYNC.md)
- Use lowercase for subdirectories (diagrams/, archive/)
- Keep filenames descriptive but concise

## Documentation Organization

```
docs/
├── README.md (this file)           # Documentation index
│
├── User Guides/
│   ├── DATA_SYNC.md               # Sync setup and usage
│   ├── API_REFERENCE.md           # CLI command reference
│   └── TROUBLESHOOTING.md         # Common issues
│
├── Developer Guides/
│   ├── ARCHITECTURE.md            # System architecture
│   ├── DEVELOPMENT.md             # Development setup
│   ├── INTEGRATION_GUIDE_INDEX.md # Integration overview
│   ├── INTEGRATION_GUIDE_GOOGLE_SHEETS.md
│   ├── INTEGRATION_GUIDE_JIRA.md
│   └── INTEGRATION_GUIDE_GIT_REPOSITORY.md
│
├── diagrams/                      # Mermaid diagram sources
│   ├── pipeline.mmd
│   ├── data-flow.mmd
│   ├── sync-sequence.mmd
│   └── components.mmd
│
├── ecosystems/                    # Ecosystem analyses
│   └── bitwarden.md
│
├── archive/                       # Historical documents
│   ├── README.md
│   └── ... (8 archived files)
│
└── aiml_modernization_proposal.md # Future proposals
```

## Quick Reference

| Document | Purpose | Audience |
|----------|---------|----------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System design and components | Developers |
| [API_REFERENCE.md](API_REFERENCE.md) | CLI commands and library API | Users, Developers |
| [DATA_SYNC.md](DATA_SYNC.md) | Sync setup and usage | Users |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Development guide | Contributors |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Common issues | Users, Developers |
| [INTEGRATION_GUIDE_INDEX.md](INTEGRATION_GUIDE_INDEX.md) | Add data sources | Developers |

## Need Help?

- **General questions**: See main [README](../README.md)
- **Setup issues**: Check [DATA_SYNC.md](DATA_SYNC.md)
- **Errors**: Consult [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- **Contributing**: Read [CONTRIBUTING.md](../CONTRIBUTING.md)
- **Architecture questions**: Review [ARCHITECTURE.md](ARCHITECTURE.md)

---

**Last Updated**: 2025-12-11  
**Version**: 0.1.0  
**Documentation maintained by**: OmniDatum contributors