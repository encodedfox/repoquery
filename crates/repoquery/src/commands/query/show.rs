use anyhow::{Context, Result};
use clap::Args;
use rq_store::RepoFilter;
use std::path::PathBuf;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct ShowArgs {
    /// Repository full name (owner/name)
    pub repo: String,
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Output format (table, json, md)
    #[arg(long, default_value = "table")]
    pub format: String,
}

pub async fn run(args: ShowArgs) -> Result<()> {
    let store = rq_store::open_store(&args.store)?;
    let fmt = OutputFormat::from_str(&args.format);

    let filter = RepoFilter {
        search_query: Some(args.repo.clone()),
        limit: Some(1),
        ..Default::default()
    };

    let repos = store.list_repos(&filter)?;
    let repo = repos
        .into_iter()
        .find(|r| r.metadata.full_name == args.repo)
        .context("Repository not found")?;

    println!("{}", fmt.formatter().format_detail(&repo));
    Ok(())
}
