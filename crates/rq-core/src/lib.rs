//! od-core: Pure types, models, config, merge, parsers — zero IO dependencies on async/network.

pub mod config;
pub mod error;
pub mod merge;
pub mod models;
pub mod parsers;
pub mod readme_updater;

pub use config::{
    credentials::CredentialManager,
    settings::{
        CredentialSource, GenerationConfig, RepoqueryConfig, StatsDetailLevel, StorageConfig,
        StorageMode, SyncConfig, ValidationConfig,
    },
};
pub use error::CoreError;
pub use merge::{DataMerger, MergeStrategy};
pub use models::{
    classify_activity, classify_all, summarize, trend_score, ActivityResult, ActivityStatus,
    ActivitySummary, AnomalyType, Book, CanonicalData, Collection, ContentType, DataAnomaly,
    DataCompletenessMetrics, DifficultyLevel, DomainRepository, EdgeType, FgatToken, IdentityAlias,
    ManualProject, ManualProjectClassification, ManualProjectMetadata, MigrationRecord,
    MigrationStatus, NormalizedDomain, Platform, PlatformInfo, PlatformKind, PlatformStatus,
    QualityMetrics, ReferenceStatus, Relation, Repository, RepositoryClassification,
    RepositoryMetadata, RepositorySource, Seed, SeedStatus, SyncQualityReport, TokenStatus,
    TraversalEdge, UnifiedIdentity, WebReference,
};
pub use parsers::ListParser;

/// Result type for od-core operations
pub type Result<T> = std::result::Result<T, CoreError>;
