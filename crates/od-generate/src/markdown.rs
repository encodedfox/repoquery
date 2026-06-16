//! Markdown document generator using Tera templates

use crate::GenerateError;
use od_core::{CanonicalData, Repository};
use std::collections::HashMap;
use tera::{Context, Tera};

/// Markdown document generator
pub struct MarkdownGenerator {
    tera: Tera,
}

/// Repository data for template rendering
#[derive(serde::Serialize)]
struct TemplateRepo {
    full_name: String,
    url: String,
    description: String,
    license: Option<String>,
    stars: u32,
    is_archived: bool,
    has_migration: bool,
    migration_platform: Option<String>,
    migration_url: Option<String>,
}

/// Language section for template
#[derive(serde::Serialize)]
struct LanguageSection {
    name: String,
    anchor: String,
    repos: Vec<TemplateRepo>,
}

/// Platform statistics for template
#[derive(serde::Serialize)]
struct PlatformStat {
    name: String,
    count: usize,
    percentage: String,
}

impl MarkdownGenerator {
    /// Create new generator with templates from directory
    pub fn new(template_dir: &str) -> Result<Self, GenerateError> {
        let pattern = format!("{}/*.tera", template_dir);
        let tera = Tera::new(&pattern)?;
        Ok(Self { tera })
    }

    /// Generate LIST.md format
    pub fn generate_list(
        &self,
        data: &CanonicalData,
        include_stats: bool,
    ) -> Result<String, GenerateError> {
        let context = self.build_context(data, include_stats)?;
        let output = self.tera.render("list.md.tera", &context)?;
        Ok(output)
    }

    /// Generate TABLE.md format
    pub fn generate_table(
        &self,
        data: &CanonicalData,
        include_stats: bool,
    ) -> Result<String, GenerateError> {
        let context = self.build_context(data, include_stats)?;
        let output = self.tera.render("table.md.tera", &context)?;
        Ok(output)
    }

    /// Generate a LIST for a named collection (subset of repos).
    pub fn generate_list_for_collection(
        &self,
        repos: &[Repository],
        collection_name: &str,
        collection_description: Option<&str>,
        include_stats: bool,
    ) -> Result<String, GenerateError> {
        let context = self.build_collection_context(repos, collection_name, collection_description, include_stats)?;
        let output = self.tera.render("list.md.tera", &context)?;
        Ok(output)
    }

    /// Generate a TABLE for a named collection (subset of repos).
    pub fn generate_table_for_collection(
        &self,
        repos: &[Repository],
        collection_name: &str,
        collection_description: Option<&str>,
        include_stats: bool,
    ) -> Result<String, GenerateError> {
        let context = self.build_collection_context(repos, collection_name, collection_description, include_stats)?;
        let output = self.tera.render("table.md.tera", &context)?;
        Ok(output)
    }

