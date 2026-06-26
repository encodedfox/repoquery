//! Retry logic with exponential backoff for network operations
//!
//! # Security
//! All sensitive data (tokens, credentials) must be redacted before logging.
//! Use [`redact_sensitive`] for error messages that may contain API responses.

use anyhow::Result;
use std::time::Duration;

/// Redact sensitive patterns from a string for safe logging.
/// Strips common token patterns: ghp_*, gho_*, ghu_*, github_pat_*, Bearer tokens.
pub fn redact_sensitive(input: &str) -> String {
    let patterns = [
        ("ghp_", "ghp_"),
        ("gho_", "gho_"),
        ("ghu_", "ghu_"),
        ("github_pat_", "github_pat_"),
    ];
    let mut result = input.to_string();
    for (prefix, label) in &patterns {
        while let Some(start) = result.find(*prefix) {
            let end = start + prefix.len();
            let remaining = &result[end..];
            let token_end = remaining
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .unwrap_or(remaining.len());
            let replacement = format!("{}___REDACTED___", label);
            result.replace_range(start..end + token_end, &replacement);
        }
    }
    // Also redact "Bearer <token>" and "token <token>" patterns
    for keyword in &["Bearer ", "bearer ", "token ", "Token "] {
        while let Some(start) = result.find(*keyword) {
            let end = start + keyword.len();
            let remaining = &result[end..];
            let token_end = remaining
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .unwrap_or(remaining.len());
            if token_end > 0 {
                result.replace_range(end..end + token_end, "___REDACTED___");
            }
        }
    }
    result
}

/// Retry an async operation with exponential backoff
///
/// # Arguments
/// * `operation` - Async closure to retry
/// * `max_attempts` - Maximum retry attempts (default: 3)
/// * `base_delay_ms` - Base delay in milliseconds (default: 1000)
///
/// # Returns
/// Result from the operation if successful, or last error if all retries exhausted
pub async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    max_attempts: u32,
    base_delay_ms: u64,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 1;
    let mut delay = Duration::from_millis(base_delay_ms);

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt >= max_attempts {
                    tracing::error!("All {} retry attempts exhausted", max_attempts);
                    return Err(e);
                }

                // Check if error is retryable (network errors)
                let error_msg = e.to_string().to_lowercase();
                let is_retryable = error_msg.contains("timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("dns")
                    || error_msg.contains("network");

                if !is_retryable {
                    tracing::debug!("Error not retryable: {}", redact_sensitive(&e.to_string()));
                    return Err(e);
                }

                tracing::warn!(
                    "Attempt {}/{} failed: {}. Retrying in {:?}...",
                    attempt,
                    max_attempts,
                    redact_sensitive(&e.to_string()),
                    delay
                );

                tokio::time::sleep(delay).await;

                attempt += 1;
                delay *= 2; // Exponential backoff
            }
        }
    }
}

/// Retry with default parameters (3 attempts, 1 second base delay)
pub async fn retry_default<F, Fut, T>(operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    retry_with_backoff(operation, 3, 1000).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_with_backoff(
            || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, anyhow::Error>(42)
                }
            },
            3,
            100,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_with_backoff(
            || {
                let calls = calls_clone.clone();
                async move {
                    let count = calls.fetch_add(1, Ordering::SeqCst) + 1;
                    if count < 3 {
                        Err(anyhow::anyhow!("Network timeout"))
                    } else {
                        Ok(42)
                    }
                }
            },
            3,
            10,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_with_backoff(
            || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>(anyhow::anyhow!("Connection refused"))
                }
            },
            3,
            10,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_with_backoff(
            || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>(anyhow::anyhow!("Invalid parameter"))
                }
            },
            3,
            10,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1); // Should not retry non-network errors
    }
}
