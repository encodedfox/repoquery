use anyhow::Result;
use od_core::CanonicalData;
use od_validate::{
    DuplicateRepositoryNameRule, ExternalDataConsistencyRule, MissingLicenseRule,
    MissingMetadataRule, NoDuplicateReposRule, PlatformMigrationRule, ReadmeCrossReferenceRule,
    StaleContentRule, ValidUrlsRule, Validator,
};
use std::path::PathBuf;

pub async fn run(input: PathBuf, output: PathBuf, check_external_consistency: bool) -> Result<()> {
    println!("✅ Validating data...");
    println!("  Input: {}", input.display());
    println!("  Report: {}", output.display());

    let data = if input.extension().and_then(|s| s.to_str()) == Some("json") {
        CanonicalData::from_json_file(&input)?
    } else {
        CanonicalData::from_yaml_file(&input)?
    };

    println!("\n📊 Data loaded:");
    println!("  Repositories: {}", data.repositories.len());
    println!("  Manual projects: {}", data.manual_projects.len());
    println!("  Web references: {}", data.web_references.len());
    println!("  Books: {}", data.books.len());

    let mut validator = Validator::new();
    validator.add_rule(NoDuplicateReposRule);
    validator.add_rule(MissingLicenseRule {
        allow_low_star_repos: true,
    });
    validator.add_rule(ValidUrlsRule::new()?);
    validator.add_rule(ReadmeCrossReferenceRule);
    validator.add_rule(PlatformMigrationRule);
    validator.add_rule(MissingMetadataRule);
    validator.add_rule(StaleContentRule { stale_days: 730 });

    if check_external_consistency {
        validator.add_rule(ExternalDataConsistencyRule);
        validator.add_rule(DuplicateRepositoryNameRule);
    }

    println!(
        "\n🔍 Running validation rules ({} total)...",
        if check_external_consistency { "9" } else { "7" }
    );

    let report = validator.validate(&data);

    println!("\n📋 Validation Summary:");
    println!("  Errors: {}", report.summary.errors);
    println!("  Warnings: {}", report.summary.warnings);
    println!("  Info: {}", report.summary.info);
    println!("  Total repos: {}", report.summary.total_repos);

    println!("\n📊 Metrics:");
    println!("  Platforms:");
    for (platform, count) in &report.metrics.repos_by_platform {
        println!("    {}: {}", platform, count);
    }
    println!("  Archived: {}", report.metrics.archived_count);
    println!("  Missing licenses: {}", report.metrics.missing_licenses);
    println!("  Stale references: {}", report.metrics.stale_references);
    println!("  Migrations: {}", report.metrics.migration_count);

    report.to_json_file(&output)?;
    println!("\n💾 Report saved to: {}", output.display());

    if !report.passed() {
        println!(
            "\n❌ Validation FAILED - {} errors found",
            report.summary.errors
        );
        std::process::exit(1);
    } else {
        println!("\n✅ Validation PASSED - No errors found!");
    }

    Ok(())
}
