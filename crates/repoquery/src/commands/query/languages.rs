use anyhow::Result;
use clap::Args;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Args)]
pub struct LanguagesArgs {
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Minimum repos with a language to include
    #[arg(long, default_value = "1")]
    pub min_repos: usize,
    /// Maximum number of languages to show
    #[arg(long)]
    pub limit: Option<usize>,
}

pub async fn run(args: LanguagesArgs) -> Result<()> {
    let store = rq_store::open_store(&args.store)?;
    let repos = store.list_repos(&Default::default())?;

    let mut lang_counts: BTreeMap<String, usize> = BTreeMap::new();
    for repo in &repos {
        let lang = if repo.metadata.primary_language.is_empty() {
            "Unknown"
        } else {
            &repo.metadata.primary_language
        };
        *lang_counts.entry(lang.to_string()).or_insert(0) += 1;
    }

    let mut counts: Vec<_> = lang_counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts.retain(|(_, c)| *c >= args.min_repos);

    if let Some(limit) = args.limit {
        counts.truncate(limit);
    }

    let total: usize = counts.iter().map(|(_, c)| c).sum();
    println!("{:<30} {:>8} {:>8}", "Language", "Repos", "%");
    println!("{}", "-".repeat(50));
    for (lang, count) in &counts {
        let pct = (*count as f64 / total as f64) * 100.0;
        println!("{:<30} {:>8} {:>7.1}%", lang, count, pct);
    }

    Ok(())
}
