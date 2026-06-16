use anyhow::Result;
use od_core::{Book, CanonicalData, DataMerger, MergeStrategy, WebReference};
use std::path::PathBuf;

pub async fn run(base: PathBuf, manual: PathBuf, output: PathBuf) -> Result<()> {
    println!("🔀 Merging data sources...");
    println!("  Base: {}", base.display());
    println!("  Manual: {}", manual.display());
    println!("  Output: {}", output.display());

    let base_data = if base.extension().and_then(|s| s.to_str()) == Some("json") {
        CanonicalData::from_json_file(&base)?
    } else {
        CanonicalData::from_yaml_file(&base)?
    };

    println!("\n📊 Base data loaded:");
    println!("  Repositories: {}", base_data.repositories.len());

    let manual_data = if manual.exists() {
        let data = CanonicalData::from_yaml_file(&manual)?;
        println!("  Manual projects: {}", data.manual_projects.len());
        Some(data)
    } else {
        println!("  No manual additions file found, skipping");
        None
    };

    let web_refs_path = manual
        .parent()
        .map(|p| p.join("web_references.yml"))
        .unwrap_or_else(|| "data/canonical/web_references.yml".into());

    let web_refs = if web_refs_path.exists() {
        let content = std::fs::read_to_string(&web_refs_path)?;
        #[derive(serde::Deserialize)]
        struct WebRefFile {
            references: Vec<WebReference>,
        }
        let file: WebRefFile = serde_yml::from_str(&content)?;
        println!("  Web references: {}", file.references.len());
        Some(file.references)
    } else {
        println!("  No web references file found, skipping");
        None
    };

    let books_path = manual
        .parent()
        .map(|p| p.join("books.yml"))
        .unwrap_or_else(|| "data/canonical/books.yml".into());

    let books = if books_path.exists() {
        let content = std::fs::read_to_string(&books_path)?;
        #[derive(serde::Deserialize)]
        struct BookFile {
            books: Vec<Book>,
        }
        let file: BookFile = serde_yml::from_str(&content)?;
        println!("  Books: {}", file.books.len());
        Some(file.books)
    } else {
        println!("  No books file found, skipping");
        None
    };

    println!("\n🔄 Merging data...");
    let merger = DataMerger::new(MergeStrategy::PreferManual);

    let mut merged = merger.merge(base_data, manual_data, web_refs, books)?;

    println!("🔍 Detecting migrations...");
    let migrations = merger.detect_migrations(&mut merged)?;
    if migrations > 0 {
        println!("  Found {} potential migrations", migrations);
    }

    println!("📊 Calculating quality scores...");
    merger.calculate_quality_scores(&mut merged)?;

    merged.to_yaml_file(&output)?;
    println!("\n💾 Merged data saved to: {}", output.display());

    println!("\n✅ Merge Summary:");
    println!("  Total repositories: {}", merged.total_count);
    println!("  Starred repos: {}", merged.repositories.len());
    println!("  Manual projects: {}", merged.manual_projects.len());
    println!("  Web references: {}", merged.web_references.len());
    println!("  Books: {}", merged.books.len());

    if let Some(stats) = &merged.statistics {
        println!("\n📊 Platform Distribution:");
        for (platform, count) in &stats.by_platform {
            println!("  {}: {}", platform, count);
        }
    }

    Ok(())
}
