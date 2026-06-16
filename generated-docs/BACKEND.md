# Backend Architecture

## Overview

OmniDatum's backend is a Rust CLI application designed for high-performance repository documentation processing. The architecture emphasizes type safety, modularity, and sub-second processing times.

## Project Structure

```
src/
├── lib.rs                 # Library root with re-exports
├── main.rs                # CLI application (429 lines)
├── models/                # Data structures (1,335 lines)
│   ├── repository.rs      # Core repository model (315 lines)
│   ├── platform.rs        # Multi-platform tracking (208 lines)
│   ├── canonical.rs       # Container model (176 lines)
│   └── [manual.rs, reference.rs, book.rs]
├── validators/            # Quality assurance (582 lines)
│   ├── framework.rs       # Validation engine (306 lines)
│   └── rules.rs           # 7 validation rules (276 lines)
├── parsers/               # Data extraction (165 lines)
├── generators/            # Output creation (181 lines)
├── cross_refs/            # Relationship tracking (410 lines)
├── merge.rs               # Data integration (264 lines)
└── readme_updater.rs      # README injection (72 lines)
```

**Total**: ~3,400 lines of production Rust code

## Data Flow

![Data Flow Diagram](../generated-diagrams/omnidatum_pipeline.png)

### Processing Pipeline

```
Input Sources → Parser → Merger → Validator → Generator → Output Files
     ↓            ↓        ↓         ↓          ↓           ↓
  LIST.md     repositories canonical  validation  templates  LIST.md
  Manual   →     .yml   →   .yml   →   cache   →  engine  → TABLE.md
  Web Refs                                                  → ARCHIVE.md
  Books
```

### Performance Characteristics

| Stage | Time | Memory | Details |
|-------|------|--------|---------|
| Parse | 50ms | 12MB | 839 repositories via regex |
| Merge | 30ms | 18MB | 4 data sources combined |
| Validate | 80ms | 20MB | 7 rules, 0 errors |
| Generate | 120ms | 15MB | Template rendering |
| **Total** | **280ms** | **65MB** | **End-to-end** |

## Core Components

### 1. Data Models (`models/`)

**Repository Structure**:
```rust
pub struct Repository {
    pub id: String,                           // Unique identifier
    pub platforms: Vec<PlatformInfo>,         // Multi-platform tracking
    pub metadata: RepositoryMetadata,         // Name, description, owner
    pub classification: RepositoryClassification, // Language, topics
    pub quality_metrics: QualityMetrics,      // Stars, scoring
    pub source: RepositorySource,             // Origin tracking
}
```

### 2. Validation System (`validators/`)

**7 Built-in Rules**:
- **E001**: No duplicate repository URLs
- **E002**: Missing license information (warning)
- **E003**: Valid URL formats
- **E004**: Cross-reference integrity
- **E005**: Platform migration completeness
- **E006**: Missing metadata
- **E007**: Stale content detection

### 3. Cross-Reference Graph (`cross_refs/`)

**Relationship Tracking**:
```rust
pub enum NodeType {
    Repository(String),
    WebReference(String), 
    Book(String),
    ReadmeSection(String),
}
```

Enables bidirectional queries and automatic navigation link generation.

### 4. Template System (`generators/`)

**Tera Integration**: Generates LIST.md, TABLE.md, ARCHIVE.md from canonical data using customizable templates with statistics and cross-references.
