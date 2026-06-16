//! od-core: Pure types, models, config, merge, parsers — zero IO dependencies on async/network.

pub mod config;
pub mod error;
pub mod merge;
pub mod models;
pub mod parsers;
pub mod readme_updater;

pub use config::{
    credentials::CredentialManager,
    settings::{CredentialSource, GenerationConfig, OmnidatumConfig, StatsDetailLevel, SyncConfig, ValidationConfig},
};
pub use error::CoreError;
pub use merge::{DataMerger, MergeStrategy};
pub use models::{
    Book, CanonicalData, Collection, ContentType, DifficultyLevel, ManualProject,
    ManualProjectClassification, ManualProjectMetadata, Platform, PlatformInfo, PlatformStatus,
    QualityMetrics, Relation, ReferenceStatus, Repository, RepositoryClassification,
    RepositoryMetadata, RepositorySource, SyncQualityReport, WebReference,
    AnomalyType, DataAnomaly, DataCompletenessMetrics, MigrationRecord, MigrationStatus,
};
pub use parsers::ListParser;

/// Result type for od-core operations
pub type Result<T> = std::result::Result<T, CoreError>;
