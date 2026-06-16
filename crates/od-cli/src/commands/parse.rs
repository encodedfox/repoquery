use anyhow::Result;
use od_core::{CanonicalData, ListParser};
use std::path::PathBuf;

pub async fn run(list: PathBuf, output: PathBuf) -> Result<()> {
    println!("📖 Parsing markdown files...");
    println!("  LIST: {}", list.display());
    println!("  Output: {}", output.display());

    let list_content = std::fs::read_to_string(&list)?;

    let parser = ListParser::new()?;
    let repos = parser.parse_file(&list_content)?;

    println!("\n✅ Successfully parsed {} repositories!", repos.len());

    let mut canonical = CanonicalData::new();
    canonical.repositories = repos;
    canonical.total_count = canonical.repositories.len();
    canonical.generated_by = "omnidatum-processor/list-parser".to_string();
    canonical.calculate_statistics();

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    canonical.to_yaml_file(&output)?;

    println!("📝 Canonical data saved to: {}", output.display());

    if let Some(stats) = &canonical.statistics {
        println!("\n📊 Statistics:");
        println!("  Total repositories: {}", stats.total);
        println!("  Active: {}", stats.by_status.active);
        println!("  Archived: {}", stats.by_status.archived);
        println!("  Languages: {}", stats.by_language.len());
    }

    Ok(())
}
