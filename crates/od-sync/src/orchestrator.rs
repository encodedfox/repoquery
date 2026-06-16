//! Sync orchestrator for coordinating external data fetches.

use crate::{SyncCache, SyncResult, ProgressTracker};
use crate::adapters::{DataSourceAdapter, GitHubAdapter};
use od_core::config::OmnidatumConfig;
use od_core::{CanonicalData, Relation, Repository, SyncQualityReport, DataCompletenessMetrics, DataAnomaly, AnomalyType};
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, error, info, info_span, warn, Instrument};

/// Sync orchestrator for coordinating external data fetches
pub struct SyncOrchestrator {
    config: OmnidatumConfig,
    cache: SyncCache,
    progress: ProgressTracker,
}

impl SyncOrchestrator {
    /// Create new sync orchestrator
    pub fn new(config: OmnidatumConfig) -> Result<Self> {
        let cache = SyncCache::load()?;
        let progress = ProgressTracker::new();

        Ok(Self {
            config,
            cache,
            progress,
        })
    }

    /// Get mutable reference to cache (for clearing)
    pub fn cache_mut(&mut self) -> &mut SyncCache {
        &mut self.cache
    }

    /// Sync all repositories from configured sources
    pub async fn sync_all(&mut self, canonical_path: &Path) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        let mut data = CanonicalData::from_yaml_file(canonical_path)?;
        let pre_sync_data = data.clone();

        let span = info_span!("sync_all", total_repos = data.repositories.len());
        let _enter = span.enter();

        info!("Starting sync for {} repositories", data.repositories.len());
        self.progress.start(data.repositories.len());

        let adapter = Arc::new(GitHubAdapter::new(&self.config).await?);
        let semaphore = Arc::new(Semaphore::new(self.config.sync.parallel_workers as usize));

        let mut synced = 0usize;
        let mut cached = 0usize;
        let mut failed = 0usize;
        let mut failures: Vec<(String, String)> = Vec::new();

        // First pass: identify work items
        let mut work_items: Vec<(usize, String, String)> = Vec::new();
        for (i, repo) in data.repositories.iter().enumerate() {
            if let Some(entry) = self.cache.get(&repo.id) {
                if entry.is_fresh(self.config.sync.cache_ttl_hours) {
                    cached += 1;
                    self.progress.increment_cached();
                    continue;
                }
            }
            let is_github_primary = repo
                .platforms
                .iter()
                .any(|p| p.is_primary && matches!(p.platform, od_core::Platform::GitHub));
            if !is_github_primary {
                cached += 1;
                self.progress.increment_cached();
                continue;
            }
            match Self::parse_github_repo(repo) {
                Ok((owner, name)) => work_items.push((i, owner, name)),
                Err(e) => {
                    warn!("Failed to parse repo {}: {}", repo.id, e);
                    failed += 1;
                    failures.push((repo.id.clone(), e.to_string()));
                    self.progress.increment_failed();
                }
            }
        }

