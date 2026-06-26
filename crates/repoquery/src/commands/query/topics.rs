use anyhow::Result;
use clap::Args;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Args)]
pub struct TopicsArgs {
    /// Path to data store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Minimum repos with a topic to include
    #[arg(long, default_value = "1")]
    pub min_repos: usize,
    /// Maximum number of topics to show
    #[arg(long)]
    pub limit: Option<usize>,
}

pub async fn run(args: TopicsArgs) -> Result<()> {
    let store = rq_store::open_store(&args.store)?;
    let repos = store.list_repos(&Default::default())?;

    let mut topic_counts: BTreeMap<String, usize> = BTreeMap::new();
    for repo in &repos {
        for topic in &repo.metadata.topics {
            *topic_counts.entry(topic.clone()).or_insert(0) += 1;
        }
    }

    let mut counts: Vec<_> = topic_counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts.retain(|(_, c)| *c >= args.min_repos);

    if let Some(limit) = args.limit {
        counts.truncate(limit);
    }

    println!("{:<30} {}", "Topic", "Repos");
    println!("{}", "-".repeat(40));
    for (topic, count) in &counts {
        println!("{:<30} {}", topic, count);
    }

    Ok(())
}
