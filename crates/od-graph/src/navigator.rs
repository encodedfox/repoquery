//! Navigation link generator for README cross-references

use super::graph::CrossRefGraph;
use od_core::CanonicalData;
use std::collections::HashMap;

/// Navigation links generator
pub struct Navigator {
    graph: CrossRefGraph,
}

impl Navigator {
    /// Create navigator from canonical data
    pub fn new(data: &CanonicalData) -> Result<Self, crate::GraphError> {
        let graph = CrossRefGraph::build(data)?;
        Ok(Self { graph })
    }

    /// Generate "Related Repositories" section for a web reference category
    pub fn related_repos_for_category(&self, data: &CanonicalData, category: &str) -> String {
        let section_name = format!("Web References/{}", category);
        let repo_ids = self.graph.repos_for_section(&section_name);

        if repo_ids.is_empty() {
            return String::new();
        }

        // Group repositories by language
        let mut by_language: HashMap<String, Vec<String>> = HashMap::new();

        for repo_id in &repo_ids {
            if let Some(repo) = data.find_repo_by_id(repo_id) {
                by_language
                    .entry(repo.classification.language_category.clone())
                    .or_default()
                    .push(repo.metadata.full_name.clone());
            }
        }

        // Sort languages
        let mut langs: Vec<_> = by_language.keys().cloned().collect();
        langs.sort();

        // Format as markdown
        let mut output = String::from("*Related repositories: ");
        let lang_links: Vec<String> = langs
            .iter()
            .map(|lang| {
                let repos = &by_language[lang];
                let repo_links: Vec<String> = repos
                    .iter()
                    .map(|r| format!("[{}](LIST.md#{})", r, lang.to_lowercase().replace(" ", "-")))
                    .collect();
                format!(
                    "[{}](LIST.md#{}) - {}",
                    lang,
                    lang.to_lowercase().replace(" ", "-"),
                    repo_links.join(", ")
                )
            })
            .collect();

        output.push_str(&lang_links.join("; "));
        output.push('*');

        output
    }

    /// Generate cross-reference notation for a repository
    pub fn cross_ref_for_repo(&self, repo_id: &str) -> Option<String> {
        let sections = self.graph.sections_for_repo(repo_id);

        if sections.is_empty() {
            return None;
        }

        let section_links: Vec<String> = sections
            .iter()
            .map(|s| {
                format!(
                    "[{}](README.md#{})",
                    s,
                    s.to_lowercase().replace(" ", "-").replace("/", "")
                )
            })
            .collect();

        Some(format!("📑 Referenced in: {}", section_links.join(", ")))
    }

    /// Generate navigation index mapping repos to README sections
    pub fn build_navigation_index(&self, data: &CanonicalData) -> NavigationIndex {
        let mut readme_to_repos: HashMap<String, Vec<String>> = HashMap::new();
        let mut repo_to_readme: HashMap<String, Vec<String>> = HashMap::new();

        // Build from web references
        for reference in &data.web_references {
            let section = format!("Web References/{}", reference.category);

            for repo_ref in &reference.related_repos {
                if let Some(repo) = data
                    .repositories
                    .iter()
                    .find(|r| r.metadata.full_name == *repo_ref || r.id == *repo_ref)
                {
                    readme_to_repos
                        .entry(section.clone())
                        .or_default()
                        .push(repo.id.clone());

                    repo_to_readme
                        .entry(repo.id.clone())
                        .or_default()
                        .push(section.clone());
                }
            }
        }

        // Build from books
        for book in &data.books {
            let section = "Books".to_string();

            for repo_ref in &book.related_repos {
                if let Some(repo) = data
                    .repositories
                    .iter()
                    .find(|r| r.metadata.full_name == *repo_ref || r.id == *repo_ref)
                {
                    readme_to_repos
                        .entry(section.clone())
                        .or_default()
                        .push(repo.id.clone());

                    repo_to_readme
                        .entry(repo.id.clone())
                        .or_default()
                        .push(section.clone());
                }
            }
        }

        NavigationIndex {
            readme_to_repos,
            repo_to_readme,
        }
    }

    /// Get graph statistics
    pub fn stats(&self) -> super::graph::CrossRefStats {
        self.graph.stats()
    }
}

/// Navigation index for cross-references
#[derive(Debug, Clone)]
pub struct NavigationIndex {
    /// Map from README section to repository IDs
    pub readme_to_repos: HashMap<String, Vec<String>>,
    /// Map from repository ID to README sections
    pub repo_to_readme: HashMap<String, Vec<String>>,
}

impl NavigationIndex {
    /// Get repositories for a README section
    pub fn repos_for_section(&self, section: &str) -> Vec<&str> {
        self.readme_to_repos
            .get(section)
            .map(|repos| repos.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get README sections for a repository
    pub fn sections_for_repo(&self, repo_id: &str) -> Vec<&str> {
        self.repo_to_readme
            .get(repo_id)
            .map(|sections| sections.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_index_creation() {
        let index = NavigationIndex {
            readme_to_repos: HashMap::new(),
            repo_to_readme: HashMap::new(),
        };

        assert_eq!(index.repos_for_section("test").len(), 0);
    }
}
