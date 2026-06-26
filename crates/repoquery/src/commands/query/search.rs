use anyhow::Result;
use clap::Args;
use rq_store::RepoFilter;
use std::path::PathBuf;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Maximum results
    #[arg(long)]
    pub limit: Option<usize>,
    /// Sort field (stars, name, updated, quality)
    #[arg(long, default_value = "stars")]
    pub sort: String,
    /// Output format (table, json, md, csv)
    #[arg(long, default_value = "table")]
    pub format: String,
}

pub async fn run(args: SearchArgs) -> Result<()> {
    let store = rq_store::open_store(&args.store)?;
    let fmt = OutputFormat::from_str(&args.format);

    let sort_field = match args.sort.to_lowercase().as_str() {
        "name" => rq_store::SortField::Name,
        "updated" => rq_store::SortField::Updated,
        "created" => rq_store::SortField::Created,
        "quality" => rq_store::SortField::Quality,
        _ => rq_store::SortField::Stars,
    };

    let filter = RepoFilter {
        search_query: Some(args.query),
        sort: sort_field,
        limit: args.limit,
        ..Default::default()
    };

    let repos = store.list_repos(&filter)?;
    println!("{}", fmt.formatter().format_list(&repos));
    Ok(())
}
