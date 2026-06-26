use super::Repository;
use chrono::{Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Activity status classification for a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityStatus {
    Active,
    Maintained,
    Stale,
    Abandoned,
    Unknown,
}

impl ActivityStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ActivityStatus::Active => "Active",
            ActivityStatus::Maintained => "Maintained",
            ActivityStatus::Stale => "Stale",
            ActivityStatus::Abandoned => "Abandoned",
            ActivityStatus::Unknown => "Unknown",
        }
    }
}

/// Result of activity classification for a single repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityResult {
    pub repo_name: String,
    pub full_name: String,
    pub owner: String,
    pub status: ActivityStatus,
    pub last_activity: String,
    pub stars: u32,
    pub language: String,
}

/// Activity summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub total: usize,
    pub active: usize,
    pub maintained: usize,
    pub stale: usize,
    pub abandoned: usize,
    pub unknown: usize,
}

/// Classify a repository's activity status based on last commit date.
///
/// Uses `last_commit_date` from quality metrics when available, falling back
/// to `last_star_update`. Returns `ActivityStatus::Unknown` when no date is available.
pub fn classify_activity(
    repo: &Repository,
    active_months: u64,
    stale_months: u64,
) -> ActivityStatus {
    let date_str = repo
        .quality_metrics
        .last_commit_date
        .as_deref()
        .unwrap_or(&repo.quality_metrics.last_star_update);

    let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return ActivityStatus::Unknown,
    };

    let today = Utc::now().date_naive();
    let active_cutoff = today - Days::new(active_months * 30);
    let stale_cutoff = today - Days::new(stale_months * 30);

    if date >= active_cutoff {
        ActivityStatus::Active
    } else if date >= stale_cutoff {
        ActivityStatus::Maintained
    } else if date >= today - Days::new(24 * 30) {
        ActivityStatus::Stale
    } else {
        ActivityStatus::Abandoned
    }
}

/// Classify all repositories and return categorized results.
pub fn classify_all(
    repos: &[Repository],
    active_months: u64,
    stale_months: u64,
) -> Vec<ActivityResult> {
    repos
        .iter()
        .map(|r| {
            let status = classify_activity(r, active_months, stale_months);
            let last_activity = r
                .quality_metrics
                .last_commit_date
                .as_deref()
                .unwrap_or(&r.quality_metrics.last_star_update)
                .to_string();
            ActivityResult {
                repo_name: r.metadata.name.clone(),
                full_name: r.metadata.full_name.clone(),
                owner: r.metadata.owner.clone(),
                status,
                last_activity,
                stars: r.metadata.stars,
                language: r.metadata.primary_language.clone(),
            }
        })
        .collect()
}

/// Compute summary statistics from classification results.
pub fn summarize(results: &[ActivityResult]) -> ActivitySummary {
    let mut summary = ActivitySummary {
        total: results.len(),
        active: 0,
        maintained: 0,
        stale: 0,
        abandoned: 0,
        unknown: 0,
    };
    for r in results {
        match r.status {
            ActivityStatus::Active => summary.active += 1,
            ActivityStatus::Maintained => summary.maintained += 1,
            ActivityStatus::Stale => summary.stale += 1,
            ActivityStatus::Abandoned => summary.abandoned += 1,
            ActivityStatus::Unknown => summary.unknown += 1,
        }
    }
    summary
}

/// Trending score: stars per day since the repo was added.
pub fn trend_score(repo: &Repository) -> f64 {
    let added = match &repo.added_date {
        Some(d) => match NaiveDate::parse_from_str(d, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return 0.0,
        },
        None => return 0.0,
    };
    let today = Utc::now().date_naive();
    let days = (today - added).num_days();
    if days < 1 {
        return repo.metadata.stars as f64;
    }
    repo.metadata.stars as f64 / days as f64
}
