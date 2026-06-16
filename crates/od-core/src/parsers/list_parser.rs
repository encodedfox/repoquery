//! Parser for LIST.md format

use crate::models::{
    Platform, PlatformInfo, PlatformStatus, QualityMetrics, Repository, RepositoryClassification,
    RepositoryMetadata, RepositorySource,
};
use regex::Regex;

/// Parsed repository entry from LIST.md
#[derive(Debug, Clone)]
pub struct ParsedListEntry {
    pub full_name: String,
    pub url: String,
    pub description: String,
    pub license: Option<String>,
    pub stars: u32,
    pub is_archived: bool,
    pub language: String,
}

/// Parser for LIST.md markdown format
pub struct ListParser {
    entry_regex: Regex,
    license_regex: Regex,
    stars_regex: Regex,
}

impl ListParser {
    /// Create new LIST.md parser
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            // Matches: - [owner/repo](url) - description \[*License*\] (⭐️stars) *Archived!*
            entry_regex: Regex::new(r"^-\s+\[([^\]]+)\]\(([^\)]+)\)\s+-\s+(.+)$")?,
            // Matches: \[*License Name*\]
            license_regex: Regex::new(r"\\\[\*([^\*]+)\*\\\]")?,
            // Matches: (⭐️123) or (⭐️123) *Archived!*
            stars_regex: Regex::new(r"\(⭐️([0-9,]+)\)")?,
        })
    }

    /// Parse entire LIST.md file
    pub fn parse_file(&self, content: &str) -> crate::Result<Vec<Repository>> {
        let mut repos = Vec::new();
        let mut current_language = String::from("Unknown");

        for line in content.lines() {
            // Detect language section headers: ## Language
            if line.starts_with("## ") && !line.starts_with("## Contents") {
                current_language = line[3..].trim().to_string();
                tracing::debug!("Entering language section: {}", current_language);
                continue;
            }

            // Skip non-entry lines
            if !line.starts_with("- [") {
                continue;
            }

            // Parse repository entry
            match self.parse_entry(line, &current_language) {
                Ok(repo) => repos.push(repo),
                Err(e) => tracing::warn!("Failed to parse line: {} - Error: {}", line, e),
            }
        }

        tracing::info!("Parsed {} repositories from LIST.md", repos.len());
        Ok(repos)
    }

    /// Parse single repository entry line
    pub fn parse_entry(&self, line: &str, language: &str) -> crate::Result<Repository> {
        // Extract basic structure: [name](url) - description
        let caps = self
            .entry_regex
            .captures(line)
            .ok_or_else(|| crate::CoreError::Config("Failed to match entry format".to_string()))?;

        let full_name = caps
            .get(1)
            .ok_or_else(|| crate::CoreError::Config("Missing name".to_string()))?
            .as_str()
            .to_string();

        let url = caps
            .get(2)
            .ok_or_else(|| crate::CoreError::Config("Missing URL".to_string()))?
            .as_str()
            .to_string();

        let remaining = caps
            .get(3)
            .ok_or_else(|| crate::CoreError::Config("Missing description".to_string()))?
            .as_str();

        // Extract license if present
        let license = self
            .license_regex
            .captures(remaining)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        // Extract stars
        let stars = self
            .stars_regex
            .captures(remaining)
            .and_then(|c| c.get(1))
            .and_then(|m| {
                let stars_str = m.as_str().replace(",", "");
                stars_str.parse::<u32>().ok()
            })
            .unwrap_or(0);

        // Check if archived
        let is_archived = remaining.contains("*Archived!*");

        // Extract description (remove license, stars, archived markers)
        let mut description = remaining.to_string();

        // Remove license
        if let Some(license_match) = self.license_regex.find(&description) {
            description = description[..license_match.start()].trim().to_string();
        }

        // Remove stars
        if let Some(stars_match) = self.stars_regex.find(&description) {
            description = description[..stars_match.start()].trim().to_string();
        }

        // Detect platform migration
        let (platforms, migration_detected) = self.detect_migration(&url, &description);

        // Generate ID from URL
        let id = self.generate_id(&url);

        // Split full_name into owner and name
        let (owner, name) = self.split_full_name(&full_name);

        Ok(Repository {
            id,
            platforms,
            metadata: RepositoryMetadata {
                name,
                owner,
                full_name,
                description: description.trim().to_string(),
                primary_language: language.to_string(),
                license: license.clone(),
                license_spdx: None, // Could be enriched later
                stars,
                topics: vec![], // Not available in LIST.md
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![], // To be enriched
                readme_sections: vec![],
                web_reference_topics: vec![],
                language_category: language.to_string(),
                language_notes: None,
                readme_inclusion: stars > 2000, // Auto-detect based on stars
                readme_inclusion_reason: if stars > 2000 {
                    Some("star_threshold".to_string())
                } else {
                    None
                },
                significance_notes: None,
            },
            quality_metrics: QualityMetrics {
                archive_status: is_archived,
                archive_date: if is_archived {
                    Some(chrono::Utc::now().format("%Y-%m-%d").to_string())
                } else {
                    None
                },
                last_commit_date: None, // Not available in LIST.md
                last_star_update: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                quality_score: QualityMetrics::calculate_score(
                    stars,
                    license.is_some(),
                    !description.trim().is_empty(),
                    false, // topics not available in LIST.md
                    is_archived,
                ),
            },
            source: RepositorySource::GitHubStars,
            added_date: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
            manually_curated: false,
            curator_notes: if migration_detected {
                Some("Platform migration detected from description".to_string())
            } else {
                None
            },
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        })
    }

    /// Detect platform migration from description
    fn detect_migration(&self, url: &str, description: &str) -> (Vec<PlatformInfo>, bool) {
        let desc_lower = description.to_lowercase();
        let mut platforms = vec![];
        let mut migration_detected = false;

        // Determine primary platform from URL
        let primary_platform = if url.contains("github.com") {
            Platform::GitHub
        } else if url.contains("codeberg.org") {
            Platform::Codeberg
        } else if url.contains("gitlab.com") {
            Platform::GitLab
        } else {
            Platform::GitHub // Default
        };

        // Check for migration keywords
        if desc_lower.contains("moved to codeberg")
            || desc_lower.contains("codeberg") && desc_lower.contains("mirror")
        {
            migration_detected = true;

            // If currently on GitHub, might have migrated to Codeberg
            if primary_platform == Platform::GitHub {
                // Original GitHub (possibly archived or mirror)
                platforms.push(PlatformInfo {
                    platform: Platform::GitHub,
                    url: url.to_string(),
                    status: if desc_lower.contains("mirror") {
                        PlatformStatus::Active
                    } else {
                        PlatformStatus::Archived
                    },
                    is_primary: desc_lower.contains("mirror"), // Mirror means still primary
                    migration_date: None,
                    last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    notes: Some("Migration detected from description".to_string()),
                });

                // Note: Actual Codeberg URL would need to be researched or extracted
                // For now, we flag it for manual review
            } else {
                // Already on Codeberg
                platforms.push(PlatformInfo {
                    platform: primary_platform,
                    url: url.to_string(),
                    status: PlatformStatus::Active,
                    is_primary: true,
                    migration_date: None,
                    last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                    notes: None,
                });
            }
        } else {
            // Single platform, no migration
            platforms.push(PlatformInfo {
                platform: primary_platform,
                url: url.to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                notes: None,
            });
        }

        (platforms, migration_detected)
    }

    /// Generate unique ID from URL
    fn generate_id(&self, url: &str) -> String {
        // Convert URL to ID: https://github.com/owner/repo -> github-com-owner-repo
        url.replace("https://", "")
            .replace("http://", "")
            .replace("/", "-")
            .replace(".", "-")
    }

    /// Split full_name into owner and name
    fn split_full_name(&self, full_name: &str) -> (String, String) {
        if let Some(pos) = full_name.find('/') {
            (
                full_name[..pos].to_string(),
                full_name[pos + 1..].to_string(),
            )
        } else {
            ("".to_string(), full_name.to_string())
        }
    }

}