        // Spawn concurrent fetches
        let mut join_set: JoinSet<(usize, String, Result<Repository>)> = JoinSet::new();
        for (idx, owner, name) in work_items {
            let adapter = Arc::clone(&adapter);
            let sem = Arc::clone(&semaphore);
            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                let identifier = format!("{}/{}", owner, name);
                let span = info_span!("sync_repo", repo = %identifier);
                let result = async { adapter.fetch_repository(&identifier).await }
                    .instrument(span)
                    .await;
                (idx, identifier, result)
            });
        }

        // Collect results
        while let Some(task_result) = join_set.join_next().await {
            match task_result {
                Ok((idx, ident, Ok(updated_repo))) => {
                    self.merge_sync_data(&mut data.repositories[idx], updated_repo);
                    self.cache.insert(&data.repositories[idx].id, &data.repositories[idx].metadata);
                    synced += 1;
                    self.progress.increment_synced(&ident);
                }
                Ok((_idx, ident, Err(e))) => {
                    error!("Failed to sync {}: {}", ident, e);
                    failed += 1;
                    failures.push((ident, e.to_string()));
                    self.progress.increment_failed();
                }
                Err(e) => {
                    error!("Sync task panicked: {}", e);
                    failed += 1;
                }
            }
        }

        data.last_updated = chrono::Utc::now().to_rfc3339();
        data.to_yaml_file(canonical_path)?;
        self.cache.save()?;

        if synced > 0 {
            match self.generate_quality_report(&pre_sync_data, &data, synced) {
                Ok(report) => {
                    info!(
                        "Quality metrics - License: {:.1}%, Description: {:.1}%, Topics: {:.1}%, Anomalies: {}",
                        report.data_completeness.license_coverage,
                        report.data_completeness.description_coverage,
                        report.data_completeness.topics_coverage,
                        report.anomalies.len()
                    );
                    if let Err(e) = self.save_quality_report(&report) {
                        warn!("Failed to save quality report: {}", e);
                    }
                }
                Err(e) => warn!("Failed to generate quality report: {}", e),
            }
        }

        let duration = start.elapsed();
        self.progress.finish();

        info!("Sync complete: {} synced, {} cached, {} failed in {:?}", synced, cached, failed, duration);

        let mut by_relation: HashMap<String, usize> = HashMap::new();
        *by_relation.entry("starred".to_string()).or_insert(0) += synced;

        Ok(SyncResult {
            total: data.repositories.len(),
            synced,
            cached,
            failed,
            duration,
            failures,
            by_relation,
        })
    }

    /// Sync repositories by relation type, fetching new repos from GitHub as needed
    pub async fn sync_by_relation(
        &mut self,
        relations: &[Relation],
        canonical_path: &Path,
    ) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        let mut data = CanonicalData::from_yaml_file(canonical_path)?;
        let adapter = Arc::new(GitHubAdapter::new(&self.config).await?);

        let mut synced = 0usize;
        let cached = 0usize;
        let failed = 0usize;
        let failures: Vec<(String, String)> = Vec::new();
        let mut by_relation: HashMap<String, usize> = HashMap::new();

        for relation in relations {
            match relation {
                Relation::Starred => {
                    // Bulk GraphQL fetch — ~10 requests instead of 845 individual REST calls
                    let repos = adapter.fetch_starred_graphql().await.unwrap_or_else(|e| {
                        warn!("fetch_starred_graphql failed: {}", e);
                        vec![]
                    });
                    let count = self.merge_fetched_repos(&mut data, repos);
                    synced += count;
                    *by_relation.entry("starred".to_string()).or_insert(0) += count;
                }
                Relation::Owned => {
                    let repos = adapter.fetch_user_repos().await.unwrap_or_else(|e| {
                        warn!("fetch_user_repos failed: {}", e);
                        vec![]
                    });
                    let count = self.merge_fetched_repos(&mut data, repos);
                    synced += count;
                    *by_relation.entry("owned".to_string()).or_insert(0) += count;
                }
                Relation::Forked => {
                    let repos = adapter.fetch_user_forks().await.unwrap_or_else(|e| {
                        warn!("fetch_user_forks failed: {}", e);
                        vec![]
                    });
                    let count = self.merge_fetched_repos(&mut data, repos);
                    synced += count;
                    *by_relation.entry("forked".to_string()).or_insert(0) += count;
                }
                Relation::Watching => {
                    let repos = adapter.fetch_watched_repos().await.unwrap_or_else(|e| {
                        warn!("fetch_watched_repos failed: {}", e);
                        vec![]
                    });
                    let count = self.merge_fetched_repos(&mut data, repos);
                    synced += count;
                    *by_relation.entry("watching".to_string()).or_insert(0) += count;
                }
                Relation::OrgMember => {
                    // org name must come from config or caller; skip if not available
                    warn!("OrgMember sync requires an org name — skipping (use sync_org_repos directly)");
                }
                other => {
                    debug!("Relation {:?} does not have a fetch strategy, skipping", other);
                }
            }
        }

        data.last_updated = chrono::Utc::now().to_rfc3339();
        data.to_yaml_file(canonical_path)?;
        self.cache.save()?;

        let duration = start.elapsed();
        Ok(SyncResult {
            total: data.repositories.len(),
            synced,
            cached,
            failed,
            duration,
            failures,
            by_relation,
        })
    }

    /// Sync repos for a specific GitHub organisation
    pub async fn sync_org_repos(
        &mut self,
        org: &str,
        canonical_path: &Path,
    ) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        let mut data = CanonicalData::from_yaml_file(canonical_path)?;
        let adapter = GitHubAdapter::new(&self.config).await?;

        let repos = adapter.fetch_org_repos(org).await.unwrap_or_else(|e| {
            warn!("fetch_org_repos({}) failed: {}", org, e);
            vec![]
        });
        let synced = self.merge_fetched_repos(&mut data, repos);

        data.last_updated = chrono::Utc::now().to_rfc3339();
        data.to_yaml_file(canonical_path)?;
        self.cache.save()?;

        let mut by_relation = HashMap::new();
        by_relation.insert("org_member".to_string(), synced);

        Ok(SyncResult {
            total: data.repositories.len(),
            synced,
            cached: 0,
            failed: 0,
            duration: start.elapsed(),
            failures: vec![],
            by_relation,
        })
    }

    /// Merge a batch of freshly-fetched repos into canonical data.
    /// Repos already present (matched by full_name) get their relations merged.
    /// New repos are appended.
    /// Returns the number of repos added or updated.
    fn merge_fetched_repos(&mut self, data: &mut CanonicalData, fetched: Vec<Repository>) -> usize {
        let mut count = 0;
        for repo in fetched {
            let existing = data
                .repositories
                .iter_mut()
                .find(|r| r.metadata.full_name.eq_ignore_ascii_case(&repo.metadata.full_name));
            match existing {
                Some(existing_repo) => {
                    // Merge relations without duplicates
                    for rel in &repo.relations {
                        if !existing_repo.relations.contains(rel) {
                            existing_repo.relations.push(rel.clone());
                        }
                    }
                    self.merge_sync_data(existing_repo, repo);
                    self.cache.insert(&existing_repo.id, &existing_repo.metadata);
                }
                None => {
                    self.cache.insert(&repo.id, &repo.metadata);
                    data.repositories.push(repo);
                }
            }
            count += 1;
        }
        count
    }

    /// Generate quality report from sync data
    pub fn generate_quality_report(
        &self,
        pre_sync_data: &CanonicalData,
        post_sync_data: &CanonicalData,
        synced_count: usize,
    ) -> Result<SyncQualityReport> {
        // Calculate data completeness metrics
        let mut with_license = 0;
        let mut with_description = 0;
        let mut with_topics = 0;
        let mut with_homepage = 0;
        let total = post_sync_data.repositories.len() as f32;

        for repo in &post_sync_data.repositories {
            if repo.metadata.license.is_some() {
                with_license += 1;
            }
            if !repo.metadata.description.is_empty() {
                with_description += 1;
            }
            if !repo.metadata.topics.is_empty() {
                with_topics += 1;
            }
            if repo.metadata.homepage.is_some() {
                with_homepage += 1;
            }
        }

        let data_completeness = DataCompletenessMetrics {
            license_coverage: (with_license as f32 / total) * 100.0,
            description_coverage: (with_description as f32 / total) * 100.0,
            topics_coverage: (with_topics as f32 / total) * 100.0,
            homepage_coverage: (with_homepage as f32 / total) * 100.0,
        };

        // Detect anomalies by comparing pre-sync and post-sync data
        let mut anomalies = Vec::new();

        // Create maps for quick lookup
        use std::collections::HashMap;
        let pre_map: HashMap<_, _> = pre_sync_data
            .repositories
            .iter()
            .map(|r| (r.id.clone(), r))
            .collect();

        for repo in &post_sync_data.repositories {
            if let Some(pre_repo) = pre_map.get(&repo.id) {
                // Check for star drops >25%
                if pre_repo.metadata.stars > 100 {
                    let old_stars = pre_repo.metadata.stars as f32;
                    let new_stars = repo.metadata.stars as f32;
                    let drop_pct = ((old_stars - new_stars) / old_stars) * 100.0;

                    if drop_pct > 25.0 {
                        anomalies.push(DataAnomaly {
                            repo_id: repo.id.clone(),
                            anomaly_type: AnomalyType::StarDrop,
                            message: format!(
                                "Star count dropped {:.1}% ({} → {})",
                                drop_pct, pre_repo.metadata.stars, repo.metadata.stars
                            ),
                            old_value: Some(pre_repo.metadata.stars.to_string()),
                            new_value: Some(repo.metadata.stars.to_string()),
                        });
                    }
                }

                // Check for star spikes >100%
                if pre_repo.metadata.stars > 0 {
                    let old_stars = pre_repo.metadata.stars as f32;
                    let new_stars = repo.metadata.stars as f32;
                    let spike_pct = ((new_stars - old_stars) / old_stars) * 100.0;

                    if spike_pct > 100.0 {
                        anomalies.push(DataAnomaly {
                            repo_id: repo.id.clone(),
                            anomaly_type: AnomalyType::StarSpike,
                            message: format!(
                                "Star count spiked {:.1}% ({} → {})",
                                spike_pct, pre_repo.metadata.stars, repo.metadata.stars
                            ),
                            old_value: Some(pre_repo.metadata.stars.to_string()),
                            new_value: Some(repo.metadata.stars.to_string()),
                        });
                    }
                }

                // Check for description changes >80% (using simple length comparison)
                if !pre_repo.metadata.description.is_empty()
                    && !repo.metadata.description.is_empty()
                {
                    let old_len = pre_repo.metadata.description.len() as f32;
                    let new_len = repo.metadata.description.len() as f32;
                    let diff_pct = ((old_len - new_len).abs() / old_len) * 100.0;

                    if diff_pct > 80.0 {
                        anomalies.push(DataAnomaly {
                            repo_id: repo.id.clone(),
                            anomaly_type: AnomalyType::DescriptionChange,
                            message: format!(
                                "Description changed significantly ({:.1}% difference)",
                                diff_pct
                            ),
                            old_value: Some(pre_repo.metadata.description.clone()),
                            new_value: Some(repo.metadata.description.clone()),
                        });
                    }
                }

                // Check for language changes
                if pre_repo.metadata.primary_language != repo.metadata.primary_language
                    && pre_repo.metadata.primary_language != "Unknown"
                    && repo.metadata.primary_language != "Unknown"
                {
                    anomalies.push(DataAnomaly {
                        repo_id: repo.id.clone(),
                        anomaly_type: AnomalyType::LanguageChange,
                        message: format!(
                            "Primary language changed: {} → {}",
                            pre_repo.metadata.primary_language, repo.metadata.primary_language
                        ),
                        old_value: Some(pre_repo.metadata.primary_language.clone()),
                        new_value: Some(repo.metadata.primary_language.clone()),
                    });
                }

                // Check for newly archived repositories
                if !pre_repo.quality_metrics.archive_status
                    && repo.quality_metrics.archive_status
                {
                    anomalies.push(DataAnomaly {
                        repo_id: repo.id.clone(),
                        anomaly_type: AnomalyType::NewlyArchived,
                        message: "Repository was archived since last sync".to_string(),
                        old_value: Some("active".to_string()),
                        new_value: Some("archived".to_string()),
                    });
                }
            }
        }

        Ok(SyncQualityReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            repos_synced: synced_count,
            data_completeness,
            anomalies,
        })
    }

    /// Save quality report to default location
    pub fn save_quality_report(&self, report: &SyncQualityReport) -> Result<()> {
        let report_path = PathBuf::from("data/cache/sync_quality_report.json");
        report.to_json_file(&report_path)?;
        info!("Quality report saved to {}", report_path.display());
        Ok(())
    }

    /// Sync specific repositories by full_name
    pub async fn sync_specific(
        &mut self,
        repo_identifiers: Vec<String>,
        canonical_path: &Path,
    ) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        // Load existing canonical data
        let mut data = CanonicalData::from_yaml_file(canonical_path)?;

        // Validate repo identifiers format
        for identifier in &repo_identifiers {
            if !identifier.contains('/') {
                return Err(anyhow::anyhow!(
                    "Invalid repository identifier '{}'. Expected format: owner/name",
                    identifier
                ));
            }
        }

        // Convert identifiers to lowercase for case-insensitive comparison
        let identifiers_lower: Vec<String> = repo_identifiers
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        // Filter repositories to only those specified
        let selected_count = data
            .repositories
            .iter()
            .filter(|r| identifiers_lower.contains(&r.metadata.full_name.to_lowercase()))
            .count();

        if selected_count == 0 {
            return Err(anyhow::anyhow!(
                "None of the specified repositories were found in canonical data"
            ));
        }

        info!(
            "Starting selective sync for {} repositories (out of {} requested)",
            selected_count,
            repo_identifiers.len()
        );
        println!(
            "🎯 Selective sync: {} repositories selected",
            selected_count
        );

        self.progress.start(selected_count);

        // Create GitHub adapter
        let github_adapter: GitHubAdapter = GitHubAdapter::new(&self.config).await?;

        let mut synced = 0;
        let mut cached = 0;
        let mut failed = 0;
        let mut failures: Vec<(String, String)> = Vec::new();

        // Process only selected repositories
        for repo in &mut data.repositories {
            // Skip if not in selection
            if !identifiers_lower.contains(&repo.metadata.full_name.to_lowercase()) {
                continue;
            }

            // Check cache (same logic as sync_all)
            if let Some(cache_entry) = self.cache.get(&repo.id) {
                if cache_entry.is_fresh(self.config.sync.cache_ttl_hours) {
                    cached += 1;
                    self.progress.increment_cached();
                    continue;
                }
            }

            // Verify primary platform is GitHub
            let should_sync = repo
                .platforms
                .iter()
                .any(|p| p.is_primary && matches!(p.platform, od_core::Platform::GitHub));

            if !should_sync {
                warn!(
                    "Repository {} is not primarily on GitHub, skipping",
                    repo.metadata.full_name
                );
                cached += 1;
                self.progress.increment_cached();
                continue;
            }

            // Extract owner/name
            let (owner, name) = match Self::parse_github_repo(repo) {
                Ok(parts) => parts,
                Err(e) => {
                    warn!("Failed to parse repo {}: {}", repo.id, e);
                    failed += 1;
                    failures.push((repo.id.clone(), e.to_string()));
                    self.progress.increment_failed();
                    continue;
                }
            };

            // Fetch from GitHub
            match github_adapter.fetch_repository(&format!("{}/{}", owner, name)).await {
                Ok(updated_repo) => {
                    self.merge_sync_data(repo, updated_repo);
                    self.cache.insert(&repo.id, &repo.metadata);
                    synced += 1;
                    self.progress
                        .increment_synced(&format!("{}/{}", owner, name));
                }
                Err(e) => {
                    error!("Failed to sync {}/{}: {}", owner, name, e);
                    failed += 1;
                    failures.push((repo.id.clone(), e.to_string()));
                    self.progress.increment_failed();
                }
            }
        }

        // Save updated data
        data.last_updated = chrono::Utc::now().to_rfc3339();
        data.to_yaml_file(canonical_path)?;

        // Save cache
        self.cache.save()?;

        let duration = start.elapsed();
        self.progress.finish();

        info!(
            "Selective sync complete: {} synced, {} cached, {} failed in {:?}",
            synced,
            cached,
            failed,
            duration
        );

        Ok(SyncResult {
            total: selected_count,
            synced,
            cached,
            failed,
            duration,
            failures,
            by_relation: HashMap::new(),
        })
    }

    /// Parse GitHub repo identifier from repository record
    fn parse_github_repo(repo: &Repository) -> Result<(String, String)> {
        // Try to extract from primary platform URL
        if let Some(url) = repo.primary_url() {
            if url.contains("github.com") {
                // Parse URL: https://github.com/owner/name
                let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
                if parts.len() >= 2 {
                    let owner = parts[parts.len() - 2].to_string();
                    let name = parts[parts.len() - 1].to_string();
                    return Ok((owner, name));
                }
            }
        }

        // Fallback: split full_name
        if repo.metadata.full_name.contains('/') {
            let parts: Vec<&str> = repo.metadata.full_name.split('/').collect();
            if parts.len() == 2 {
                return Ok((parts[0].to_string(), parts[1].to_string()));
            }
        }

        Err(anyhow::anyhow!(
            "Could not parse GitHub owner/name from repository"
        ))
    }

    /// Merge synced data into existing repository (preserve manual fields)
    fn merge_sync_data(&self, existing: &mut Repository, synced: Repository) {
        // Update fields that come from GitHub API
        existing.metadata.description = synced.metadata.description;
        existing.metadata.stars = synced.metadata.stars;
        existing.metadata.license = synced.metadata.license;
        existing.metadata.license_spdx = synced.metadata.license_spdx;
        existing.metadata.topics = synced.metadata.topics;
        existing.metadata.homepage = synced.metadata.homepage;

        // Update quality metrics from API
        existing.quality_metrics.archive_status = synced.quality_metrics.archive_status;
        existing.quality_metrics.last_commit_date = synced.quality_metrics.last_commit_date;
        existing.quality_metrics.last_star_update =
            chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Update fork parent info from API (GraphQL provides this)
        if synced.fork_parent.is_some() {
            existing.fork_parent = synced.fork_parent;
            existing.fork_parent_url = synced.fork_parent_url;
        }

        // Preserve manual curation fields:
        // - manually_curated flag
        // - curator_notes
        // - classification.significance_notes
        // - classification.readme_inclusion_reason (if manually set)

        debug!("Merged sync data for {}", existing.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::{
        Platform, PlatformInfo, PlatformStatus, QualityMetrics, RepositoryClassification,
        RepositoryMetadata, RepositorySource,
    };

    fn create_test_repo(full_name: &str) -> Repository {
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
                description: "Test repo".to_string(),
                primary_language: "Rust".to_string(),
                license: Some("MIT".to_string()),
                license_spdx: None,
                stars: 100,
                topics: vec![],
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
                last_commit_date: None,
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

    #[test]
    fn test_parse_github_repo_from_url() {
        let repo = create_test_repo("rust-lang/rust");
        let result = SyncOrchestrator::parse_github_repo(&repo);
        assert!(result.is_ok());
        let (owner, name) = result.unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(name, "rust");
    }

    #[test]
    fn test_parse_github_repo_from_full_name() {
        let mut repo = create_test_repo("kubernetes/kubernetes");
        // Clear URL to force fallback to full_name
        repo.platforms[0].url = "https://example.com".to_string();
        
        let result = SyncOrchestrator::parse_github_repo(&repo);
        assert!(result.is_ok());
        let (owner, name) = result.unwrap();
        assert_eq!(owner, "kubernetes");
        assert_eq!(name, "kubernetes");
    }

    #[test]
    fn test_merge_sync_data_preserves_manual_fields() {
        let config = OmnidatumConfig::default();
        let orchestrator = SyncOrchestrator::new(config).unwrap();

        let mut existing = create_test_repo("test/repo");
        existing.metadata.stars = 100;
        existing.metadata.description = "Old description".to_string();
        existing.manually_curated = true;
        existing.curator_notes = Some("Important notes".to_string());

        let mut synced = existing.clone();
        synced.metadata.stars = 200;
        synced.metadata.description = "New description".to_string();

        orchestrator.merge_sync_data(&mut existing, synced);

        // API fields should be updated
        assert_eq!(existing.metadata.stars, 200);
        assert_eq!(existing.metadata.description, "New description");

        // Manual fields should be preserved
        assert!(existing.manually_curated);
        assert_eq!(existing.curator_notes, Some("Important notes".to_string()));
    }

    fn make_canonical(repos: Vec<Repository>) -> CanonicalData {
        let mut data = CanonicalData::new();
        data.repositories = repos;
        data
    }

    #[test]
    fn test_quality_report_detects_star_drop() {
        let config = OmnidatumConfig::default();
        let orchestrator = SyncOrchestrator::new(config).unwrap();

        let mut pre = create_test_repo("owner/repo");
        pre.metadata.stars = 1000;
        let mut post = pre.clone();
        post.metadata.stars = 500; // 50% drop — above 25% threshold

        let report = orchestrator
            .generate_quality_report(&make_canonical(vec![pre]), &make_canonical(vec![post]), 1)
            .unwrap();

        assert!(
            report.anomalies.iter().any(|a| matches!(a.anomaly_type, AnomalyType::StarDrop)),
            "expected StarDrop anomaly"
        );
    }

    #[test]
    fn test_quality_report_detects_newly_archived() {
        let config = OmnidatumConfig::default();
        let orchestrator = SyncOrchestrator::new(config).unwrap();

        let pre = create_test_repo("owner/repo");
        let mut post = pre.clone();
        post.quality_metrics.archive_status = true;

        let report = orchestrator
            .generate_quality_report(&make_canonical(vec![pre]), &make_canonical(vec![post]), 1)
            .unwrap();

        assert!(
            report.anomalies.iter().any(|a| matches!(a.anomaly_type, AnomalyType::NewlyArchived)),
            "expected NewlyArchived anomaly"
        );
    }

    #[test]
    fn test_quality_report_completeness_metrics() {
        let config = OmnidatumConfig::default();
        let orchestrator = SyncOrchestrator::new(config).unwrap();

        let mut r1 = create_test_repo("a/b");
        r1.metadata.license = Some("MIT".to_string());
        r1.metadata.description = "desc".to_string();
        r1.metadata.topics = vec!["topic".to_string()];

        let mut r2 = create_test_repo("c/d");
        r2.metadata.license = None;
        r2.metadata.description = String::new();
        r2.metadata.topics = vec![];

        let pre = make_canonical(vec![r1.clone(), r2.clone()]);
        let post = make_canonical(vec![r1, r2]);

        let report = orchestrator.generate_quality_report(&pre, &post, 2).unwrap();

        // 1 of 2 repos has license/description/topics → 50%
        assert!((report.data_completeness.license_coverage - 50.0).abs() < 0.1);
        assert!((report.data_completeness.description_coverage - 50.0).abs() < 0.1);
        assert!((report.data_completeness.topics_coverage - 50.0).abs() < 0.1);
    }
}