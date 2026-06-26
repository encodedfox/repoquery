use anyhow::Result;
use clap::Args;
use rq_store::RepoFilter;
use std::path::PathBuf;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct ListArgs {
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Filter by language
    #[arg(long)]
    pub language: Option<String>,
    /// Filter by minimum stars
    #[arg(long)]
    pub min_stars: Option<u32>,
    /// Filter by maximum stars
    #[arg(long)]
    pub max_stars: Option<u32>,
    /// Filter by owner
    #[arg(long)]
    pub owner: Option<String>,
    /// Filter by license (SPDX identifier)
    #[arg(long)]
    pub license: Option<String>,
    /// Filter by topic
    #[arg(long)]
    pub topic: Option<String>,
    /// Filter by source (github/gitlab/codeberg)
    #[arg(long)]
    pub source: Option<String>,
    /// Sort field (stars, name, updated, quality)
    #[arg(long, default_value = "stars")]
    pub sort: String,
    /// Sort order (asc, desc)
    #[arg(long, default_value = "desc")]
    pub order: String,
    /// Maximum results
    #[arg(long)]
    pub limit: Option<usize>,
    /// Output format (table, json, md, csv)
    #[arg(long, default_value = "table")]
    pub format: String,
    /// Show only archived repositories
    #[arg(long)]
    pub archived: Option<bool>,
}

pub async fn run(args: ListArgs) -> Result<()> {
    let store = rq_store::open_store(&args.store)?;
    let fmt = OutputFormat::from_str(&args.format);

    let sort_field = match args.sort.to_lowercase().as_str() {
        "name" => rq_store::SortField::Name,
        "updated" => rq_store::SortField::Updated,
        "created" => rq_store::SortField::Created,
        "quality" => rq_store::SortField::Quality,
        _ => rq_store::SortField::Stars,
    };
    let sort_order = match args.order.to_lowercase().as_str() {
        "asc" => rq_store::SortOrder::Asc,
        _ => rq_store::SortOrder::Desc,
    };

    let filter = RepoFilter {
        language: args.language,
        min_stars: args.min_stars,
        max_stars: args.max_stars,
        owner: args.owner,
        license: args.license,
        topic: args.topic,
        source: args.source,
        archived: args.archived,
        sort: sort_field,
        order: sort_order,
        limit: args.limit,
        ..Default::default()
    };

    let repos = store.list_repos(&filter)?;
    println!("{}", fmt.formatter().format_list(&repos));
    Ok(())
}
