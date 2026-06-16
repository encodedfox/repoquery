//! GitHub GraphQL client for bulk repository fetching.

use anyhow::{Context, Result};
use od_core::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Relation, Repository,
    RepositoryClassification, RepositoryMetadata, RepositorySource,
};
use reqwest::Client;
use serde_json::{json, Value};

const GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";

const STARRED_QUERY: &str = r#"
query($cursor: String) {
  viewer {
    starredRepositories(first: 100, after: $cursor, orderBy: {field: STARRED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      edges {
        node {
          nameWithOwner name owner { login } description url
          stargazerCount primaryLanguage { name } licenseInfo { name spdxId }
          isArchived isFork parent { nameWithOwner url } pushedAt
          repositoryTopics(first: 20) { nodes { topic { name } } }
          homepageUrl
        }
      }
    }
  }
}"#;

const OWNED_QUERY: &str = r#"
query($cursor: String) {
  viewer {
    repositories(first: 100, after: $cursor, ownerAffiliations: OWNER, isFork: false) {
      pageInfo { hasNextPage endCursor }
      nodes {
        nameWithOwner name owner { login } description url
        stargazerCount primaryLanguage { name } licenseInfo { name spdxId }
        isArchived isFork parent { nameWithOwner url } pushedAt
        repositoryTopics(first: 20) { nodes { topic { name } } }
        homepageUrl
      }
    }
  }
}"#;

const FORKS_QUERY: &str = r#"
query($cursor: String) {
  viewer {
    repositories(first: 100, after: $cursor, ownerAffiliations: OWNER, isFork: true) {
      pageInfo { hasNextPage endCursor }
      nodes {
        nameWithOwner name owner { login } description url
        stargazerCount primaryLanguage { name } licenseInfo { name spdxId }
        isArchived isFork parent { nameWithOwner url } pushedAt
        repositoryTopics(first: 20) { nodes { topic { name } } }
        homepageUrl
      }
    }
  }
}"#;

const WATCHING_QUERY: &str = r#"
query($cursor: String) {
  viewer {
    watching(first: 100, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        nameWithOwner name owner { login } description url
        stargazerCount primaryLanguage { name } licenseInfo { name spdxId }
        isArchived isFork parent { nameWithOwner url } pushedAt
        repositoryTopics(first: 20) { nodes { topic { name } } }
        homepageUrl
      }
    }
  }
}"#;

/// Thin GitHub GraphQL client for bulk fetching.
pub struct GitHubGraphQL {
    client: Client,
    token: String,
}

impl GitHubGraphQL {
    pub fn new(token: String) -> Result<Self> {
        let client = Client::builder()
            .user_agent("omnidatum-processor/0.1")
            .build()
            .context("Failed to build reqwest client")?;
        Ok(Self { client, token })
    }

