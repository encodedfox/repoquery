//! GitHub API adapter using octocrab

use super::{DataSourceAdapter, GitHubGraphQL};
use od_core::config::OmnidatumConfig;
use od_core::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Relation, Repository,
    RepositoryClassification, RepositoryMetadata, RepositorySource,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use octocrab::Octocrab;

/// GitHub API adapter
pub struct GitHubAdapter {
    client: Octocrab,
    graphql: GitHubGraphQL,
    rate_limit_buffer: u16,
}

impl GitHubAdapter {
    /// Create new GitHub adapter with authentication
    pub async fn new(config: &OmnidatumConfig) -> Result<Self> {
        use od_core::config::CredentialManager;

        let creds = CredentialManager::new(config.credentials.source.clone());
        let token = creds.get_github_token().context(
            "Failed to load GitHub credentials. Run 'cargo run -- configure' to set up.",
        )?;

        // Redact token for logging
        tracing::info!(
            "Authenticating with GitHub token: {}",
            CredentialManager::redact(&token)
        );

        let client = Octocrab::builder().personal_token(token.clone()).build()?;

        // Verify authentication
        let _user = client.current().user().await.context(
            "Failed to authenticate with GitHub. Check your token permissions.",
        )?;

        tracing::info!("Successfully authenticated with GitHub API");

        let graphql = GitHubGraphQL::new(token)?;

        Ok(Self {
            client,
            graphql,
            rate_limit_buffer: config.sync.rate_limit_buffer,
        })
    }

    /// Access the underlying GraphQL client (for fork status checks etc.)
    pub fn graphql(&self) -> &GitHubGraphQL {
        &self.graphql
    }

    /// Fetch repository metadata from GitHub
    async fn fetch_repo_by_parts(&self, owner: &str, name: &str) -> Result<Repository> {
        tracing::debug!("Fetching {}/{} from GitHub API", owner, name);

        let result = self
            .client
            .repos(owner, name)
            .get()
            .await;

        match result {
            Ok(repo) => {
                // Convert octocrab repo to our Repository model
                let repository = self.convert_repo_to_model(repo)?;
                Ok(repository)
            }
            Err(octocrab::Error::GitHub { source, .. }) if source.message.contains("Not Found") => {
                // 404 - Repository not found, mark as deprecated instead of failing
                tracing::warn!(
                    "Repository {}/{} not found (404), marking as deprecated",
                    owner, name
                );

                // Create deprecated repository entry
                let full_name = format!("{}/{}", owner, name);
                let id = format!("github-com-{}", full_name.replace('/', "-"));

                Ok(Repository {
                    id,
                    platforms: vec![PlatformInfo {
                        platform: Platform::GitHub,
                        url: format!("https://github.com/{}", full_name),
                        status: PlatformStatus::Deprecated,
                        is_primary: true,
                        migration_date: None,
                        last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                        notes: None,
                    }],
                    metadata: RepositoryMetadata {
                        name: name.to_string(),
                        owner: owner.to_string(),
                        full_name: full_name.clone(),
                        description: "Repository not found - may have been deleted or made private".to_string(),
                        primary_language: "Unknown".to_string(),
                        license: None,
                        license_spdx: None,
                        stars: 0,
                        topics: vec![],
                        homepage: None,
                        language_breakdown: None,
                        secondary_languages: vec![],
                    },
                    classification: RepositoryClassification {
                        categories: vec![],
                        readme_sections: vec![],
                        web_reference_topics: vec![],
                        language_category: "Unknown".to_string(),
                        language_notes: None,
                        readme_inclusion: false,
                        readme_inclusion_reason: None,
                        significance_notes: None,
                    },
                    quality_metrics: QualityMetrics {
                        archive_status: true,
                        archive_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                        last_commit_date: None,
                        last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                        quality_score: 0,
                    },
                    source: RepositorySource::GitHubStars,
                    added_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    manually_curated: false,
                    curator_notes: Some("Repository not found - may have been deleted or made private".to_string()),
                    relations: vec![],
                    fork_parent: None,
                    fork_parent_url: None,
                    custom_tags: vec![],
                    fork_ahead: None,
                    fork_behind: None,
                })
            }
            Err(e) => {
                Err(anyhow::anyhow!("Failed to fetch repository {}/{}: {}", owner, name, e))
            }
        }
    }

    /// Fetch all repos owned by the authenticated user (via GraphQL)
    pub async fn fetch_user_repos(&self) -> Result<Vec<Repository>> {
        self.graphql.fetch_all_by_relation(Relation::Owned).await
    }

    /// Fetch all forks owned by the authenticated user (via GraphQL)
    pub async fn fetch_user_forks(&self) -> Result<Vec<Repository>> {
        self.graphql.fetch_all_by_relation(Relation::Forked).await
    }

