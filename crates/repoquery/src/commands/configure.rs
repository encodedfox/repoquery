use anyhow::Result;
use rq_core::{CredentialManager, RepoqueryConfig};

pub async fn run(interactive: bool, github_token: Option<String>, show: bool) -> Result<()> {
    println!("Configuration");

    if show {
        let config = RepoqueryConfig::load()?;
        println!("\nCurrent Configuration:");
        println!("\n[sync]");
        println!("  enabled = {}", config.sync.enabled);
        println!("  interval_hours = {}", config.sync.interval_hours);
        println!("  parallel_workers = {}", config.sync.parallel_workers);
        println!("  cache_ttl_hours = {}", config.sync.cache_ttl_hours);
        println!("  rate_limit_buffer = {}", config.sync.rate_limit_buffer);
        println!(
            "  request_timeout_secs = {}",
            config.sync.request_timeout_secs
        );

        println!("\n[credentials]");
        println!("  source = {:?}", config.credentials.source);
        if let Some(path) = &config.credentials.file_path {
            println!("  file_path = {:?}", path);
        }

        println!("\n[validation]");
        println!("  rules enabled: {}", config.validation.rules.len());

        println!("\n[generation]");
        println!(
            "  include_archived = {}",
            config.generation.include_archived
        );
        println!("  platform_info = {}", config.generation.platform_info);
        println!(
            "  stats_detail_level = {:?}",
            config.generation.stats_detail_level
        );

        println!("\nConfig location: {:?}", RepoqueryConfig::config_path());

        let cred_mgr = CredentialManager::new(config.credentials.source);
        match cred_mgr.get_github_token() {
            Ok(token) => {
                println!(
                    "GitHub token: {} (configured)",
                    CredentialManager::redact(&token)
                );
            }
            Err(_) => {
                println!("GitHub token: not configured");
            }
        }

        return Ok(());
    }

    let token = if let Some(t) = github_token {
        t
    } else if interactive {
        println!("\nGitHub Token Setup");
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

    if let Err(warning) = CredentialManager::validate_token_format(&token) {
        println!("Warning: {}", warning);
        println!("   Proceeding anyway, but this may not be a valid GitHub token.");
    }

    let config = RepoqueryConfig::load()?;
    let cred_mgr = CredentialManager::new(config.credentials.source.clone());
    cred_mgr.store_token(&token)?;

    // Validate token scopes via GitHub API
    validate_token_scopes(&token).await?;

    println!("\nCredentials configured successfully!");
    println!("   Token: {}", CredentialManager::redact(&token));
    println!("   Storage: {:?}", config.credentials.source);

    Ok(())
}

/// Validate GitHub token scopes by making an API request to the /user endpoint
async fn validate_token_scopes(token: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "repoquery/0.1.0")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to validate token with GitHub: {}", e))?;

    let status = response.status();
    let scopes = response
        .headers()
        .get("X-OAuth-Scopes")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let scope_list: Vec<&str> = scopes.split(',').map(|s| s.trim()).collect();

    if status.is_success() {
        if !scope_list.is_empty() && scope_list[0] != "" {
            println!("   GitHub API responded with scopes: {}", scopes);
        } else {
            println!("   GitHub token validated (no scopes reported by API)");
        }
    } else {
        println!(
            "   Warning: GitHub API returned status {} for token validation",
            status
        );
        println!("   The token may not have the required permissions.");
    }

    Ok(())
}
