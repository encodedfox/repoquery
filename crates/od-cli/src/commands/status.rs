use anyhow::Result;
use od_core::{CredentialManager, OmnidatumConfig, SyncQualityReport};
use od_sync::SyncCache;

pub async fn run(detailed: bool) -> Result<()> {
    println!("📊 OmniDatum Status\n");

    let config = OmnidatumConfig::load()?;

    let cache = SyncCache::load()?;
    let stats = cache.stats();

    println!("Sync Status:");
    if stats.total_entries > 0 {
        let now = chrono::Utc::now();
        let age = now - stats.oldest_entry;
        println!(
            "  Last sync: {} ({} ago)",
            stats.oldest_entry.format("%Y-%m-%d %H:%M:%S UTC"),
            if age.num_hours() < 24 {
                format!("{} hours", age.num_hours())
            } else {
                format!("{} days", age.num_days())
            }
        );
        println!("  Repositories cached: {}", stats.total_entries);
    } else {
        println!("  No sync performed yet");
        println!("  Run 'cargo run -- sync' to fetch repository metadata");
    }

    let quality_report_path =
        std::path::PathBuf::from("data/cache/sync_quality_report.json");
    if quality_report_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&quality_report_path) {
            if let Ok(report) = serde_json::from_str::<SyncQualityReport>(&content) {
                println!("\nData Quality:");
                println!(
                    "  License coverage: {:.1}%",
                    report.data_completeness.license_coverage
                );
                println!(
                    "  Description coverage: {:.1}%",
                    report.data_completeness.description_coverage
                );
                println!(
                    "  Topics coverage: {:.1}%",
                    report.data_completeness.topics_coverage
                );
                println!(
                    "  Homepage coverage: {:.1}%",
                    report.data_completeness.homepage_coverage
                );

                if report.data_completeness.license_coverage < 90.0 {
                    println!("  ⚠️  License coverage below 90%");
                }
                if report.data_completeness.description_coverage < 90.0 {
                    println!("  ⚠️  Description coverage below 90%");
                }

                if !report.anomalies.is_empty() {
                    println!("\n  Recent anomalies: {}", report.anomalies.len());

                    if detailed {
                        println!("\n  Anomaly Details:");
                        for anomaly in report.anomalies.iter().take(10) {
                            println!("    - {}: {}", anomaly.repo_id, anomaly.message);
                        }
                        if report.anomalies.len() > 10 {
                            println!("    ... and {} more", report.anomalies.len() - 10);
                        }
                    }
                }
            }
        }
    }

    println!("\nCache:");
    println!("  Entries: {}", stats.total_entries);
    println!("  TTL: {} hours", config.sync.cache_ttl_hours);
    if stats.total_entries > 0 {
        println!(
            "  Oldest entry: {} ago",
            chrono::Utc::now()
                .signed_duration_since(stats.oldest_entry)
                .num_days()
        );
    }

    println!("\nConfiguration:");
    println!("  Sync enabled: {}", config.sync.enabled);
    println!("  Parallel workers: {}", config.sync.parallel_workers);
    println!("  Rate limit buffer: {}", config.sync.rate_limit_buffer);

    let cred_mgr = CredentialManager::new(config.credentials.source);
    println!("\nCredentials:");
    match cred_mgr.get_github_token() {
        Ok(token) => {
            println!(
                "  ✅ GitHub token: {} (configured)",
                CredentialManager::redact(&token)
            );

            if detailed {
                println!("\n🔍 Checking GitHub API rate limits...");
                println!("  (Use 'cargo run -- sync' to check current rate limits)");
            }
        }
        Err(e) => {
            println!("  ❌ Not configured: {}", e);
            println!("  Run 'cargo run -- configure' to set up");
        }
    }

    Ok(())
}
