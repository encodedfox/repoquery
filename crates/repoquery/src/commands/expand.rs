use anyhow::{Context, Result};
use clap::Subcommand;
use rq_core::{FgatToken, PlatformKind, Seed, SeedStatus, TokenStatus};
use rq_expander::{
    BfsTraverser, FgatPool, GitHubClient, PlatformApiClient, RepoCollector, TrustConfig,
    TrustScorer,
};
use rq_store::{GraphStore, SqliteStore};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Subcommand)]
pub enum SeedAction {
    /// Add a new seed user for graph expansion
    Add {
        /// Platform (github, gitlab, etc.)
        #[arg(long, default_value = "github")]
        platform: String,
        /// Username on the platform
        #[arg(long)]
        username: String,
        /// Maximum traversal depth
        #[arg(long, default_value = "2")]
        max_depth: u32,
        /// Path to SQLite store
        #[arg(long, default_value = "data/repoquery.db")]
        store: PathBuf,
    },
    /// List all seeds
    List {
        /// Path to SQLite store
        #[arg(long, default_value = "data/repoquery.db")]
        store: PathBuf,
    },
    /// Show seed details
    Show {
        /// Seed ID
        #[arg(long)]
        id: String,
        /// Path to SQLite store
        #[arg(long, default_value = "data/repoquery.db")]
        store: PathBuf,
    },
    /// Delete a seed
    Delete {
        /// Seed ID
        #[arg(long)]
        id: String,
        /// Path to SQLite store
        #[arg(long, default_value = "data/repoquery.db")]
        store: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum TokenAction {
    /// Add an FGAT token for API access
    Add {
        /// Platform (github, gitlab, etc.)
        #[arg(long, default_value = "github")]
        platform: String,
        /// The raw token value (will be stored as-is; hash only for display)
        #[arg(long)]
        token: String,
        /// Rate limit (requests per hour)
        #[arg(long)]
        rate_limit: Option<u32>,
        /// Path to SQLite store
        #[arg(long, default_value = "data/repoquery.db")]
        store: PathBuf,
    },
    /// List all registered tokens
    List {
        /// Path to SQLite store
        #[arg(long, default_value = "data/repoquery.db")]
        store: PathBuf,
    },
    /// Remove a token
    Remove {
        /// Token ID
        #[arg(long)]
        id: String,
        /// Path to SQLite store
        #[arg(long, default_value = "data/repoquery.db")]
        store: PathBuf,
    },
}

#[derive(clap::Args)]
pub struct TrustArgs {
    /// Decay factor per traversal level (0.0 – 1.0)
    #[arg(long, default_value = "0.5")]
    pub decay: f64,
    /// Minimum score to include in output
    #[arg(long, default_value = "0.001")]
    pub min_score: f64,
    /// Number of results to show (0 = all)
    #[arg(long, default_value = "20")]
    pub limit: usize,
    /// Path to SQLite store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
}

#[derive(clap::Args)]
pub struct RunArgs {
    /// Platform to expand (github, gitlab, etc.)
    #[arg(long, default_value = "github")]
    pub platform: String,
    /// Concurrency (max parallel seed processors)
    #[arg(long, default_value = "4")]
    pub concurrency: usize,
    /// Path to SQLite store
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
}

#[derive(clap::Args)]
pub struct CollectArgs {
    /// Path to the graph store (reads trust scores)
    #[arg(long, default_value = "data/repoquery.db")]
    pub store: PathBuf,
    /// Path to the repo store (writes collected repos)
    #[arg(long, default_value = "data/repoquery.db")]
    pub repo_store: PathBuf,
    /// Platform to collect from
    #[arg(long, default_value = "github")]
    pub platform: String,
    /// Minimum trust threshold (0.0 – 1.0)
    #[arg(long, default_value = "0.01")]
    pub min_trust: f64,
    /// Attribution string for discovered_via
    #[arg(long, default_value = "graph_expansion")]
    pub discovered_via: String,
}

fn open_store(path: &PathBuf) -> Result<Arc<Mutex<dyn GraphStore + Send>>> {
    let store = SqliteStore::new(path)
        .with_context(|| format!("Failed to open store at {}", path.display()))?;
    Ok(Arc::new(Mutex::new(store)))
}

fn open_repo_store(path: &PathBuf) -> Result<Arc<Mutex<dyn rq_store::RepoStore + Send>>> {
    let store = SqliteStore::new(path)
        .with_context(|| format!("Failed to open repo store at {}", path.display()))?;
    Ok(Arc::new(Mutex::new(store)))
}

fn parse_platform(s: &str) -> Result<PlatformKind> {
    s.parse::<PlatformKind>().map_err(|_| {
        anyhow::anyhow!(
            "Unknown platform '{}'. Valid: github, gitlab, codeberg, gitea, bitbucket, sourcehut",
            s
        )
    })
}

pub async fn run_seed(action: SeedAction) -> Result<()> {
    match action {
        SeedAction::Add {
            platform,
            username,
            max_depth,
            store: store_path,
        } => {
            let platform = parse_platform(&platform)?;
            let store = open_store(&store_path)?;
            let id = format!("{}-{}", platform, username);
            let seed = Seed {
                id,
                platform,
                username,
                status: SeedStatus::Pending,
                depth: 0,
                max_depth,
                added_at: chrono::Utc::now().to_rfc3339(),
                completed_at: None,
                error_message: None,
            };
            let guard = store.lock().await;
            guard.add_seed(&seed)?;
            println!("Added seed: {} ({})", seed.id, seed.platform);
            Ok(())
        }

        SeedAction::List { store: store_path } => {
            let store = open_store(&store_path)?;
            let guard = store.lock().await;
            let seeds = guard.list_seeds()?;
            if seeds.is_empty() {
                println!("No seeds registered.");
                return Ok(());
            }
            println!(
                "{:<24} {:<10} {:<20} {:<10} {}",
                "ID", "Platform", "Username", "Depth", "Status"
            );
            println!("{}", "-".repeat(80));
            for s in &seeds {
                println!(
                    "{:<24} {:<10} {:<20} {}/{} {:<10}",
                    s.id,
                    s.platform.to_string(),
                    s.username,
                    s.depth,
                    s.max_depth,
                    format!("{:?}", s.status),
                );
            }
            Ok(())
        }

        SeedAction::Show {
            id,
            store: store_path,
        } => {
            let store = open_store(&store_path)?;
            let guard = store.lock().await;
            match guard.get_seed(&id)? {
                Some(s) => {
                    println!("ID:           {}", s.id);
                    println!("Platform:     {}", s.platform);
                    println!("Username:     {}", s.username);
                    println!("Status:       {:?}", s.status);
                    println!("Depth:        {}/{}", s.depth, s.max_depth);
                    println!("Added:        {}", s.added_at);
                    if let Some(ref t) = s.completed_at {
                        println!("Completed:    {}", t);
                    }
                    if let Some(ref e) = s.error_message {
                        println!("Error:        {}", e);
                    }
                    Ok(())
                }
                None => {
                    anyhow::bail!("Seed '{}' not found", id);
                }
            }
        }

        SeedAction::Delete {
            id,
            store: store_path,
        } => {
            let store = open_store(&store_path)?;
            let guard = store.lock().await;
            match guard.get_seed(&id)? {
                Some(_) => {
                    // We don't have a `delete_seed` method; simulate via update.
                    // For now, just mark as completed with an error note.
                    // In a full implementation, add a delete_seed to GraphStore.
                    println!("Seed '{}' found. Marking as failed (soft-delete).", id);
                    guard.update_seed_status(&id, "failed")?;
                    Ok(())
                }
                None => {
                    anyhow::bail!("Seed '{}' not found", id);
                }
            }
        }
    }
}

pub async fn run_token(action: TokenAction) -> Result<()> {
    match action {
        TokenAction::Add {
            platform,
            token,
            rate_limit,
            store: store_path,
        } => {
            let platform = parse_platform(&platform)?;
            let store = open_store(&store_path)?;
            let id = format!("{}-{}", platform, &token[..token.len().min(8)]);
            let fgat = FgatToken {
                id,
                platform,
                token_hash: token,
                status: TokenStatus::Available,
                requests_used: 0,
                rate_limit_limit: rate_limit,
                rate_limit_remaining: rate_limit,
                rate_limit_reset_at: None,
                last_used_at: None,
                added_at: chrono::Utc::now().to_rfc3339(),
                expires_at: None,
                notes: None,
            };
            let guard = store.lock().await;
            guard.add_fgat_token(&fgat)?;
            println!("Added token: {}", fgat.id);
            Ok(())
        }

        TokenAction::List { store: store_path } => {
            let store = open_store(&store_path)?;
            let guard = store.lock().await;
            let tokens = guard.list_fgat_tokens()?;
            if tokens.is_empty() {
                println!("No tokens registered.");
                return Ok(());
            }
            println!(
                "{:<24} {:<10} {:<20} {:<8} {:<10}",
                "ID", "Platform", "Token Hash", "Remaining", "Status"
            );
            println!("{}", "-".repeat(80));
            for t in &tokens {
                println!(
                    "{:<24} {:<10} {:<20} {:<8} {:<10}",
                    t.id,
                    t.platform.to_string(),
                    &t.token_hash[..t.token_hash.len().min(16)],
                    t.rate_limit_remaining
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    format!("{:?}", t.status),
                );
            }
            Ok(())
        }

        TokenAction::Remove {
            id,
            store: store_path,
        } => {
            let store = open_store(&store_path)?;
            let guard = store.lock().await;
            guard.update_fgat_token_status(&id, "revoked")?;
            println!("Token '{}' revoked.", id);
            Ok(())
        }
    }
}

pub async fn run_expand(args: RunArgs) -> Result<()> {
    let platform = parse_platform(&args.platform)?;
    let store = open_store(&args.store)?;

    // Setup FGAT pool
    let pool = Arc::new(FgatPool::new(store.clone()));
    pool.load(platform).await?;

    // Setup API client
    let client: Arc<dyn PlatformApiClient + Send + Sync> =
        Arc::new(GitHubClient::new(pool.clone()));

    // Run traverser
    let traverser =
        BfsTraverser::new(store.clone(), client, pool.clone()).with_concurrency(args.concurrency);

    println!("Starting BFS graph expansion...");
    let report = traverser.run().await?;

    println!();
    println!("Expansion complete:");
    println!("  Seeds completed: {}", report.seeds_completed);
    println!("  Edges discovered: {}", report.edges_discovered);
    println!("  Errors: {}", report.errors);

    Ok(())
}

pub async fn run_trust(args: TrustArgs) -> Result<()> {
    let store = open_store(&args.store)?;
    let config = TrustConfig {
        decay_factor: args.decay,
        min_score: args.min_score,
        ..TrustConfig::default()
    };

    let guard = store.lock().await;
    let entries = TrustScorer::compute(&*guard, &config)?;

    if entries.is_empty() {
        println!("No trust scores computed. Add seeds and run expansion first.");
        return Ok(());
    }

    let show = if args.limit == 0 {
        entries.len()
    } else {
        args.limit.min(entries.len())
    };

    println!(
        "{:<6} {:<28} {:<8} {:<10}",
        "Rank", "User", "Depth", "Score"
    );
    println!("{}", "-".repeat(54));
    for (i, entry) in entries.iter().take(show).enumerate() {
        println!(
            "{:<6} {:<28} {:<8} {:.6}",
            i + 1,
            entry.username,
            entry.depth,
            entry.score
        );
    }

    if show < entries.len() {
        println!(
            "... and {} more (use --limit 0 to show all)",
            entries.len() - show
        );
    }

    Ok(())
}

pub async fn run_collect(args: CollectArgs) -> Result<()> {
    let platform = parse_platform(&args.platform)?;
    let store = open_store(&args.store)?;
    let repo_store = open_repo_store(&args.repo_store)?;

    // Compute trust scores
    let config = TrustConfig {
        min_score: args.min_trust,
        ..TrustConfig::default()
    };

    let guard = store.lock().await;
    let entries = TrustScorer::compute(&*guard, &config)?;
    drop(guard);

    if entries.is_empty() {
        println!("No users meet the minimum trust threshold.");
        println!("Add seeds, run expansion, then retry.");
        return Ok(());
    }

    println!(
        "{} users meet trust threshold {:.3}. Fetching repos...",
        entries.len(),
        args.min_trust
    );

    // Setup API client
    let pool = Arc::new(FgatPool::new(store.clone()));
    pool.load(platform).await?;
    let client: Arc<dyn PlatformApiClient + Send + Sync> =
        Arc::new(GitHubClient::new(pool.clone()));

    // Run collector
    let collector = RepoCollector::new(repo_store.clone(), client);
    let report = collector
        .collect(&entries, args.min_trust, &args.discovered_via)
        .await?;

    println!();
    println!("Collection complete:");
    println!("  Users processed: {}", report.users_processed);
    println!("  Repos collected: {}", report.repos_collected);
    println!("  Repos merged: {}", report.repos_merged);
    println!("  Errors: {}", report.errors);

    Ok(())
}
