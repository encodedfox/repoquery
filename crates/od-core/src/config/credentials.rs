//! Secure credential management
//!
//! Handles secure storage and retrieval of GitHub tokens and other credentials.

use super::settings::CredentialSource;
use std::fs;
use std::path::PathBuf;

/// Credential manager for secure token storage
pub struct CredentialManager {
    source: CredentialSource,
}

impl CredentialManager {
    /// Create new credential manager with specified source
    pub fn new(source: CredentialSource) -> Self {
        Self { source }
    }

    /// Get GitHub token from configured source
    pub fn get_github_token(&self) -> crate::Result<String> {
        match self.source {
            CredentialSource::Env => self.load_from_env(),
            CredentialSource::File => self.load_from_file(),
            CredentialSource::Keychain => self.load_from_keychain(),
        }
    }

    /// Read token from environment variable
    fn load_from_env(&self) -> crate::Result<String> {
        std::env::var("GITHUB_TOKEN")
            .or_else(|_| std::env::var("GH_TOKEN"))
            .map_err(|_| crate::CoreError::Config(
                "GitHub token not found in environment. Set GITHUB_TOKEN or GH_TOKEN, \
                 or run 'cargo run -- configure'".to_string()
            ))
    }

    /// Read token from file
    fn load_from_file(&self) -> crate::Result<String> {
        let path = Self::credentials_file_path();

        // Check file permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&path).map_err(|_| crate::CoreError::Config(
                "Credentials file not found. Run 'cargo run -- configure' to set up credentials.".to_string()
            ))?;
            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                return Err(crate::CoreError::Config(format!(
                    "Credentials file has insecure permissions ({:o}). Run: chmod 600 {:?}",
                    mode & 0o777,
                    path
                )));
            }
        }

        let content = fs::read_to_string(&path)?;
        Ok(content.trim().to_string())
    }

    /// Read token from system keychain (OS-specific)
    fn load_from_keychain(&self) -> crate::Result<String> {
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("security")
                .args([
                    "find-generic-password",
                    "-a", "omnidatum",
                    "-s", "github-token",
                    "-w",
                ])
                .output()?;

            if output.status.success() {
                let token = String::from_utf8(output.stdout)
                    .map_err(|e| crate::CoreError::Config(e.to_string()))?;
                return Ok(token.trim().to_string());
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("secret-tool")
                .args([
                    "lookup", "application", "omnidatum",
                    "type", "github-token",
                ])
                .output()?;

            if output.status.success() {
                let token = String::from_utf8(output.stdout)
                    .map_err(|e| crate::CoreError::Config(e.to_string()))?;
                return Ok(token.trim().to_string());
            }
        }

        Err(crate::CoreError::Config(
            "Keychain access not available on this platform. \
             Use environment variable or file-based credentials.".to_string()
        ))
    }

    /// Store token in file with secure permissions
    pub fn store_token(&self, token: &str) -> crate::Result<()> {
        let path = Self::credentials_file_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, token.trim())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path)?.permissions();
            permissions.set_mode(0o600);
            fs::set_permissions(&path, permissions)?;
        }

        tracing::info!("Token stored securely at {:?}", path);
        Ok(())
    }

    /// Get credentials file path
    fn credentials_file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("omnidatum")
            .join("credentials")
    }

    /// Redact token for logging (show first 4 chars + "***REDACTED***")
    pub fn redact(token: &str) -> String {
        if token.len() > 8 {
            format!("{}...***REDACTED***", &token[..4])
        } else {
            "***REDACTED***".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_redact_token() {
        let token = "ghp_1234567890abcdefghij";
        let redacted = CredentialManager::redact(token);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("1234567890"));
        assert!(redacted.starts_with("ghp_"));
    }

    #[test]
    fn test_redact_short_token() {
        let token = "short";
        let redacted = CredentialManager::redact(token);
        assert_eq!(redacted, "***REDACTED***");
    }

    #[test]
    #[serial]
    fn test_load_from_env() {
        // Set environment variable for this test
        let test_token = "test_token_value_12345";
        std::env::set_var("GITHUB_TOKEN", test_token);
        
        let mgr = CredentialManager::new(CredentialSource::Env);
        let token = mgr.get_github_token().unwrap();
        assert_eq!(token, test_token);
        
        // Cleanup
        std::env::remove_var("GITHUB_TOKEN");
    }

    #[test]
    #[serial]
    fn test_load_from_env_fallback() {
        // Remove primary, set fallback
        let test_token = "fallback_token_67890";
        std::env::remove_var("GITHUB_TOKEN");
        std::env::set_var("GH_TOKEN", test_token);
        
        let mgr = CredentialManager::new(CredentialSource::Env);
        let token = mgr.get_github_token().unwrap();
        assert_eq!(token, test_token);
        
        // Cleanup
        std::env::remove_var("GH_TOKEN");
    }

    #[test]
    #[serial]
    fn test_load_from_env_missing() {
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("GH_TOKEN");
        let mgr = CredentialManager::new(CredentialSource::Env);
        let result = mgr.get_github_token();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cargo run -- configure"));
    }

}