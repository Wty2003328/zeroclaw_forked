use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::time::Duration;

use super::{parse_interval, Collector};
use crate::pulse::config::GitHubConfig;
use crate::pulse::models::RawItem;

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
    author: Option<GitHubAuthor>,
}

#[derive(Debug, Deserialize)]
struct GitHubAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRepo {
    full_name: String,
    description: Option<String>,
    html_url: String,
    stargazers_count: u64,
    language: Option<String>,
    forks_count: u64,
}

pub struct GitHubCollector {
    config: GitHubConfig,
    client: reqwest::Client,
}

impl GitHubCollector {
    pub fn new(config: GitHubConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Pulse/0.1.0")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Fetch latest release for a watched repo.
    async fn fetch_repo_releases(&self, repo: &str) -> Result<Vec<RawItem>> {
        tracing::debug!("Fetching releases for {}", repo);

        let url = format!("https://api.github.com/repos/{}/releases?per_page=3", repo);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("GitHub API returned {} for {}", response.status(), repo);
        }

        let releases: Vec<GitHubRelease> = response.json().await?;

        let items: Vec<RawItem> = releases
            .into_iter()
            .map(|release| {
                let title = format!(
                    "{} — {}",
                    repo,
                    release.name.as_deref().unwrap_or(&release.tag_name)
                );

                let published_at = release
                    .published_at
                    .as_ref()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc));

                let metadata = serde_json::json!({
                    "repo": repo,
                    "tag": release.tag_name,
                    "author": release.author.map(|a| a.login),
                    "type": "release",
                });

                RawItem {
                    source: format!("github:{}", repo),
                    collector_id: "github".to_string(),
                    title,
                    url: Some(release.html_url),
                    content: release.body,
                    metadata,
                    published_at,
                }
            })
            .collect();

        Ok(items)
    }

    /// Fetch trending repos for configured languages.
    async fn fetch_trending(&self) -> Result<Vec<RawItem>> {
        let mut all_items = Vec::new();

        for lang in &self.config.trending_languages {
            tracing::debug!("Fetching GitHub trending for {}", lang);

            // Use GitHub search API to find recently created popular repos
            let url = format!(
                "https://api.github.com/search/repositories?q=language:{}&sort=stars&order=desc&per_page=10",
                lang
            );

            match self.client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    #[derive(Deserialize)]
                    struct SearchResult {
                        items: Vec<GitHubRepo>,
                    }

                    if let Ok(result) = response.json::<SearchResult>().await {
                        for repo in result.items {
                            let title = format!(
                                "{} — {} ({} stars)",
                                repo.full_name,
                                repo.description.as_deref().unwrap_or("No description"),
                                repo.stargazers_count
                            );

                            let metadata = serde_json::json!({
                                "repo": repo.full_name,
                                "stars": repo.stargazers_count,
                                "forks": repo.forks_count,
                                "language": repo.language,
                                "type": "trending",
                            });

                            all_items.push(RawItem {
                                source: format!("github:trending/{}", lang),
                                collector_id: "github".to_string(),
                                title,
                                url: Some(repo.html_url),
                                content: repo.description,
                                metadata,
                                published_at: Some(Utc::now()),
                            });
                        }
                    }
                }
                Ok(response) => {
                    tracing::warn!(
                        "GitHub trending API returned {} for {}",
                        response.status(),
                        lang
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch GitHub trending for {}: {}", lang, e);
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(all_items)
    }
}

#[async_trait]
impl Collector for GitHubCollector {
    fn id(&self) -> &str {
        "github"
    }

    fn name(&self) -> &str {
        "GitHub"
    }

    fn default_interval(&self) -> Duration {
        parse_interval(&self.config.interval)
    }

    fn enabled(&self) -> bool {
        self.config.enabled
    }

    async fn collect(&self) -> Result<Vec<RawItem>> {
        let mut all_items = Vec::new();

        // Fetch releases for watched repos
        for repo in &self.config.watch_repos {
            match self.fetch_repo_releases(repo).await {
                Ok(items) => all_items.extend(items),
                Err(e) => tracing::warn!("Failed to fetch releases for {}: {}", repo, e),
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }

        // Fetch trending repos
        if !self.config.trending_languages.is_empty() {
            match self.fetch_trending().await {
                Ok(items) => all_items.extend(items),
                Err(e) => tracing::warn!("Failed to fetch trending repos: {}", e),
            }
        }

        tracing::info!("Fetched {} items from GitHub", all_items.len());
        Ok(all_items)
    }
}