    /// Fetch repos watched by the authenticated user (via GraphQL)
    pub async fn fetch_watched_repos(&self) -> Result<Vec<Repository>> {
        self.graphql.fetch_all_by_relation(Relation::Watching).await
    }

    /// Bulk-fetch all starred repos via GraphQL (replaces per-repo REST calls)
    pub async fn fetch_starred_graphql(&self) -> Result<Vec<Repository>> {
        self.graphql.fetch_all_starred().await
    }

    /// Fetch repos belonging to an organisation the user is a member of
    pub async fn fetch_org_repos(&self, org: &str) -> Result<Vec<Repository>> {
        let page = self.client.orgs(org).list_repos().send().await?;
        self.collect_pages(page, Relation::OrgMember).await
    }

    /// Drain a paginated response into a Vec<Repository>, tagging each with `relation`
    async fn collect_pages(
        &self,
        mut page: octocrab::Page<octocrab::models::Repository>,
        relation: Relation,
    ) -> Result<Vec<Repository>> {
        let mut repos = Vec::new();
        loop {
            for raw in &page {
                let mut repo = self.convert_repo_to_model(raw.clone())?;
                repo.relations = vec![relation.clone()];
                repos.push(repo);
            }
            match self.client.get_page::<octocrab::models::Repository>(&page.next).await? {
                Some(next) => page = next,
                None => break,
            }
        }
        Ok(repos)
    }

    /// Check rate limit status
    pub async fn check_rate_limit(&self) -> Result<()> {
        let rate_limit = self.client.ratelimit().get().await?;

        let remaining = rate_limit.resources.core.remaining;
        let limit = rate_limit.resources.core.limit;
        let reset_time = rate_limit.resources.core.reset;

        tracing::debug!("GitHub API rate limit: {}/{} remaining", remaining, limit);

        if remaining < self.rate_limit_buffer as usize {
            let now = chrono::Utc::now();
            let reset =
                chrono::DateTime::from_timestamp(reset_time as i64, 0).unwrap_or(now);
            let wait_duration = reset - now;

            tracing::warn!(
                "Rate limit approaching: {}/{} remaining. Reset in {} seconds",
                remaining,
                limit,
                wait_duration.num_seconds()
            );

            if remaining < 10 {
                return Err(anyhow::anyhow!(
                    "Rate limit exhausted. Please wait {} seconds",
                    wait_duration.num_seconds()
                ));
            }
        }

        Ok(())
    }

    /// Convert octocrab repo to our Repository model
    fn convert_repo_to_model(&self, repo: octocrab::models::Repository) -> Result<Repository> {
        let full_name = repo.full_name.clone().unwrap_or_else(|| {
            format!(
                "{}/{}",
                repo.owner
                    .as_ref()
                    .map(|o| o.login.as_str())
                    .unwrap_or(""),
                repo.name
            )
        });

        let id = format!("github-com-{}", full_name.replace('/', "-"));

        let language = repo
            .language
            .as_ref()
            .map(|l| l.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(Repository {
            id,
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: repo
                    .html_url
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| format!("https://github.com/{}", full_name)),
                status: if repo.archived.unwrap_or(false) {
                    PlatformStatus::Archived
                } else {
                    PlatformStatus::Active
                },
                is_primary: true,
                migration_date: None,
                last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: repo.name.clone(),
                owner: repo
                    .owner
                    .as_ref()
                    .map(|o| o.login.clone())
                    .unwrap_or_default(),
                full_name: full_name.clone(),
                description: repo.description.clone().unwrap_or_default(),
                primary_language: language.clone(),
                license: repo.license.as_ref().map(|l| l.name.clone()),
                license_spdx: repo.license.as_ref().map(|l| l.spdx_id.clone()),
                stars: repo.stargazers_count.unwrap_or(0),
                topics: repo.topics.clone().unwrap_or_default(),
                homepage: repo.homepage.clone(),
                language_breakdown: None, // Would require separate API call
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: language,
                language_notes: None,
                readme_inclusion: false,
                readme_inclusion_reason: None,
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: repo.archived.unwrap_or(false),
                archive_date: if repo.archived.unwrap_or(false) {
                    Some(chrono::Utc::now().format("%Y-%m-%d").to_string())
                } else {
                    None
                },
                last_commit_date: repo
                    .pushed_at
                    .map(|dt| dt.format("%Y-%m-%d").to_string()),
                last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                quality_score: 70, // Will be recalculated by DataMerger
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
        })
    }
}

#[async_trait]
impl DataSourceAdapter for GitHubAdapter {
    async fn fetch_repository(&self, identifier: &str) -> Result<Repository> {
        let parts: Vec<&str> = identifier.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid GitHub repository identifier: {}",
                identifier
            ));
        }

        self.fetch_repo_by_parts(parts[0], parts[1]).await
    }

    async fn check_connection(&self) -> Result<()> {
        self.check_rate_limit().await
    }

    fn source_name(&self) -> &str {
        "github"
    }
}