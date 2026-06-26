//! Helper data for benchmarks.

use rq_core::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository, RepositoryClassification,
    RepositoryMetadata, RepositorySource,
};

/// Create a test repository with the given parameters.
pub fn create_repo(full_name: &str, lang: &str, stars: u32, topics: &[&str]) -> Repository {
    let parts: Vec<&str> = full_name.split('/').collect();
    Repository {
        id: format!("bench-{}", full_name.replace('/', "-")),
        platforms: vec![PlatformInfo {
            platform: Platform::GitHub,
            url: format!("https://github.com/{}", full_name),
            status: PlatformStatus::Active,
            is_primary: true,
            last_verified: None,
            migration_date: None,
            notes: None,
        }],
        metadata: RepositoryMetadata {
            name: parts[1].to_string(),
            owner: parts[0].to_string(),
            full_name: full_name.to_string(),
            description: format!("Benchmark repository: {}", full_name),
            primary_language: lang.to_string(),
            license: Some("MIT".to_string()),
            license_spdx: Some("MIT".to_string()),
            stars,
            topics: topics.iter().map(|t| t.to_string()).collect(),
            homepage: None,
            language_breakdown: None,
            secondary_languages: vec![],
        },
        classification: RepositoryClassification {
            categories: vec![],
            readme_sections: vec![],
            web_reference_topics: vec![],
            language_category: lang.to_string(),
            language_notes: None,
            readme_inclusion: false,
            readme_inclusion_reason: None,
            significance_notes: None,
        },
        quality_metrics: QualityMetrics {
            archive_status: false,
            archive_date: None,
            last_commit_date: Some("2024-12-01".to_string()),
            last_star_update: "2024-12-10".to_string(),
            quality_score: 50,
        },
        source: RepositorySource::GitHubStars,
        added_date: Some("2024-01-01".to_string()),
        manually_curated: false,
        curator_notes: None,
        relations: vec![],
        fork_parent: None,
        fork_parent_url: None,
        custom_tags: vec![],
        fork_ahead: None,
        fork_behind: None,
        domain: None,
        unified_owner_id: None,
        discovered_via: None,
    }
}

/// Compute a simulated quality score.
pub fn compute_quality_score(repo: &Repository) -> u8 {
    let star_score: u8 = match repo.metadata.stars {
        0..=10 => 0,
        11..=100 => 10,
        101..=500 => 25,
        501..=1000 => 40,
        1001..=5000 => 60,
        5001..=10000 => 75,
        _ => 90,
    };
    star_score
        .saturating_add(if repo.metadata.license.is_some() {
            5
        } else {
            0
        })
        .saturating_add(if !repo.metadata.description.is_empty() {
            3
        } else {
            0
        })
        .min(100)
}

/// Classify activity status for benchmarks.
pub fn classify_activity(
    repo: &Repository,
    _active_months: u64,
    _stale_months: u64,
) -> &'static str {
    if repo.quality_metrics.archive_status {
        "Archived"
    } else if repo.metadata.stars > 10000 {
        "Active"
    } else if repo.metadata.stars > 1000 {
        "Maintained"
    } else {
        "Stale"
    }
}

/// Create a batch of repositories for filtering benchmarks.
pub fn create_repo_batch(count: usize) -> Vec<Repository> {
    let langs = [
        "Rust",
        "Go",
        "JavaScript",
        "Python",
        "TypeScript",
        "C++",
        "Java",
        "Ruby",
        "C",
        "Shell",
    ];
    (0..count)
        .map(|i| {
            let lang = langs[i % langs.len()];
            let name = format!("bench-repo-{}", i);
            create_repo(
                &format!("benchmark/{}", name),
                lang,
                (i as u32 * 100) % 50000,
                &["benchmark"],
            )
        })
        .collect()
}
