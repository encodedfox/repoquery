use anyhow::Result;
use od_core::{CanonicalData, OmnidatumConfig, Platform, Relation};
use od_sync::{GitHubAdapter, SyncOrchestrator};
use std::path::PathBuf;

fn parse_relations(s: &str) -> Result<Vec<Relation>> {
    s.split(',')
        .map(|r| match r.trim() {
            "starred" => Ok(Relation::Starred),
            "owned" => Ok(Relation::Owned),
            "forked" => Ok(Relation::Forked),
            "watching" => Ok(Relation::Watching),
            other => Err(anyhow::anyhow!(
                "Unknown relation '{}'. Valid: starred,owned,forked,watching",
                other
            )),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    repos: Option<String>,
    force: bool,
    dry_run: bool,
    verbose: bool,
    clear_cache: bool,
    input: PathBuf,
    relations: Option<String>,
    check_forks: bool,
) -> Result<()> {
    println!("🔄 Sync Repository Metadata");

    if verbose {
        tracing::debug!("Verbose mode enabled");
    }

    let mut config = OmnidatumConfig::load()?;

    if force {
        config.sync.cache_ttl_hours = 0;
    }

    let cache_ttl = config.sync.cache_ttl_hours;

    let mut orchestrator = SyncOrchestrator::new(config)?;

    if clear_cache {
        println!("🗑️  Clearing cache...");
        orchestrator.cache_mut().clear()?;
        println!("  ✅ Cache cleared");
    }

    if dry_run {
        println!("\n🔍 Dry Run Mode - No changes will be made");
        let data = CanonicalData::from_yaml_file(&input)?;

        let mut would_sync = 0;
        let mut would_cache = 0;
        let mut would_skip = 0;
        let mut sample_repos = Vec::new();

        let cache = orchestrator.cache_mut();
        let ttl = cache_ttl;

        for repo in &data.repositories {
            let is_github = repo
                .platforms
                .iter()
                .any(|p| p.is_primary && matches!(p.platform, Platform::GitHub));

            if !is_github {
                would_skip += 1;
                continue;
            }

            if let Some(cache_entry) = cache.get(&repo.id) {
                if cache_entry.is_fresh(ttl) {
                    would_cache += 1;
                    continue;
                }
            }

            would_sync += 1;
            if sample_repos.len() < 5 {
                sample_repos.push(repo.metadata.full_name.clone());
            }
        }

        println!("\n📊 Sync Plan:");
        println!("  Total repositories: {}", data.repositories.len());
        println!("  Would sync: {} repos", would_sync);
        println!("  Would use cache: {} repos", would_cache);
        println!("  Would skip (non-GitHub): {} repos", would_skip);

        if let Some(repo_list) = repos {
            let requested: Vec<_> = repo_list.split(',').map(|s| s.trim()).collect();
            println!("\n🎯 Selective Sync:");
            println!("  Requested: {} repos", requested.len());

            let mut found = 0;
            let mut not_found = Vec::new();
            for req in &requested {
                let exists = data
                    .repositories
                    .iter()
                    .any(|r| r.metadata.full_name.eq_ignore_ascii_case(req));
                if exists {
                    found += 1;
                } else {
                    not_found.push(*req);
                }
            }
            println!("  Found: {}", found);
            if !not_found.is_empty() {
                println!("  Not found: {}", not_found.len());
                for repo in not_found.iter().take(3) {
                    println!("    ⚠️  {}", repo);
                }
            }

            println!("\n  Sample repositories:");
            for repo in requested.iter().take(5) {
                println!("    - {}", repo);
            }
        } else if !sample_repos.is_empty() {
            println!("\n  Sample repositories to sync:");
            for repo in sample_repos {
                println!("    - {}", repo);
            }
        }

        println!("\n🔍 Checking GitHub API rate limits...");
        let check_config = OmnidatumConfig::load()?;
        match GitHubAdapter::new(&check_config).await {
            Ok(adapter) => match adapter.check_rate_limit().await {
                Ok(()) => println!("  ✅ Rate limit OK"),
                Err(e) => println!("  ⚠️  Rate limit: {}", e),
            },
            Err(e) => println!("  ⚠️  Could not check rate limit: {}", e),
        }

        println!("\n✅ Dry run complete - no files were modified");
        return Ok(());
    }

    println!("\n🚀 Starting sync...");
    let result = if let Some(repo_list) = repos {
        let repos: Vec<String> = repo_list
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        orchestrator.sync_specific(repos, &input).await?
    } else if let Some(rel_str) = relations {
        let rels = parse_relations(&rel_str)?;
        orchestrator.sync_by_relation(&rels, &input).await?
    } else {
        orchestrator.sync_all(&input).await?
    };

    println!("\n📊 Sync Results:");
    println!("  Total: {}", result.total);
    println!("  ✅ Synced: {}", result.synced);
    println!("  ⚠️  Cached: {}", result.cached);
    println!("  ❌ Failed: {}", result.failed);
    println!("  ⏱️  Duration: {:?}", result.duration);

    if !result.failures.is_empty() {
        println!("\n❌ Failures:");
        for (repo_id, error) in result.failures.iter().take(10) {
            println!("  - {}: {}", repo_id, error);
        }
        if result.failures.len() > 10 {
            println!("  ... and {} more", result.failures.len() - 10);
        }
    }

    if result.failed == 0 {
        println!("\n✅ Sync completed successfully!");
    } else {
        println!("\n⚠️  Sync completed with {} failures", result.failed);
    }

    if check_forks {
        println!("\n🔀 Checking fork status...");
        let config = OmnidatumConfig::load()?;
        let adapter = GitHubAdapter::new(&config).await?;
        let graphql = adapter.graphql();
        let data = CanonicalData::from_yaml_file(&input)?;
        let forks: Vec<_> = data
            .repositories
            .iter()
            .filter(|r| r.fork_parent.is_some())
            .collect();
        println!("  Found {} forked repos", forks.len());
        let mut updated = 0usize;
        for repo in &forks {
            if let Some(parent) = &repo.fork_parent {
                match graphql.check_fork_status(&repo.metadata.full_name, parent).await {
                    Ok((ahead, behind)) => {
                        println!("  {} → ahead:{} behind:{}", repo.metadata.full_name, ahead, behind);
                        updated += 1;
                        let _ = (ahead, behind); // stored in-memory only; persist via store if needed
                    }
                    Err(e) => println!("  ⚠️  {}: {}", repo.metadata.full_name, e),
                }
            }
        }
        println!("  ✅ Checked {} forks", updated);
    }

    Ok(())
}