impl Default for ListParser {
    fn default() -> Self {
        Self::new().expect("Failed to create ListParser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_entry() {
        let parser = ListParser::new().unwrap();
        let line = "- [blade-lang/blade](https://github.com/blade-lang/blade) - A modern general-purpose programming language focused on enterprise Web, IoT, and secure application development. \\[*BSD 2-Clause \"Simplified\" License*\\] (⭐️192)";

        let repo = parser.parse_entry(line, "Blade").unwrap();

        assert_eq!(repo.metadata.full_name, "blade-lang/blade");
        assert_eq!(repo.metadata.owner, "blade-lang");
        assert_eq!(repo.metadata.name, "blade");
        assert_eq!(repo.metadata.stars, 192);
        assert_eq!(
            repo.metadata.license,
            Some("BSD 2-Clause \"Simplified\" License".to_string())
        );
        assert!(!repo.quality_metrics.archive_status);
        assert_eq!(repo.classification.language_category, "Blade");
    }

    #[test]
    fn test_parse_archived_entry() {
        let parser = ListParser::new().unwrap();
        let line = "- [lucas-santoni/mandala-c-sdl2](https://github.com/lucas-santoni/mandala-c-sdl2) - Using the C language, the SDL2 library and some modulo operations to draw cool mandalas. (⭐️4) *Archived!*";

        let repo = parser.parse_entry(line, "C").unwrap();

        assert_eq!(repo.metadata.full_name, "lucas-santoni/mandala-c-sdl2");
        assert_eq!(repo.metadata.stars, 4);
        assert!(repo.quality_metrics.archive_status);
        assert_eq!(repo.metadata.license, None);
    }

    #[test]
    fn test_parse_migration_entry() {
        let parser = ListParser::new().unwrap();
        let line = "- [technomancy/leiningen](https://github.com/technomancy/leiningen) - Moved to Codeberg; this is a convenience mirror (⭐️7301)";

        let repo = parser.parse_entry(line, "Clojure").unwrap();

        assert_eq!(repo.metadata.full_name, "technomancy/leiningen");
        assert_eq!(repo.metadata.stars, 7301);
        assert!(repo.metadata.description.contains("Moved to Codeberg"));
        // Should detect migration
        assert!(repo.curator_notes.is_some());
    }

    #[test]
    fn test_quality_score_calculation() {
        // Low stars, not archived (no license/description/topics)
        assert_eq!(QualityMetrics::calculate_score(5, false, false, false, false), 0);

        // Medium stars with description
        assert_eq!(QualityMetrics::calculate_score(500, false, true, false, false), 28);

        // High stars, not archived
        assert_eq!(QualityMetrics::calculate_score(10000, false, false, false, false), 75);

        // High stars but archived
        assert_eq!(QualityMetrics::calculate_score(10000, false, false, false, true), 55);
    }

    #[test]
    fn test_id_generation() {
        let parser = ListParser::new().unwrap();

        let id = parser.generate_id("https://github.com/owner/repo");
        assert_eq!(id, "github-com-owner-repo");

        let id2 = parser.generate_id("https://codeberg.org/user/project");
        assert_eq!(id2, "codeberg-org-user-project");
    }
}
