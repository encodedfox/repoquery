//! Mock GitHub adapter for testing sync functionality

use async_trait::async_trait;
use omnidatum_processor::sync::DataSourceAdapter;
use omnidatum_processor::Repository;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock GitHub adapter for testing
#[derive(Clone)]
pub struct MockGitHubAdapter {
    responses: Arc<Mutex<HashMap<String, Repository>>>,
    connection_result: Arc<Mutex<Result<(), anyhow::Error>>>,
}

impl MockGitHubAdapter {
    /// Create new mock adapter
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            connection_result: Arc::new(Mutex::new(Ok(()))),
        }
    }

    /// Add a repository response
    pub fn add_response(&mut self, identifier: &str, repo: Repository) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(identifier.to_string(), repo);
    }

    /// Set connection check result
    pub fn set_connection_result(&mut self, result: Result<(), anyhow::Error>) {
        let mut conn_result = self.connection_result.lock().unwrap();
        *conn_result = result;
    }

    /// Get number of responses configured
    pub fn response_count(&self) -> usize {
        let responses = self.responses.lock().unwrap();
        responses.len()
    }
}

impl Default for MockGitHubAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSourceAdapter for MockGitHubAdapter {
    async fn fetch_repository(&self, identifier: &str) -> anyhow::Result<Repository> {
        let responses = self.responses.lock().unwrap();
        
        match responses.get(identifier) {
            Some(repo) => Ok(repo.clone()),
            None => Err(anyhow::anyhow!(
                "Repository '{}' not found in mock responses",
                identifier
            )),
        }
    }

    async fn check_connection(&self) -> anyhow::Result<()> {
        let result = self.connection_result.lock().unwrap();
        match &*result {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn source_name(&self) -> &str {
        "mock_github"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnidatum_processor::models::{
        Platform, PlatformInfo, PlatformStatus, QualityMetrics, RepositoryClassification,
        RepositoryMetadata, RepositorySource,
    };

    fn create_test_repo(full_name: &str, stars: u32) -> Repository {
        let parts: Vec<&str> = full_name.split('/').collect();
        Repository {
            id: format!("test-{}", full_name.replace('/', "-")),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: format!("https://github.com/{}", full_name),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: parts[1].to_string(),
                owner: parts[0].to_string(),
                full_name: full_name.to_string(),
                description: format!("Test repository: {}", full_name),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: Some("MIT".to_string()),
                stars,
                topics: vec!["test".to_string()],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: "Rust".to_string(),
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: false,
                archive_date: None,
                last_commit_date: Some("2024-12-10".to_string()),
                last_star_update: "2024-12-10".to_string(),
                quality_score: 70,
            },
            source: RepositorySource::GitHubStars,
            added_date: None,
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        }
    }

    #[tokio::test]
    async fn test_mock_adapter_fetch_success() {
        let mut adapter = MockGitHubAdapter::new();
        let repo = create_test_repo("rust-lang/rust", 50000);
        adapter.add_response("rust-lang/rust", repo.clone());

        let result = adapter.fetch_repository("rust-lang/rust").await;
        assert!(result.is_ok());
        
        let fetched = result.unwrap();
        assert_eq!(fetched.metadata.full_name, "rust-lang/rust");
        assert_eq!(fetched.metadata.stars, 50000);
    }

    #[tokio::test]
    async fn test_mock_adapter_fetch_not_found() {
        let adapter = MockGitHubAdapter::new();
        
        let result = adapter.fetch_repository("nonexistent/repo").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_mock_adapter_check_connection_success() {
        let adapter = MockGitHubAdapter::new();
        
        let result = adapter.check_connection().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_adapter_check_connection_failure() {
        let mut adapter = MockGitHubAdapter::new();
        adapter.set_connection_result(Err(anyhow::anyhow!("Connection failed")));
        
        let result = adapter.check_connection().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection failed"));
    }

    #[test]
    fn test_mock_adapter_source_name() {
        let adapter = MockGitHubAdapter::new();
        assert_eq!(adapter.source_name(), "mock_github");
    }

    #[test]
    fn test_mock_adapter_response_count() {
        let mut adapter = MockGitHubAdapter::new();
        assert_eq!(adapter.response_count(), 0);
        
        adapter.add_response("test/repo1", create_test_repo("test/repo1", 100));
        assert_eq!(adapter.response_count(), 1);
        
        adapter.add_response("test/repo2", create_test_repo("test/repo2", 200));
        assert_eq!(adapter.response_count(), 2);
    }
}