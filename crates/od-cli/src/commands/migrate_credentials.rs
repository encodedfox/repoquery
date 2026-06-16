use anyhow::Result;
use od_core::{CredentialManager, OmnidatumConfig};
use std::path::PathBuf;

pub async fn run(from: PathBuf, delete_source: bool) -> Result<()> {
    println!("🔄 Migrating Credentials");
    println!("  From: {}", from.display());

    if !from.exists() {
        return Err(anyhow::anyhow!("Source file not found: {}", from.display()));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&from)?;
        let mode = metadata.permissions().mode();
        if mode & 0o077 != 0 {
            println!(
                "⚠️  Warning: Source file has insecure permissions ({:o})",
                mode & 0o777
            );
            println!("   Consider: chmod 600 {:?}", from);
        }
    }

    let token = std::fs::read_to_string(&from)?;
    let token = token.trim();

    if !token.starts_with("ghp_") && !token.starts_with("github_pat_") {
        println!("⚠️  Warning: Token doesn't match expected GitHub format");
    }

    let config = OmnidatumConfig::load()?;
    let cred_mgr = CredentialManager::new(config.credentials.source.clone());
    cred_mgr.store_token(token)?;

    println!("\n✅ Migration successful!");
    println!("   Token: {}", CredentialManager::redact(token));
    println!("   New location: {:?}", config.credentials.source);

    if delete_source {
        std::fs::remove_file(&from)?;
        println!("   🗑️  Source file deleted: {}", from.display());
    } else {
        println!("\n💡 Source file preserved at: {}", from.display());
        println!("   You can safely delete it now.");
    }

    Ok(())
}
