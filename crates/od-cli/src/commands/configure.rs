use anyhow::Result;
use od_core::{CredentialManager, OmnidatumConfig};

pub async fn run(interactive: bool, github_token: Option<String>, show: bool) -> Result<()> {
    println!("⚙️  OmniDatum Configuration");

    if show {
        let config = OmnidatumConfig::load()?;
        println!("\n📋 Current Configuration:");
        println!("\n[sync]");
        println!("  enabled = {}", config.sync.enabled);
        println!("  interval_hours = {}", config.sync.interval_hours);
        println!("  parallel_workers = {}", config.sync.parallel_workers);
        println!("  cache_ttl_hours = {}", config.sync.cache_ttl_hours);
        println!("  rate_limit_buffer = {}", config.sync.rate_limit_buffer);
        println!("  request_timeout_secs = {}", config.sync.request_timeout_secs);

        println!("\n[credentials]");
        println!("  source = {:?}", config.credentials.source);
        if let Some(path) = &config.credentials.file_path {
            println!("  file_path = {:?}", path);
        }

        println!("\n[validation]");
        println!("  rules enabled: {}", config.validation.rules.len());

        println!("\n[generation]");
        println!("  include_archived = {}", config.generation.include_archived);
        println!("  platform_info = {}", config.generation.platform_info);
        println!("  stats_detail_level = {:?}", config.generation.stats_detail_level);

        println!("\n📁 Config location: {:?}", OmnidatumConfig::config_path());

        let cred_mgr = CredentialManager::new(config.credentials.source);
        match cred_mgr.get_github_token() {
            Ok(token) => {
                println!("🔑 GitHub token: {} (configured)", CredentialManager::redact(&token));
            }
            Err(_) => {
                println!("⚠️  GitHub token: not configured");
            }
        }

        return Ok(());
    }

    let token = if let Some(t) = github_token {
        t
    } else if interactive {
        println!("\n🔐 GitHub Token Setup");
        println!("To use the sync feature, you need a GitHub Personal Access Token.");
        println!("Required permissions: public_repo (read-only)");
        println!("\nCreate a token at: https://github.com/settings/tokens/new");
        println!("\nEnter your GitHub token (input will be hidden):");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    } else {
        return Err(anyhow::anyhow!(
            "Either --interactive or --github-token must be specified"
        ));
    };

    if !token.starts_with("ghp_") && !token.starts_with("github_pat_") {
        println!("⚠️  Warning: Token doesn't match expected format (ghp_* or github_pat_*)");
        println!("   Proceeding anyway, but this may not be a valid GitHub token.");
    }

    let config = OmnidatumConfig::load()?;
    let cred_mgr = CredentialManager::new(config.credentials.source.clone());
    cred_mgr.store_token(&token)?;

    println!("\n✅ Credentials configured successfully!");
    println!("   Token: {}", CredentialManager::redact(&token));
    println!("   Storage: {:?}", config.credentials.source);

    Ok(())
}
