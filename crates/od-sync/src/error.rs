//! Sync-specific error types

use thiserror::Error;

/// Sync operation errors
#[derive(Error, Debug)]
pub enum SyncError {
    /// Authentication with GitHub failed
    #[error("Authentication failed: {message}\n\nHelp: Run 'cargo run -- configure' to set up credentials")]
    AuthenticationFailed { message: String },

    /// GitHub API rate limit exceeded
    #[error("Rate limit exhausted. Please wait {wait_seconds} seconds.\nResets at: {reset_time}\n\nHelp: Use '--cache-ttl-hours=48' to reduce API calls")]
    RateLimitExceeded {
        wait_seconds: u64,
        reset_time: String,
    },

    /// Repository not found (404)
    #[error("Repository '{owner}/{name}' not found (404)\n\nThis may indicate:\n- Repository was deleted\n- Repository was made private\n- Repository was renamed\n\nHelp: The repository will be marked as deprecated")]
    RepositoryNotFound { owner: String, name: String },

    /// Network error (timeout, connection refused, DNS failure)
    #[error("Network error: {message}\n\nHelp: Check internet connection and try again")]
    NetworkError { message: String },

    /// Invalid configuration
    #[error("Invalid configuration: {message}\n\nHelp: Run 'cargo run -- configure --show' to review settings")]
    InvalidConfiguration { message: String },

    /// Cache operation failed
    #[error("Cache error: {message}\n\nHelp: Try 'cargo run -- sync --clear-cache' to reset cache")]
    CacheError { message: String },

    /// API response parsing failed
    #[error("Failed to parse GitHub API response: {message}")]
    ParseError { message: String },
}

impl SyncError {
    /// Create authentication failure error
    pub fn auth_failed(message: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            message: message.into(),
        }
    }

    /// Create rate limit error
    pub fn rate_limit(wait_seconds: u64, reset_time: impl Into<String>) -> Self {
        Self::RateLimitExceeded {
            wait_seconds,
            reset_time: reset_time.into(),
        }
    }

    /// Create repository not found error
    pub fn not_found(owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self::RepositoryNotFound {
            owner: owner.into(),
            name: name.into(),
        }
    }

    /// Create network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::NetworkError {
            message: message.into(),
        }
    }

    /// Create invalid configuration error
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfiguration {
            message: message.into(),
        }
    }

    /// Create cache error
    pub fn cache(message: impl Into<String>) -> Self {
        Self::CacheError {
            message: message.into(),
        }
    }

    /// Create parse error
    pub fn parse(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_message() {
        let err = SyncError::auth_failed("Invalid token");
        let msg = err.to_string();
        assert!(msg.contains("Authentication failed"));
        assert!(msg.contains("configure"));
    }

    #[test]
    fn test_rate_limit_error_message() {
        let err = SyncError::rate_limit(3600, "2024-12-11T22:00:00Z");
        let msg = err.to_string();
        assert!(msg.contains("3600 seconds"));
        assert!(msg.contains("2024-12-11T22:00:00Z"));
        assert!(msg.contains("cache-ttl-hours"));
    }

    #[test]
    fn test_not_found_error_message() {
        let err = SyncError::not_found("owner", "repo");
        let msg = err.to_string();
        assert!(msg.contains("owner/repo"));
        assert!(msg.contains("404"));
        assert!(msg.contains("deprecated"));
    }

    #[test]
    fn test_network_error_message() {
        let err = SyncError::network("Connection timeout");
        let msg = err.to_string();
        assert!(msg.contains("Network error"));
        assert!(msg.contains("Connection timeout"));
    }

    #[test]
    fn test_invalid_config_error() {
        let err = SyncError::invalid_config("parallel_workers must be 1-10");
        let msg = err.to_string();
        assert!(msg.contains("Invalid configuration"));
        assert!(msg.contains("configure --show"));
    }

    #[test]
    fn test_cache_error() {
        let err = SyncError::cache("Failed to read cache file");
        let msg = err.to_string();
        assert!(msg.contains("Cache error"));
        assert!(msg.contains("clear-cache"));
    }
}