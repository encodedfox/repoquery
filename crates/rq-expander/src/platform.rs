use crate::FgatPool;
use anyhow::Result;
use async_trait::async_trait;
use rq_core::PlatformKind;
use std::sync::Arc;

/// Minimal representation of a repository returned by platform API.
#[derive(Debug, Clone)]
pub struct PlatformRepo {
    pub name: String,
    pub owner: String,
    pub full_name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: u32,
    pub topics: Vec<String>,
    pub license: Option<String>,
    pub is_archived: bool,
    pub fork: bool,
    pub html_url: String,
}

#[async_trait]
pub trait PlatformApiClient: Send + Sync {
    fn platform(&self) -> PlatformKind;
    async fn fetch_followers(&self, username: &str) -> Result<Vec<String>>;
    async fn fetch_following(&self, username: &str) -> Result<Vec<String>>;
    async fn fetch_user_repos(&self, username: &str) -> Result<Vec<PlatformRepo>>;
}

pub struct GitHubClient {
    pool: Arc<FgatPool>,
}

impl GitHubClient {
    pub fn new(pool: Arc<FgatPool>) -> Self {
        Self { pool }
    }

    fn build_client(&self, token: &str) -> Result<octocrab::Octocrab> {
        Ok(octocrab::Octocrab::builder()
            .personal_token(token.to_string())
            .build()?)
    }
}

#[async_trait]
impl PlatformApiClient for GitHubClient {
    fn platform(&self) -> PlatformKind {
        PlatformKind::GitHub
    }

    async fn fetch_followers(&self, username: &str) -> Result<Vec<String>> {
        let token = self
            .pool
            .acquire(self.platform())
            .await
            .ok_or_else(|| anyhow::anyhow!("No available FGAT token for GitHub"))?;

        let client = self.build_client(&token.raw)?;
        let mut page: octocrab::Page<serde_json::Value> = client
            .get(format!("/users/{}/followers", username), None::<&()>)
            .await?;
        let mut followers = Vec::new();

        loop {
            for item in &page.items {
                if let Some(login) = item["login"].as_str() {
                    followers.push(login.to_string());
                }
            }
            if let Some(uri) = page.next.take() {
                page = client.get_page(&Some(uri)).await?.unwrap_or_default();
            } else {
                break;
            }
        }

        self.pool.release(token).await;
        Ok(followers)
    }

    async fn fetch_following(&self, username: &str) -> Result<Vec<String>> {
        let token = self
            .pool
            .acquire(self.platform())
            .await
            .ok_or_else(|| anyhow::anyhow!("No available FGAT token for GitHub"))?;

        let client = self.build_client(&token.raw)?;
        let mut page: octocrab::Page<serde_json::Value> = client
            .get(format!("/users/{}/following", username), None::<&()>)
            .await?;
        let mut following = Vec::new();

        loop {
            for item in &page.items {
                if let Some(login) = item["login"].as_str() {
                    following.push(login.to_string());
                }
            }
            if let Some(uri) = page.next.take() {
                page = client.get_page(&Some(uri)).await?.unwrap_or_default();
            } else {
                break;
            }
        }

        self.pool.release(token).await;
        Ok(following)
    }

    async fn fetch_user_repos(&self, username: &str) -> Result<Vec<PlatformRepo>> {
        let token = self
            .pool
            .acquire(self.platform())
            .await
            .ok_or_else(|| anyhow::anyhow!("No available FGAT token for GitHub"))?;

        let client = self.build_client(&token.raw)?;
        let mut page: octocrab::Page<serde_json::Value> = client
            .get(
                format!("/users/{}/repos?per_page=100&type=public", username),
                None::<&()>,
            )
            .await?;
        let mut repos = Vec::new();

        loop {
            for item in &page.items {
                let name = match item["name"].as_str() {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let owner = item["owner"]["login"].as_str().unwrap_or("").to_string();
                let full_name = item["full_name"].as_str().unwrap_or("").to_string();
                let description = item["description"].as_str().map(|s| s.to_string());
                let language = item["language"].as_str().map(|s| s.to_string());
                let stars = item["stargazers_count"].as_u64().unwrap_or(0) as u32;
                let topics: Vec<String> = item["topics"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let license = item["license"]["spdx_id"].as_str().map(|s| s.to_string());
                let is_archived = item["archived"].as_bool().unwrap_or(false);
                let fork = item["fork"].as_bool().unwrap_or(false);
                let html_url = item["html_url"].as_str().unwrap_or("").to_string();

                repos.push(PlatformRepo {
                    name,
                    owner,
                    full_name,
                    description,
                    language,
                    stars,
                    topics,
                    license,
                    is_archived,
                    fork,
                    html_url,
                });
            }
            if let Some(uri) = page.next.take() {
                page = client.get_page(&Some(uri)).await?.unwrap_or_default();
            } else {
                break;
            }
        }

        self.pool.release(token).await;
        Ok(repos)
    }
}