    /// Build template context from canonical data
    fn build_context(&self, data: &CanonicalData, include_stats: bool) -> Result<Context, GenerateError> {
        let mut context = Context::new();

        // Get all repositories (active or archived based on use)
        let all_repos = data.all_repositories();

        // Group by language
        let mut by_language: HashMap<String, Vec<&Repository>> = HashMap::new();
        for repo in &all_repos {
            by_language
                .entry(repo.classification.language_category.clone())
                .or_default()
                .push(repo);
        }

        // Sort languages alphabetically
        let mut lang_names: Vec<String> = by_language.keys().cloned().collect();
        lang_names.sort();

        // Build language sections
        let mut language_sections = Vec::new();
        for lang_name in &lang_names {
            let repos = by_language.get(lang_name).unwrap();

            let template_repos: Vec<TemplateRepo> = repos
                .iter()
                .map(|r| {
                    let migration = r.migration_record();
                    let alt_urls = migration.alternative_urls();

                    TemplateRepo {
                        full_name: r.metadata.full_name.clone(),
                        url: r.primary_url().unwrap_or("").to_string(),
                        description: r.metadata.description.clone(),
                        license: r.metadata.license.clone(),
                        stars: r.metadata.stars,
                        is_archived: r.quality_metrics.archive_status,
                        has_migration: !alt_urls.is_empty(),
                        migration_platform: alt_urls.first().map(|(p, _)| format!("{}", p)),
                        migration_url: alt_urls.first().map(|(_, u)| u.to_string()),
                    }
                })
                .collect();

            language_sections.push(LanguageSection {
                name: lang_name.clone(),
                anchor: lang_name.to_lowercase().replace("#", "").replace(" ", "-"),
                repos: template_repos,
            });
        }

        context.insert("total_count", &all_repos.len());
        context.insert("languages", &language_sections);
        context.insert("include_stats", &include_stats);

        if include_stats {
            if let Some(stats) = &data.statistics {
                // Platform stats
                let mut platform_stats: Vec<PlatformStat> = stats
                    .by_platform
                    .iter()
                    .map(|(name, count)| PlatformStat {
                        name: name.clone(),
                        count: *count,
                        percentage: format!("{:.1}", (*count as f32 / stats.total as f32) * 100.0),
                    })
                    .collect();
                platform_stats.sort_by(|a, b| b.count.cmp(&a.count));

                context.insert("platforms", &platform_stats);

                // Status stats
                #[derive(serde::Serialize)]
                struct StatusStats {
                    active: usize,
                    archived: usize,
                    active_pct: String,
                    archived_pct: String,
                }

                let status_stats = StatusStats {
                    active: stats.by_status.active,
                    archived: stats.by_status.archived,
                    active_pct: format!(
                        "{:.1}",
                        (stats.by_status.active as f32 / stats.total as f32) * 100.0
                    ),
                    archived_pct: format!(
                        "{:.1}",
                        (stats.by_status.archived as f32 / stats.total as f32) * 100.0
                    ),
                };

                context.insert("stats", &status_stats);

                context.insert("timestamp", &data.last_updated);
            }
        }

        Ok(context)
    }

    /// Build template context from a slice of repos with a collection title.
    fn build_collection_context(
        &self,
        repos: &[Repository],
        collection_name: &str,
        collection_description: Option<&str>,
        include_stats: bool,
    ) -> Result<Context, GenerateError> {
        let mut by_language: HashMap<String, Vec<&Repository>> = HashMap::new();
        for repo in repos {
            by_language
                .entry(repo.classification.language_category.clone())
                .or_default()
                .push(repo);
        }

        let mut lang_names: Vec<String> = by_language.keys().cloned().collect();
        lang_names.sort();

        let mut language_sections = Vec::new();
        for lang_name in &lang_names {
            let lang_repos = by_language.get(lang_name).unwrap();
            let template_repos: Vec<TemplateRepo> = lang_repos
                .iter()
                .map(|r| {
                    let migration = r.migration_record();
                    let alt_urls = migration.alternative_urls();
                    TemplateRepo {
                        full_name: r.metadata.full_name.clone(),
                        url: r.primary_url().unwrap_or("").to_string(),
                        description: r.metadata.description.clone(),
                        license: r.metadata.license.clone(),
                        stars: r.metadata.stars,
                        is_archived: r.quality_metrics.archive_status,
                        has_migration: !alt_urls.is_empty(),
                        migration_platform: alt_urls.first().map(|(p, _)| format!("{}", p)),
                        migration_url: alt_urls.first().map(|(_, u)| u.to_string()),
                    }
                })
                .collect();
            language_sections.push(LanguageSection {
                name: lang_name.clone(),
                anchor: lang_name.to_lowercase().replace('#', "").replace(' ', "-"),
                repos: template_repos,
            });
        }

        let mut context = Context::new();
        context.insert("total_count", &repos.len());
        context.insert("languages", &language_sections);
        context.insert("include_stats", &include_stats);
        context.insert("collection_name", &collection_name);
        context.insert("collection_description", &collection_description);
        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::{
        CanonicalData, Platform, PlatformInfo, PlatformStatus, QualityMetrics,
        RepositoryClassification, RepositoryMetadata, RepositorySource,
    };

    fn test_repo(id: &str, lang: &str, stars: u32) -> od_core::Repository {
        od_core::Repository {
            id: id.to_string(),
            platforms: vec![PlatformInfo {
                platform: Platform::GitHub,
                url: format!("https://github.com/{}", id),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            }],
            metadata: RepositoryMetadata {
                name: id.split('/').last().unwrap_or(id).to_string(),
                owner: id.split('/').next().unwrap_or("owner").to_string(),
                full_name: id.to_string(),
                description: format!("Desc {}", id),
                primary_language: lang.to_string(),
                license: Some("MIT".to_string()),
                license_spdx: Some("MIT".to_string()),
                stars,
                topics: vec![],
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
                last_commit_date: None,
                last_star_update: "2024-12-10".to_string(),
                quality_score: 70,
            },
            source: RepositorySource::GitHubStars,
            added_date: None,
            manually_curated: false,
            curator_notes: None,
            relations: vec![],
                    fork_parent: None,
            fork_parent_url: None,
            custom_tags: vec![],
            fork_ahead: None,
            fork_behind: None,
        }
    }

    /// Build a MarkdownGenerator with minimal inline templates (no filesystem dependency).
    fn inline_generator() -> MarkdownGenerator {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "list.md.tera",
            "{% for lang in languages %}## {{ lang.name }}\n{% for repo in lang.repos %}- [{{ repo.full_name }}]({{ repo.url }}) ⭐️{{ repo.stars }} {{ repo.license | default(value=\"\") }}\n{% endfor %}{% endfor %}",
        )
        .unwrap();
        tera.add_raw_template(
            "table.md.tera",
            "{% for lang in languages %}## {{ lang.name }}\n| Name | Stars |\n{% for repo in lang.repos %}| {{ repo.full_name }} | ⭐️{{ repo.stars }} |\n{% endfor %}{% endfor %}",
        )
        .unwrap();
        MarkdownGenerator { tera }
    }

