//! Configuration management for RepoQuery
//!
//! Handles application settings and secure credential storage.

pub mod credentials;
pub mod settings;

pub use credentials::CredentialManager;
pub use settings::{
    CredentialSource, GenerationConfig, RepoqueryConfig, StatsDetailLevel, StorageConfig,
    StorageMode, SyncConfig, ValidationConfig,
};
