use anyhow::Result;
use clap::Subcommand;
use rq_core::config::RepoqueryConfig;
use toml;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Create default configuration file
    Init,
    /// Display current configuration
    Show,
    /// Set a configuration value: config set <key> <value>
    Set {
        /// Configuration key (e.g., sync.parallel_workers)
        key: String,
        /// Configuration value
        value: String,
    },
}

pub async fn run(action: ConfigCommand) -> Result<()> {
    match action {
        ConfigCommand::Init => run_init().await,
        ConfigCommand::Show => run_show().await,
        ConfigCommand::Set { key, value } => run_set(&key, &value).await,
    }
}

async fn run_init() -> Result<()> {
    let config_path = RepoqueryConfig::config_path();
    if config_path.exists() {
        println!("Config already exists at {:?}", config_path);
        return Ok(());
    }
    let config = RepoqueryConfig::default();
    config.save()?;
    println!("Default configuration created at {:?}", config_path);
    Ok(())
}

async fn run_show() -> Result<()> {
    let config = RepoqueryConfig::load()?;
    println!("{}", toml::to_string_pretty(&config)?);
    println!("Config location: {:?}", RepoqueryConfig::config_path());
    Ok(())
}

async fn run_set(key: &str, value: &str) -> Result<()> {
    let mut config = RepoqueryConfig::load()?;

    match key {
        "sync.enabled" => config.sync.enabled = value.parse()?,
        "sync.interval_hours" => config.sync.interval_hours = value.parse()?,
        "sync.parallel_workers" => config.sync.parallel_workers = value.parse()?,
        "sync.cache_ttl_hours" => config.sync.cache_ttl_hours = value.parse()?,
        "sync.rate_limit_buffer" => config.sync.rate_limit_buffer = value.parse()?,
        "sync.request_timeout_secs" => config.sync.request_timeout_secs = value.parse()?,
        "storage.mode" => match value {
            "yaml" => config.storage.mode = rq_core::config::StorageMode::Yaml,
            "sqlite" => config.storage.mode = rq_core::config::StorageMode::Sqlite,
            "dual" => config.storage.mode = rq_core::config::StorageMode::Dual,
            _ => return Err(anyhow::anyhow!("Invalid storage mode: {}. Use yaml, sqlite, or dual", value)),
        },
        "credentials.source" => match value {
            "env" => config.credentials.source = rq_core::config::CredentialSource::Env,
            "file" => config.credentials.source = rq_core::config::CredentialSource::File,
            "keychain" => config.credentials.source = rq_core::config::CredentialSource::Keychain,
            _ => return Err(anyhow::anyhow!("Invalid credential source: {}. Use env, file, or keychain", value)),
        },
        _ => return Err(anyhow::anyhow!("Unknown config key: {}. Valid keys: sync.enabled, sync.interval_hours, sync.parallel_workers, sync.cache_ttl_hours, sync.rate_limit_buffer, sync.request_timeout_secs, storage.mode, credentials.source", key)),
    }

    config.save()?;
    println!("Set {} = {}", key, value);
    Ok(())
}