    fn active_data() -> CanonicalData {
        let mut data = CanonicalData::new();
        data.repositories = vec![
            test_repo("owner/alpha", "Rust", 1000),
            test_repo("owner/beta", "Go", 500),
        ];
        data.calculate_statistics();
        data
    }

    #[test]
    fn test_generate_list_markdown() {
        let gen = inline_generator();
        let data = active_data();
        let output = gen.generate_list(&data, false).expect("generate_list failed");
        assert!(output.contains("owner/alpha"), "missing alpha");
        assert!(output.contains("owner/beta"), "missing beta");
        assert!(output.contains("⭐️1000"));
        assert!(output.contains("MIT"));
    }

    #[test]
    fn test_generate_table_markdown() {
        let gen = inline_generator();
        let data = active_data();
        let output = gen.generate_table(&data, false).expect("generate_table failed");
        assert!(output.contains("owner/alpha"));
        assert!(output.contains("⭐️500"));
        assert!(output.contains("| Name | Stars |"));
    }

    #[test]
    fn test_empty_data_generates_valid_output() {
        let gen = inline_generator();
        let data = CanonicalData::new();
        assert!(gen.generate_list(&data, false).is_ok());
        assert!(gen.generate_table(&data, false).is_ok());
    }

    #[test]
    fn test_generate_list_for_collection() {
        let gen = inline_generator();
        let repos = vec![
            test_repo("owner/alpha", "Rust", 1000),
            test_repo("owner/gamma", "Rust", 200),
        ];
        let output = gen
            .generate_list_for_collection(&repos, "My Rust Collection", Some("Best Rust repos"), false)
            .expect("generate_list_for_collection failed");
        assert!(output.contains("owner/alpha"), "missing alpha");
        assert!(output.contains("owner/gamma"), "missing gamma");
        assert!(output.contains("⭐️1000"));
    }

    #[test]
    fn test_generate_table_for_collection() {
        let gen = inline_generator();
        let repos = vec![test_repo("owner/beta", "Go", 500)];
        let output = gen
            .generate_table_for_collection(&repos, "Go Collection", None, false)
            .expect("generate_table_for_collection failed");
        assert!(output.contains("owner/beta"));
        assert!(output.contains("⭐️500"));
    }
}
