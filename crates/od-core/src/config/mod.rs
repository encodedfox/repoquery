//! Configuration management for OmniDatum
//!
//! Handles application settings and secure credential storage.

pub mod credentials;
pub mod settings;

pub use credentials::CredentialManager;
pub use settings::{
    CredentialSource, GenerationConfig, OmnidatumConfig, StatsDetailLevel, SyncConfig,
    ValidationConfig,
};