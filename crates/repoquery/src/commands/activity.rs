use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Args, Subcommand};
use rq_core::{self, classify_all, summarize, ActivityStatus, Repository};
use rq_store::RepoFilter;

#[derive(Subcommand)]
pub enum ActivityAction {
    /// Show activity breakdown for all repositories
    Overview(OverviewArgs),
    /// List stale repositories (default: >12 months since last commit)
    Stale(StaleArgs),
    /// List active repositories (default: <3 months since last commit)
    Active(ActiveArgs),
    /// List repositories with fastest star growth
    Trending(TrendingArgs),
}

#[derive(Args)]
pub struct OverviewArgs {
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Active threshold in months
    #[arg(long, default_value = "3")]
    pub active_months: u64,
    /// Stale threshold in months
    #[arg(long, default_value = "12")]
    pub stale_months: u64,
    /// Show histogram chart
    #[arg(long)]
    pub chart: bool,
    /// Output format (table, json, md, csv)
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args)]
pub struct StaleArgs {
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Stale threshold in months
    #[arg(long, default_value = "12")]
    pub stale_threshold: u64,
    /// Output format (table, json, md, csv)
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args)]
pub struct ActiveArgs {
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Active threshold in months
    #[arg(long, default_value = "3")]
    pub active_threshold: u64,
    /// Output format (table, json, md, csv)
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args)]
pub struct TrendingArgs {
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Number of days to consider
    #[arg(long, default_value = "90")]
    pub since: u64,
    /// Maximum results
    #[arg(long, default_value = "20")]
    pub limit: usize,
}

fn load_repos(store_path: &Path) -> Result<Vec<Repository>> {
    let store = rq_store::open_store(store_path)?;
    store.list_repos(&RepoFilter::default())
}

pub async fn run_overview(args: OverviewArgs) -> Result<()> {
    let repos = load_repos(&args.store)?;
    let results = classify_all(&repos, args.active_months, args.stale_months);
    let summary = summarize(&results);

    use crate::output::OutputFormat;

    if args.chart {
        let data = [
            ("Active", summary.active as u64),
            ("Maintained", summary.maintained as u64),
            ("Stale", summary.stale as u64),
            ("Abandoned", summary.abandoned as u64),
            ("Unknown", summary.unknown as u64),
        ];
        println!(
            "{}",
            crate::output::chart::bar_chart(&data, "Activity Distribution")
        );
    }

    let fmt = OutputFormat::from_str(&args.format);
    let display_repos: Vec<rq_core::Repository> = results
        .into_iter()
        .map(|r| {
            let repo = repos
                .iter()
                .find(|x| x.metadata.full_name == r.full_name)
                .cloned()
                .unwrap_or_else(|| rq_core::Repository {
                    id: r.full_name.clone(),
                    platforms: vec![],
                    metadata: rq_core::RepositoryMetadata {
                        name: r.repo_name,
                        owner: r.owner,
                        full_name: r.full_name,
                        description: String::new(),
                        primary_language: r.language,
                        license: None,
                        license_spdx: None,
                        stars: r.stars,
                        topics: vec![],
                        homepage: None,
                        language_breakdown: None,
                        secondary_languages: vec![],
                    },
                    classification: rq_core::RepositoryClassification {
                        categories: vec![],
                        readme_sections: vec![],
                        web_reference_topics: vec![],
                        language_category: String::new(),
                        language_notes: None,
                        readme_inclusion: false,
                        readme_inclusion_reason: None,
                        significance_notes: None,
                    },
                    quality_metrics: rq_core::QualityMetrics {
                        archive_status: false,
                        archive_date: None,
                        last_commit_date: None,
                        last_star_update: String::new(),
                        quality_score: 0,
                    },
                    source: rq_core::RepositorySource::GitHubStars,
                    added_date: None,
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
                    discovered_via: None,
                });
            // Embellish repository with activity status so the formatter can see it
            rq_core::Repository {
                classification: rq_core::RepositoryClassification {
                    language_category: format!("{}", r.status.label()),
                    ..repo.classification
                },
                ..repo
            }
        })
        .collect();

    println!("{}", fmt.formatter().format_list(&display_repos));
    println!(
        "\nSummary: {} total | {} active | {} maintained | {} stale | {} abandoned | {} unknown",
        summary.total,
        summary.active,
        summary.maintained,
        summary.stale,
        summary.abandoned,
        summary.unknown
    );
    Ok(())
}

pub async fn run_stale(args: StaleArgs) -> Result<()> {
    let repos = load_repos(&args.store)?;
    let results = classify_all(&repos, 3, args.stale_threshold);
    let stale: Vec<_> = results
        .into_iter()
        .filter(|r| matches!(r.status, ActivityStatus::Stale | ActivityStatus::Abandoned))
        .collect();
    println!(
        "Found {} stale/abandoned repositories (threshold: {}mo)",
        stale.len(),
        args.stale_threshold
    );
    for r in &stale {
        println!("  {} (last activity: {})", r.full_name, r.last_activity);
    }
    Ok(())
}

pub async fn run_active(args: ActiveArgs) -> Result<()> {
    let repos = load_repos(&args.store)?;
    let results = classify_all(&repos, args.active_threshold, 12);
    let active: Vec<_> = results
        .into_iter()
        .filter(|r| {
            matches!(
                r.status,
                ActivityStatus::Active | ActivityStatus::Maintained
            )
        })
        .collect();
    println!(
        "Found {} active/maintained repositories (threshold: {}mo)",
        active.len(),
        args.active_threshold
    );
    for r in &active {
        println!("  {}", r.full_name);
    }
    Ok(())
}

pub async fn run_trending(args: TrendingArgs) -> Result<()> {
    let repos = load_repos(&args.store)?;
    let mut scored: Vec<_> = repos.iter().map(|r| (rq_core::trend_score(r), r)).collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(args.limit);

    println!("{:<40} {:>12} {:>12}", "Repository", "Stars", "Score");
    println!("{}", "-".repeat(70));
    for (score, repo) in &scored {
        println!(
            "{:<40} {:>12} {:>11.2}/day",
            repo.metadata.full_name, repo.metadata.stars, score
        );
    }
    Ok(())
}
