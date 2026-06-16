use anyhow::Result;
use od_core::CanonicalData;
use od_generate::MarkdownGenerator;
use od_validate::{
    DuplicateRepositoryNameRule, ExternalDataConsistencyRule, NoDuplicateReposRule, Severity,
    ValidUrlsRule, Validator,
};
use std::path::PathBuf;

pub async fn run(
    input: PathBuf,
    include_archived: bool,
    validate: bool,
    validate_sync_data: bool,
    _platforms: bool,
    collection: Option<String>,
) -> Result<()> {
    println!("📝 Generating documents...");
    println!("  Input: {}", input.display());
    println!("  Include archived: {}", include_archived);
    println!("  Validate before generation: {}", validate);
    println!("  Validate sync data: {}", validate_sync_data);
    println!("  Include platform info: {}", _platforms);

    let data = if input.extension().and_then(|s| s.to_str()) == Some("json") {
        CanonicalData::from_json_file(&input)?
    } else {
        CanonicalData::from_yaml_file(&input)?
    };

    if validate {
        println!("\n🔍 Running pre-generation validation...");
        let mut validator = Validator::new();
        validator.add_rule(NoDuplicateReposRule);
        validator.add_rule(ValidUrlsRule::new()?);

        let report = validator.validate(&data);
        if !report.passed() {
            println!("❌ Validation failed with {} errors", report.summary.errors);
            std::process::exit(1);
        }
        println!("✅ Validation passed");
    }

    if validate_sync_data {
        println!("\n🔍 Validating sync data integrity...");
        let mut validator = Validator::new();
        validator.add_rule(ExternalDataConsistencyRule);
        validator.add_rule(DuplicateRepositoryNameRule);

        let report = validator.validate(&data);

        println!("  Errors: {}", report.summary.errors);
        println!("  Warnings: {}", report.summary.warnings);

        if report.summary.errors > 0 {
            println!("\n❌ Sync data validation failed:");
            for issue in report
                .issues
                .iter()
                .filter(|i| matches!(i.severity, Severity::Error))
                .take(5)
            {
                println!("  [{}] {}: {}", issue.code, issue.location.file, issue.message);
            }
            println!("\n💡 Run 'cargo run -- validate --check-external-consistency' for details");
            println!("   Or use '--validate-sync-data=false' to skip this check");
            std::process::exit(1);
        }

        if report.summary.warnings > 0 {
            println!("  ⚠️  {} warnings (non-blocking)", report.summary.warnings);
        }

        println!("✅ Sync data validation passed");
    }

    println!("\n📊 Data loaded:");
    println!("  Total repositories: {}", data.total_count);

    let generator = MarkdownGenerator::new("data/templates")?;

    // Per-collection generation
    if let Some(collection_id) = collection {
        let col = data
            .collections
            .iter()
            .find(|c| c.id == collection_id || c.name == collection_id)
            .ok_or_else(|| anyhow::anyhow!("Collection '{}' not found", collection_id))?;

        let col_repos: Vec<_> = data
            .repositories
            .iter()
            .filter(|r| col.repo_ids.contains(&r.id))
            .cloned()
            .collect();

        println!("  Collection '{}': {} repos", col.name, col_repos.len());

        let out_dir = PathBuf::from("generated").join(&col.id);
        std::fs::create_dir_all(&out_dir)?;

        let list = generator.generate_list_for_collection(
            &col_repos,
            &col.name,
            col.description.as_deref(),
            false,
        )?;
        std::fs::write(out_dir.join("LIST.md"), list)?;

        let table = generator.generate_table_for_collection(
            &col_repos,
            &col.name,
            col.description.as_deref(),
            false,
        )?;
        std::fs::write(out_dir.join("TABLE.md"), table)?;

        println!("✅ Generated {}/LIST.md and {}/TABLE.md", col.id, col.id);
        return Ok(());
    }

    // Standard full generation
    let active_repos = data.active_repositories();
    let archived_repos = data.archived_repositories();

    let mut active_data = CanonicalData {
        repositories: active_repos,
        ..data.clone()
    };
    active_data.calculate_statistics();

    let mut archive_data = CanonicalData {
        repositories: archived_repos,
        ..data.clone()
    };
    archive_data.calculate_statistics();

    println!("  Active: {}", active_data.repositories.len());
    println!("  Archived: {}", archive_data.repositories.len());

    println!("\n📄 Generating LIST.md...");
    let list_content = generator.generate_list(&active_data, true)?;
    std::fs::write("LIST.md", list_content)?;
    println!("  ✅ LIST.md generated");

    println!("📄 Generating TABLE.md...");
    let table_content = generator.generate_table(&active_data, true)?;
    std::fs::write("TABLE.md", table_content)?;
    println!("  ✅ TABLE.md generated");

    if include_archived && !archive_data.repositories.is_empty() {
        println!("📄 Generating ARCHIVE.md...");
        let archive_list = generator.generate_list(&archive_data, true)?;
        std::fs::write("ARCHIVE.md", archive_list)?;
        println!("  ✅ ARCHIVE.md generated");

        println!("📄 Generating ARCHIVE_TABLE.md...");
        let archive_table = generator.generate_table(&archive_data, true)?;
        std::fs::write("ARCHIVE_TABLE.md", archive_table)?;
        println!("  ✅ ARCHIVE_TABLE.md generated");
    }

    println!("\n✅ Generation complete!");
    println!("  LIST.md: {} repos", active_data.repositories.len());
    println!("  TABLE.md: {} repos", active_data.repositories.len());
    if include_archived {
        println!("  ARCHIVE.md: {} repos", archive_data.repositories.len());
        println!(
            "  ARCHIVE_TABLE.md: {} repos",
            archive_data.repositories.len()
        );
    }

    Ok(())
}
