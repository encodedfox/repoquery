use crate::platform::{PlatformApiClient, PlatformRepo};
use crate::trust::TrustEntry;
use anyhow::Result;
use rq_core::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository, RepositoryClassification,
    RepositoryMetadata, RepositorySource,
};
use rq_store::RepoStore;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Report from a collection run.
#[derive(Debug, Default, Clone)]
pub struct CollectReport {
    pub users_processed: u64,
    pub repos_collected: u64,
    pub repos_merged: u64,
    pub errors: u64,
}

/// Collects repositories from discovered users and merges them into a RepoStore.
pub struct RepoCollector {
    store: Arc<Mutex<dyn RepoStore + Send>>,
    client: Arc<dyn PlatformApiClient + Send + Sync>,
}

impl RepoCollector {
    pub fn new(
        store: Arc<Mutex<dyn RepoStore + Send>>,
        client: Arc<dyn PlatformApiClient + Send + Sync>,
    ) -> Self {
        Self { store, client }
    }

    /// Process discovered users and collect their repositories.
    ///
    /// For each user with trust ≥ `min_trust`:
    /// 1. Fetch their public repos via the platform API
    /// 2. Convert to `Repository` with `discovered_via` attribution
    /// 3. Upsert into the `RepoStore`
    pub async fn collect(
        &self,
        users: &[TrustEntry],
        min_trust: f64,
        discovered_via: &str,
    ) -> Result<CollectReport> {
        let mut report = CollectReport::default();

        let candidates: Vec<&TrustEntry> = users.iter().filter(|u| u.score >= min_trust).collect();
        if candidates.is_empty() {
            tracing::info!("No users meet the minimum trust threshold ({})", min_trust);
            return Ok(report);
        }

        tracing::info!(
            "Collecting repos for {} users (threshold: {})",
            candidates.len(),
            min_trust
        );

        for entry in &candidates {
            match self.collect_user(entry, discovered_via).await {
                Ok(count) => {
                    report.users_processed += 1;
                    report.repos_collected += count as u64;
                    report.repos_merged += count as u64;
                }
                Err(e) => {
                    tracing::error!("Failed to collect repos for {}: {}", entry.username, e);
                    report.errors += 1;
                }
            }
        }

        Ok(report)
    }

    async fn collect_user(&self, entry: &TrustEntry, discovered_via: &str) -> Result<usize> {
        let repos = self.client.fetch_user_repos(&entry.username).await?;
        let mut count = 0;

        for prepo in &repos {
            // Skip forks — they're just clones of other repos.
            if prepo.fork {
                continue;
            }

            let repo = self.platform_repo_to_repository(prepo, discovered_via);
            let guard = self.store.lock().await;
            guard.upsert_repo(&repo)?;
            count += 1;
        }

        tracing::debug!("Collected {} repos from {}", count, entry.username);

        Ok(count)
    }

    fn platform_repo_to_repository(
        &self,
        prepo: &PlatformRepo,
        discovered_via: &str,
    ) -> Repository {
        let id = format!("github-com-{}", prepo.full_name.replace('/', "-"));
        let platform = PlatformInfo {
            platform: Platform::GitHub,
            url: prepo.html_url.clone(),
            status: if prepo.is_archived {
                PlatformStatus::Archived
            } else {
                PlatformStatus::Active
            },
            is_primary: true,
            migration_date: None,
            last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            notes: None,
        };

        Repository {
            id,
            platforms: vec![platform],
            metadata: RepositoryMetadata {
                name: prepo.name.clone(),
                owner: prepo.owner.clone(),
                full_name: prepo.full_name.clone(),
                description: prepo.description.clone().unwrap_or_default(),
                primary_language: prepo
                    .language
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string()),
                license: prepo.license.clone(),
                license_spdx: prepo.license.clone(),
                stars: prepo.stars,
                topics: prepo.topics.clone(),
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: prepo
                    .language
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string()),
                language_notes: None,
                readme_inclusion: prepo.stars > 2000,
                readme_inclusion_reason: if prepo.stars > 2000 {
                    Some("star_threshold".to_string())
                } else {
                    None
                },
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: prepo.is_archived,
                archive_date: if prepo.is_archived {
                    Some(chrono::Utc::now().format("%Y-%m-%d").to_string())
                } else {
                    None
                },
                last_commit_date: None,
                last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                quality_score: QualityMetrics::calculate_score(
                    prepo.stars,
                    prepo.license.is_some(),
                    prepo.description.is_some(),
                    !prepo.topics.is_empty(),
                    prepo.is_archived,
                ),
            },
            source: RepositorySource::GitHubStars,
            added_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
            fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
            domain: None,
            unified_owner_id: None,
            discovered_via: Some(discovered_via.to_string()),
        }
    }
}
