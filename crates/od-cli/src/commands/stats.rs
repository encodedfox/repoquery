use anyhow::Result;
use od_core::{CanonicalData, Repository};
use std::path::PathBuf;

pub async fn run(input: PathBuf, enhanced_json: bool, diff: Option<PathBuf>) -> Result<()> {
    println!("📊 Repository Statistics");
    println!("  Input: {}", input.display());

    let mut data = if input.extension().and_then(|s| s.to_str()) == Some("json") {
        CanonicalData::from_json_file(&input)?
    } else {
        CanonicalData::from_yaml_file(&input)?
    };

    data.calculate_statistics();

    if enhanced_json {
        let json = serde_json::to_string_pretty(&data)?;
        println!("\n{}", json);
        return Ok(());
    }

    if let Some(diff_path) = diff {
        println!("\n📊 Comparing with: {}", diff_path.display());
        let prev_data = if diff_path.extension().and_then(|s| s.to_str()) == Some("json") {
            CanonicalData::from_json_file(&diff_path)?
        } else {
            CanonicalData::from_yaml_file(&diff_path)?
        };

        let repo_diff = data.repositories.len() as i32 - prev_data.repositories.len() as i32;
        let sign = if repo_diff > 0 { "+" } else { "" };
        println!("  Repository count: {}{}", sign, repo_diff);

        if let (Some(curr_stats), Some(prev_stats)) = (&data.statistics, &prev_data.statistics) {
            let active_diff =
                curr_stats.by_status.active as i32 - prev_stats.by_status.active as i32;
            let archived_diff =
                curr_stats.by_status.archived as i32 - prev_stats.by_status.archived as i32;
            println!(
                "  Active repos: {}{}",
                if active_diff > 0 { "+" } else { "" },
                active_diff
            );
            println!(
                "  Archived repos: {}{}",
                if archived_diff > 0 { "+" } else { "" },
                archived_diff
            );
        }
    }

    println!("\n📈 Overall Statistics:");
    println!("  Schema version: {}", data.schema_version);
    println!("  Total repositories: {}", data.total_count);
    println!("  Starred repos: {}", data.repositories.len());
    println!("  Manual projects: {}", data.manual_projects.len());
    println!("  Web references: {}", data.web_references.len());
    println!("  Books: {}", data.books.len());

    if let Some(stats) = &data.statistics {
        println!("\n📊 By Platform:");
        for (platform, count) in &stats.by_platform {
            let pct = (count * 100) as f32 / stats.total as f32;
            println!("  {}: {} ({:.1}%)", platform, count, pct);
        }

        println!("\n📊 By Status:");
        println!(
            "  Active: {} ({:.1}%)",
            stats.by_status.active,
            (stats.by_status.active * 100) as f32 / stats.total as f32
        );
        println!(
            "  Archived: {} ({:.1}%)",
            stats.by_status.archived,
            (stats.by_status.archived * 100) as f32 / stats.total as f32
        );

        println!("\n📊 By Language (top 10):");
        let mut langs: Vec<_> = stats.by_language.iter().collect();
        langs.sort_by(|a, b| b.1.cmp(a.1));
        for (lang, count) in langs.iter().take(10) {
            let pct = (**count * 100) as f32 / stats.total as f32;
            println!("  {}: {} ({:.1}%)", lang, count, pct);
        }

        println!("\n📊 Archive Candidates:");
        let archive_candidates = data
            .all_repositories()
            .iter()
            .filter(|r: &&Repository| r.is_archive_candidate())
            .count();
        println!(
            "  Total: {} ({:.1}%)",
            archive_candidates,
            (archive_candidates * 100) as f32 / stats.total as f32
        );

        println!("\n📊 README Inclusion Candidates:");
        let readme_candidates = data
            .all_repositories()
            .iter()
            .filter(|r: &&Repository| r.meets_readme_criteria())
            .count();
        println!(
            "  Total: {} ({:.1}%)",
            readme_candidates,
            (readme_candidates * 100) as f32 / stats.total as f32
        );
    }

    Ok(())
}
