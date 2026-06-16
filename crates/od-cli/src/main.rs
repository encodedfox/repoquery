//! OmniDatum Processor - Enhanced documentation processor for GitHub starred repositories

mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "omnidatum-processor")]
#[command(author = "encodedfox")]
#[command(version = "0.1.0")]
#[command(about = "Enhanced documentation processor for OmniDatum repository", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum CollectionAction {
    /// List all collections
    List,
    /// Create a new collection
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Show collection details
    Show {
        #[arg(long)]
        id: String,
    },
    /// Add a repository to a collection
    Add {
        #[arg(long)]
        collection: String,
        #[arg(long)]
        repo: String,
    },
    /// Remove a repository from a collection
    Remove {
        #[arg(long)]
        collection: String,
        #[arg(long)]
        repo: String,
    },
    /// Delete a collection
    Delete {
        #[arg(long)]
        id: String,
    },
    /// Auto-generate collections from GitHub topics
    AutoGenerate {
        /// Minimum repos with a topic to create a collection
        #[arg(long, default_value = "3")]
        min_repos: usize,
        /// Path to data store
        #[arg(long, default_value = "data/omnidatum.db")]
        store: PathBuf,
    },
}

#[derive(Subcommand)]
enum RepoAction {
    /// Add a tag to a repository
    Tag {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        tag: String,
        #[arg(long, default_value = "data/omnidatum.db")]
        store: PathBuf,
    },
    /// Remove a tag from a repository
    Untag {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        tag: String,
        #[arg(long, default_value = "data/omnidatum.db")]
        store: PathBuf,
    },
    /// Set a note on a repository
    Note {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        text: String,
        #[arg(long, default_value = "data/omnidatum.db")]
        store: PathBuf,
    },
    /// Show repository details
    Show {
        #[arg(long)]
        repo: String,
        #[arg(long, default_value = "data/omnidatum.db")]
        store: PathBuf,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Parse existing LIST.md and TABLE.md into canonical format
    Parse {
        /// Path to LIST.md file
        #[arg(short, long, default_value = "LIST.md")]
        list: PathBuf,

        /// Path to TABLE.md file
        #[arg(short, long, default_value = "TABLE.md")]
        table: PathBuf,

        /// Output path for canonical data
        #[arg(short, long, default_value = "data/canonical/repositories.yml")]
        output: PathBuf,
    },

    /// Validate canonical data
    Validate {
        /// Path to canonical data file (YAML or JSON)
        #[arg(short, long, default_value = "data/canonical/repositories.yml")]
        input: PathBuf,

        /// Output path for validation report
        #[arg(short, long, default_value = "data/cache/validation_report.json")]
        output: PathBuf,

        /// Check external data consistency (E007, E008)
        #[arg(long, default_value = "true")]
        check_external_consistency: bool,
    },

    /// Generate markdown documents from canonical data
    Generate {
        /// Path to canonical data file
        #[arg(short, long, default_value = "data/canonical/repositories.yml")]
        input: PathBuf,

        /// Include archived repositories in separate files
        #[arg(long, default_value = "true")]
        include_archived: bool,

        /// Validate data before generating
        #[arg(long, default_value = "false")]
        validate: bool,

        /// Validate sync data integrity before generation
        #[arg(long, default_value = "true")]
        validate_sync_data: bool,

        /// Include multi-platform information in output
        #[arg(long, default_value = "false")]
        platforms: bool,

        /// Generate for a specific collection (by id or name)
        #[arg(long)]
        collection: Option<String>,
    },

    /// Merge manual additions into canonical data
    Merge {
        /// Path to base canonical data
        #[arg(short, long, default_value = "data/canonical/repositories.yml")]
        base: PathBuf,

        /// Path to manual additions YAML
        #[arg(short, long, default_value = "data/canonical/manual_additions.yml")]
        manual: PathBuf,

        /// Output path for merged canonical data
        #[arg(short, long, default_value = "data/canonical/repositories.yml")]
        output: PathBuf,
    },

    /// Show statistics about repository data
    Stats {
        /// Path to canonical data file
        #[arg(short, long, default_value = "data/canonical/repositories.yml")]
        input: PathBuf,

        /// Output as enhanced JSON with detailed metadata
        #[arg(long, default_value = "false")]
        enhanced_json: bool,

        /// Compare with previous statistics
        #[arg(long)]
        diff: Option<PathBuf>,
    },

    /// Configure OmniDatum settings and credentials
    Configure {
        /// Interactive mode (prompt for values)
        #[arg(short, long, default_value = "true")]
        interactive: bool,

        /// GitHub token (non-interactive)
        #[arg(long)]
        github_token: Option<String>,

        /// Show current configuration
        #[arg(long, default_value = "false")]
        show: bool,
    },

    /// Migrate credentials from legacy location
    MigrateCredentials {
        /// Path to legacy token file
        #[arg(long)]
        from: PathBuf,

        /// Delete source file after migration
        #[arg(long, default_value = "false")]
        delete_source: bool,
    },

    /// Sync repository metadata from external sources
    Sync {
        /// Sync specific repositories (comma-separated owner/name)
        #[arg(long)]
        repos: Option<String>,

        /// Force sync even if cached
        #[arg(long, default_value = "false")]
        force: bool,

        /// Dry run (show what would be synced)
        #[arg(long, default_value = "false")]
        dry_run: bool,

        /// Verbose output
        #[arg(short, long, default_value = "false")]
        verbose: bool,

        /// Clear cache before syncing
        #[arg(long, default_value = "false")]
        clear_cache: bool,

        /// Path to canonical data file
        #[arg(short, long, default_value = "data/canonical/repositories.yml")]
        input: PathBuf,

        /// Relation types to sync (comma-separated: starred,owned,forked,watching)
        #[arg(long)]
        relations: Option<String>,

        /// Check fork status (commits ahead/behind upstream)
        #[arg(long, default_value = "false")]
        check_forks: bool,
    },

    /// Show sync status and system health
    Status {
        /// Show detailed information
        #[arg(long, default_value = "false")]
        detailed: bool,
    },

    /// Import data from one store format to another (e.g. YAML → SQLite)
    Import {
        #[arg(long, default_value = "data/canonical/repositories.yml")]
        from: PathBuf,
        #[arg(long, default_value = "data/omnidatum.db")]
        to: PathBuf,
    },

    /// Export data from one store format to another (e.g. SQLite → YAML)
    Export {
        #[arg(long, default_value = "data/omnidatum.db")]
        from: PathBuf,
        #[arg(long, default_value = "data/canonical/repositories.yml")]
        to: PathBuf,
    },

    /// Manage repository collections
    Collections {
        #[command(subcommand)]
        action: CollectionAction,
    },

    /// Interactive terminal UI for browsing and managing repositories
    Tui {
        /// Path to data store (SQLite DB or YAML file)
        #[arg(long, default_value = "data/omnidatum.db")]
        store: PathBuf,
    },

    /// Manage individual repository metadata
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialise logger after parsing so the verbose flag can set the level.
    let verbose = matches!(&cli.command, Commands::Sync { verbose, .. } if *verbose);
    {
        use tracing_subscriber::EnvFilter;
        let filter = if verbose {
            EnvFilter::new("debug")
        } else {
            EnvFilter::from_default_env()
        };
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    match cli.command {
        Commands::Parse {
            list,
            table: _,
            output,
        } => crate::commands::parse::run(list, output).await,

        Commands::Validate {
            input,
            output,
            check_external_consistency,
        } => crate::commands::validate::run(input, output, check_external_consistency).await,

        Commands::Generate {
            input,
            include_archived,
            validate,
            validate_sync_data,
            platforms,
            collection,
        } => crate::commands::generate::run(input, include_archived, validate, validate_sync_data, platforms, collection).await,

        Commands::Merge {
            base,
            manual,
            output,
        } => crate::commands::merge::run(base, manual, output).await,

        Commands::Stats {
            input,
            enhanced_json,
            diff,
        } => crate::commands::stats::run(input, enhanced_json, diff).await,

        Commands::Configure {
            interactive,
            github_token,
            show,
        } => crate::commands::configure::run(interactive, github_token, show).await,

        Commands::MigrateCredentials {
            from,
            delete_source,
        } => crate::commands::migrate_credentials::run(from, delete_source).await,

        Commands::Sync {
            repos,
            force,
            dry_run,
            verbose,
            clear_cache,
            input,
            relations,
            check_forks,
        } => crate::commands::sync_cmd::run(repos, force, dry_run, verbose, clear_cache, input, relations, check_forks).await,

        Commands::Status { detailed } => crate::commands::status::run(detailed).await,

        Commands::Import { from, to } => crate::commands::import::run(from, to).await,

        Commands::Export { from, to } => crate::commands::export::run(from, to).await,

        Commands::Collections { action } => match action {
            CollectionAction::List => crate::commands::collections::list().await,
            CollectionAction::Create { name, description } => {
                crate::commands::collections::create(name, description).await
            }
            CollectionAction::Show { id } => crate::commands::collections::show(id).await,
            CollectionAction::Add { collection, repo } => {
                crate::commands::collections::add(collection, repo).await
            }
            CollectionAction::Remove { collection, repo } => {
                crate::commands::collections::remove(collection, repo).await
            }
            CollectionAction::Delete { id } => crate::commands::collections::delete(id).await,
            CollectionAction::AutoGenerate { min_repos, store } => {
                crate::commands::collections::auto_generate(min_repos, store).await
            }
        },

        Commands::Tui { store } => crate::commands::tui::run(store).await,

        Commands::Repo { action } => match action {
            RepoAction::Tag { repo, tag, store } => {
                crate::commands::repo::tag(repo, tag, store).await
            }
            RepoAction::Untag { repo, tag, store } => {
                crate::commands::repo::untag(repo, tag, store).await
            }
            RepoAction::Note { repo, text, store } => {
                crate::commands::repo::note(repo, text, store).await
            }
            RepoAction::Show { repo, store } => {
                crate::commands::repo::show(repo, store).await
            }
        },
    }
}
