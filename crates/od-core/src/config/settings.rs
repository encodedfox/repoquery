//! Application configuration settings

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OmnidatumConfig {
    pub sync: SyncConfig,
    pub credentials: CredentialsConfig,
    pub validation: ValidationConfig,
    pub generation: GenerationConfig,
}

/// Sync-related configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncConfig {
    /// Enable automatic sync
    pub enabled: bool,
    /// Sync interval in hours
    pub interval_hours: u32,
    /// Number of parallel workers
    pub parallel_workers: u8,
    /// Cache TTL in hours
    pub cache_ttl_hours: u32,
    /// Rate limit buffer (requests to leave unused)
    pub rate_limit_buffer: u16,
    /// Timeout for individual requests (seconds)
    pub request_timeout_secs: u64,
}

/// Credentials configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialsConfig {
    /// Source: env, file, or keychain
    pub source: CredentialSource,
    /// Path to credentials file (if source=file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<PathBuf>,
}

/// Credential storage source
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialSource {
    /// Read from environment variables
    Env,
    /// Read from file
    File,
    /// Read from system keychain
    Keychain,
}

/// Validation configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidationConfig {
    /// Map of error code to enabled status
    pub rules: HashMap<String, bool>,
}

/// Generation configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerationConfig {
    /// Include archived repositories in separate files
    pub include_archived: bool,
    /// Include multi-platform information in output
    pub platform_info: bool,
    /// Statistics detail level
    pub stats_detail_level: StatsDetailLevel,
}

/// Statistics detail level for output
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatsDetailLevel {
    /// Minimal statistics
    Minimal,
    /// Standard statistics (default)
    Standard,
    /// Detailed statistics
    Detailed,
}

impl OmnidatumConfig {
    /// Load configuration from file with fallback to defaults
    pub fn load() -> crate::Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Self = toml::from_str(&content)?;
            config.validate()?;
            Ok(config)
        } else {
            tracing::debug!(
                "Config file not found at {:?}, using defaults",
                config_path
            );
            Ok(Self::default())
        }
    }

    /// Get configuration file path
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("omnidatum");
        config_dir.join("config.toml")
    }

    /// Validate configuration values
    pub fn validate(&self) -> crate::Result<()> {
        if self.sync.parallel_workers < 1 || self.sync.parallel_workers > 10 {
            return Err(crate::CoreError::Config(format!(
                "parallel_workers must be between 1 and 10, got {}",
                self.sync.parallel_workers
            )));
        }
        if self.sync.cache_ttl_hours < 1 || self.sync.cache_ttl_hours > 168 {
            return Err(crate::CoreError::Config(format!(
                "cache_ttl_hours must be between 1 and 168 (1 week), got {}",
                self.sync.cache_ttl_hours
            )));
        }
        if self.sync.rate_limit_buffer > 1000 {
            return Err(crate::CoreError::Config(format!(
                "rate_limit_buffer must be between 0 and 1000, got {}",
                self.sync.rate_limit_buffer
            )));
        }
        Ok(())
    }

    /// Save configuration to file
    pub fn save(&self) -> crate::Result<()> {
        let config_path = Self::config_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Serialize to TOML with pretty formatting
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        tracing::info!("Configuration saved to {:?}", config_path);
        Ok(())
    }
}

impl Default for OmnidatumConfig {
    fn default() -> Self {
        Self {
            sync: SyncConfig {
                enabled: false,
                interval_hours: 24,
                parallel_workers: 3,
                cache_ttl_hours: 24,
                rate_limit_buffer: 500,
                request_timeout_secs: 30,
            },
            credentials: CredentialsConfig {
                source: CredentialSource::Env,
                file_path: None,
            },
            validation: ValidationConfig {
                rules: [
                    ("E001".to_string(), true),
                    ("E002".to_string(), true),
                    ("E003".to_string(), true),
                    ("E004".to_string(), true),
                    ("E005".to_string(), true),
                    ("E006".to_string(), true),
                    ("E007".to_string(), true),
                    ("E008".to_string(), true),
                ]
                .iter()
                .cloned()
                .collect(),
            },
            generation: GenerationConfig {
                include_archived: true,
                platform_info: false,
                stats_detail_level: StatsDetailLevel::Standard,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = OmnidatumConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_parallel_workers_too_low() {
        let mut config = OmnidatumConfig::default();
        config.sync.parallel_workers = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_parallel_workers_too_high() {
        let mut config = OmnidatumConfig::default();
        config.sync.parallel_workers = 11;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_cache_ttl_too_low() {
        let mut config = OmnidatumConfig::default();
        config.sync.cache_ttl_hours = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_cache_ttl_too_high() {
        let mut config = OmnidatumConfig::default();
        config.sync.cache_ttl_hours = 169; // More than 1 week
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_rate_limit_buffer_too_high() {
        let mut config = OmnidatumConfig::default();
        config.sync.rate_limit_buffer = 1001;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid_ranges() {
        let mut config = OmnidatumConfig::default();
        config.sync.parallel_workers = 5;
        config.sync.cache_ttl_hours = 72;
        config.sync.rate_limit_buffer = 750;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_credential_source_serialization() {
        let source = CredentialSource::Env;
        let serialized = serde_json::to_string(&source).unwrap();
        assert_eq!(serialized, "\"env\"");

        let source = CredentialSource::File;
        let serialized = serde_json::to_string(&source).unwrap();
        assert_eq!(serialized, "\"file\"");

        let source = CredentialSource::Keychain;
        let serialized = serde_json::to_string(&source).unwrap();
        assert_eq!(serialized, "\"keychain\"");
    }

    #[test]
    fn test_stats_detail_level_serialization() {
        let level = StatsDetailLevel::Minimal;
        let serialized = serde_json::to_string(&level).unwrap();
        assert_eq!(serialized, "\"minimal\"");

        let level = StatsDetailLevel::Standard;
        let serialized = serde_json::to_string(&level).unwrap();
        assert_eq!(serialized, "\"standard\"");

        let level = StatsDetailLevel::Detailed;
        let serialized = serde_json::to_string(&level).unwrap();
        assert_eq!(serialized, "\"detailed\"");
    }
}