    /// Fetch one page of starred repos. Returns (repos, next_cursor).
    pub async fn fetch_starred_page(
        &self,
        cursor: Option<&str>,
    ) -> Result<(Vec<Repository>, Option<String>)> {
        let body = json!({
            "query": STARRED_QUERY,
            "variables": { "cursor": cursor }
        });
        let data = self.post(body).await?;
        let conn = &data["viewer"]["starredRepositories"];
        let has_next = conn["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false);
        let end_cursor = if has_next {
            conn["pageInfo"]["endCursor"].as_str().map(str::to_owned)
        } else {
            None
        };
        let repos = conn["edges"]
            .as_array()
            .map(|edges| {
                edges
                    .iter()
                    .filter_map(|e| {
                        self.convert_node(&e["node"], Relation::Starred).ok()
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok((repos, end_cursor))
    }

    /// Paginate through all starred repos.
    pub async fn fetch_all_starred(&self) -> Result<Vec<Repository>> {
        self.paginate_all(STARRED_QUERY, "starredRepositories", Relation::Starred, true)
            .await
    }

    /// Fetch all repos for a given relation (Owned, Forked, Watching).
    pub async fn fetch_all_by_relation(&self, relation: Relation) -> Result<Vec<Repository>> {
        let (query, field) = match relation {
            Relation::Owned => (OWNED_QUERY, "repositories"),
            Relation::Forked => (FORKS_QUERY, "repositories"),
            Relation::Watching => (WATCHING_QUERY, "watching"),
            other => anyhow::bail!("GraphQL fetch not supported for relation {:?}", other),
        };
        self.paginate_all(query, field, relation, false).await
    }

    /// Check how many commits a fork is ahead/behind its parent.
    /// Returns `(ahead_by, behind_by)`.
    pub async fn check_fork_status(
        &self,
        fork_full_name: &str,
        parent_full_name: &str,
    ) -> Result<(u32, u32)> {
        let url = format!(
            "https://api.github.com/repos/{}/compare/{}:HEAD...{}:HEAD",
            parent_full_name,
            parent_full_name.split('/').next().unwrap_or(""),
            fork_full_name.split('/').next().unwrap_or(""),
        );
        let resp: serde_json::Value = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "omnidatum")
            .send()
            .await?
            .json()
            .await?;
        let ahead = resp["ahead_by"].as_u64().unwrap_or(0) as u32;
        let behind = resp["behind_by"].as_u64().unwrap_or(0) as u32;
        Ok((ahead, behind))
    }

    // ── internals ─────────────────────────────────────────────────────────────

    async fn post(&self, body: Value) -> Result<Value> {
        let resp = self
            .client
            .post(GITHUB_GRAPHQL_URL)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .context("GraphQL request failed")?;

        let status = resp.status();
        let json: Value = resp.json().await.context("Failed to parse GraphQL response")?;

        if !status.is_success() {
            anyhow::bail!("GitHub GraphQL returned {}: {}", status, json);
        }
        if let Some(errors) = json.get("errors") {
            anyhow::bail!("GitHub GraphQL errors: {}", errors);
        }
        Ok(json["data"].clone())
    }

    /// Generic paginator for queries that use `nodes` (non-starred) or `edges` (starred).
    async fn paginate_all(
        &self,
        query: &str,
        field: &str,
        relation: Relation,
        use_edges: bool,
    ) -> Result<Vec<Repository>> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let body = json!({
                "query": query,
                "variables": { "cursor": cursor.as_deref() }
            });
            let data = self.post(body).await?;
            let conn = &data["viewer"][field];

            let nodes: Vec<&Value> = if use_edges {
                conn["edges"]
                    .as_array()
                    .map(|v| v.iter().map(|e| &e["node"]).collect())
                    .unwrap_or_default()
            } else {
                conn["nodes"]
                    .as_array()
                    .map(|v| v.iter().collect())
                    .unwrap_or_default()
            };

            for node in nodes {
                match self.convert_node(node, relation.clone()) {
                    Ok(r) => all.push(r),
                    Err(e) => tracing::warn!("Skipping node: {}", e),
                }
            }

            let has_next = conn["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false);
            if !has_next {
                break;
            }
            cursor = conn["pageInfo"]["endCursor"].as_str().map(str::to_owned);
        }

        Ok(all)
    }

    fn convert_node(&self, node: &Value, relation: Relation) -> Result<Repository> {
        let full_name = node["nameWithOwner"]
            .as_str()
            .context("missing nameWithOwner")?;
        let id = format!("github-com-{}", full_name.replace('/', "-"));

        let owner = node["owner"]["login"].as_str().unwrap_or("").to_owned();
        let name = node["name"].as_str().unwrap_or("").to_owned();
        let description = node["description"].as_str().unwrap_or("").to_owned();
        let url = node["url"].as_str().unwrap_or("").to_owned();
        let stars = node["stargazerCount"].as_u64().unwrap_or(0) as u32;
        let language = node["primaryLanguage"]["name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_owned();
        let license = node["licenseInfo"]["name"].as_str().map(str::to_owned);
        let license_spdx = node["licenseInfo"]["spdxId"].as_str().map(str::to_owned);
        let is_archived = node["isArchived"].as_bool().unwrap_or(false);
        let homepage = node["homepageUrl"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let last_commit = node["pushedAt"].as_str().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.format("%Y-%m-%d").to_string())
        });
        let topics: Vec<String> = node["repositoryTopics"]["nodes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| n["topic"]["name"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();

        let fork_parent = node.get("parent").and_then(|p| {
            p["nameWithOwner"].as_str().map(str::to_owned)
        });
        let fork_parent_url = node.get("parent").and_then(|p| {
            p["url"].as_str().map(str::to_owned)
        });

        Ok(Repository {
            id,
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: url.clone(),
                status: if is_archived {
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
                name,
                owner,
                full_name: full_name.to_owned(),
                description,
                primary_language: language.clone(),
                license,
                license_spdx,
                stars,
                topics,
                homepage,
                language_breakdown: None,
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
                archive_status: is_archived,
                archive_date: if is_archived {
                    Some(chrono::Utc::now().format("%Y-%m-%d").to_string())
                } else {
                    None
                },
                last_commit_date: last_commit,
                last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                quality_score: 70,
            },
            source: RepositorySource::GitHubStars,
            added_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            manually_curated: false,
            curator_notes: None,
            relations: vec![relation],
            fork_parent,
            fork_parent_url,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_node() -> Value {
        json!({
            "nameWithOwner": "rust-lang/rust",
            "name": "rust",
            "owner": { "login": "rust-lang" },
            "description": "Empowering everyone to build reliable software.",
            "url": "https://github.com/rust-lang/rust",
            "stargazerCount": 98000,
            "primaryLanguage": { "name": "Rust" },
            "licenseInfo": { "name": "MIT", "spdxId": "MIT" },
            "isArchived": false,
            "isFork": false,
            "parent": null,
            "pushedAt": "2024-12-01T10:00:00Z",
            "repositoryTopics": {
                "nodes": [
                    { "topic": { "name": "rust" } },
                    { "topic": { "name": "systems-programming" } }
                ]
            },
            "homepageUrl": "https://www.rust-lang.org"
        })
    }

    fn fork_node() -> Value {
        json!({
            "nameWithOwner": "myuser/rust",
            "name": "rust",
            "owner": { "login": "myuser" },
            "description": "My fork",
            "url": "https://github.com/myuser/rust",
            "stargazerCount": 0,
            "primaryLanguage": { "name": "Rust" },
            "licenseInfo": null,
            "isArchived": false,
            "isFork": true,
            "parent": { "nameWithOwner": "rust-lang/rust", "url": "https://github.com/rust-lang/rust" },
            "pushedAt": "2024-11-01T00:00:00Z",
            "repositoryTopics": { "nodes": [] },
            "homepageUrl": null
        })
    }

    fn make_graphql() -> GitHubGraphQL {
        GitHubGraphQL {
            client: Client::new(),
            token: "test-token".to_string(),
        }
    }

    #[test]
    fn test_convert_graphql_node() {
        let gql = make_graphql();
        let repo = gql.convert_node(&sample_node(), Relation::Starred).unwrap();

        assert_eq!(repo.id, "github-com-rust-lang-rust");
        assert_eq!(repo.metadata.full_name, "rust-lang/rust");
        assert_eq!(repo.metadata.owner, "rust-lang");
        assert_eq!(repo.metadata.stars, 98000);
        assert_eq!(repo.metadata.primary_language, "Rust");
        assert_eq!(repo.metadata.license.as_deref(), Some("MIT"));
        assert_eq!(repo.metadata.license_spdx.as_deref(), Some("MIT"));
        assert_eq!(repo.metadata.topics, vec!["rust", "systems-programming"]);
        assert_eq!(repo.metadata.homepage.as_deref(), Some("https://www.rust-lang.org"));
        assert_eq!(repo.quality_metrics.last_commit_date.as_deref(), Some("2024-12-01"));
        assert!(!repo.quality_metrics.archive_status);
        assert_eq!(repo.relations, vec![Relation::Starred]);
        assert!(repo.fork_parent.is_none());
        assert!(repo.fork_parent_url.is_none());
    }

    #[test]
    fn test_convert_fork_node_captures_parent() {
        let gql = make_graphql();
        let repo = gql.convert_node(&fork_node(), Relation::Forked).unwrap();

        assert_eq!(repo.fork_parent.as_deref(), Some("rust-lang/rust"));
        assert_eq!(
            repo.fork_parent_url.as_deref(),
            Some("https://github.com/rust-lang/rust")
        );
        assert_eq!(repo.relations, vec![Relation::Forked]);
    }

    #[test]
    fn test_fork_parent_fields_backward_compat() {
        // Deserialize Repository YAML without fork_parent fields — must succeed.
        let yaml = r#"
id: github-com-owner-repo
platforms: []
metadata:
  name: repo
  owner: owner
  full_name: owner/repo
  description: Test
  primary_language: Rust
  stars: 10
  language_category: Rust
classification:
  language_category: Rust
quality_metrics:
  archive_status: false
  last_star_update: "2024-01-01"
  quality_score: 50
source: git_hub_stars
manually_curated: false
"#;
        let repo: Repository = serde_yml::from_str(yaml).expect("backward compat deserialization failed");
        assert!(repo.fork_parent.is_none());
        assert!(repo.fork_parent_url.is_none());
    }
}
