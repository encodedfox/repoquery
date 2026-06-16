//! Cross-reference graph for bidirectional linking

use crate::GraphError;
use od_core::CanonicalData;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Node type in the cross-reference graph
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeType {
    Repository(String),    // repo ID
    WebReference(String),  // reference ID
    Book(String),          // book ID
    ReadmeSection(String), // section name
}

/// Edge type representing relationship
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationType {
    Implements,   // Repo implements concept
    References,   // Section references repo
    Alternative,  // Alternative implementation
    Prerequisite, // Required before
}

/// Cross-reference graph
pub struct CrossRefGraph {
    graph: DiGraph<NodeType, RelationType>,
    node_map: HashMap<NodeType, NodeIndex>,
}

impl CrossRefGraph {
    /// Create new empty graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Build graph from canonical data
    pub fn build(data: &CanonicalData) -> Result<Self, GraphError> {
        let mut graph = Self::new();

        // Add repository nodes
        for repo in &data.repositories {
            graph.add_node(NodeType::Repository(repo.id.clone()));
        }

        // Add web reference nodes and links to repos
        for reference in &data.web_references {
            let ref_node = NodeType::WebReference(reference.id.clone());
            graph.add_node(ref_node.clone());

            // Link to related repositories
            for repo_ref in &reference.related_repos {
                // Find repository by full_name or id
                if let Some(repo) = data
                    .repositories
                    .iter()
                    .find(|r| r.metadata.full_name == *repo_ref || r.id == *repo_ref)
                {
                    graph.add_edge(
                        ref_node.clone(),
                        NodeType::Repository(repo.id.clone()),
                        RelationType::References,
                    );
                }
            }

            // Create README section nodes for web ref categories
            let section_node =
                NodeType::ReadmeSection(format!("Web References/{}", reference.category));
            graph.add_node(section_node.clone());
            graph.add_edge(section_node, ref_node, RelationType::References);
        }

        // Add book nodes and links
        for book in &data.books {
            let book_node = NodeType::Book(book.id.clone());
            graph.add_node(book_node.clone());

            // Link to related repositories
            for repo_ref in &book.related_repos {
                if let Some(repo) = data
                    .repositories
                    .iter()
                    .find(|r| r.metadata.full_name == *repo_ref || r.id == *repo_ref)
                {
                    graph.add_edge(
                        book_node.clone(),
                        NodeType::Repository(repo.id.clone()),
                        RelationType::References,
                    );
                }
            }

            // Create README section node for books
            let section_node = NodeType::ReadmeSection("Books".to_string());
            graph.add_node(section_node.clone());
            graph.add_edge(section_node, book_node, RelationType::References);
        }

        // Add README section nodes for repos that meet inclusion criteria
        for repo in &data.repositories {
            if repo.meets_readme_criteria() {
                for section in &repo.classification.readme_sections {
                    let section_node = NodeType::ReadmeSection(section.clone());
                    graph.add_node(section_node.clone());
                    graph.add_edge(
                        section_node,
                        NodeType::Repository(repo.id.clone()),
                        RelationType::References,
                    );
                }
            }
        }

        Ok(graph)
    }

    /// Add a node to the graph
    fn add_node(&mut self, node: NodeType) {
        if !self.node_map.contains_key(&node) {
            let idx = self.graph.add_node(node.clone());
            self.node_map.insert(node, idx);
        }
    }

    /// Add an edge between nodes
    fn add_edge(&mut self, from: NodeType, to: NodeType, rel: RelationType) {
        let from_idx = self.node_map[&from];
        let to_idx = self.node_map[&to];
        self.graph.add_edge(from_idx, to_idx, rel);
    }

    /// Get all repositories referenced by a README section
    pub fn repos_for_section(&self, section: &str) -> Vec<String> {
        let section_node = NodeType::ReadmeSection(section.to_string());

        if let Some(&node_idx) = self.node_map.get(&section_node) {
            let mut repos = Vec::new();

            // Find all outgoing edges
            use petgraph::visit::EdgeRef;
            for edge in self.graph.edges(node_idx) {
                if let NodeType::Repository(repo_id) = &self.graph[edge.target()] {
                    repos.push(repo_id.clone());
                }
            }

            repos
        } else {
            vec![]
        }
    }

    /// Get all README sections that reference a repository
    pub fn sections_for_repo(&self, repo_id: &str) -> Vec<String> {
        let repo_node = NodeType::Repository(repo_id.to_string());

        if let Some(&node_idx) = self.node_map.get(&repo_node) {
            let mut sections = Vec::new();

            // Find all incoming edges from README sections
            use petgraph::visit::EdgeRef;
            for edge in self
                .graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
            {
                if let NodeType::ReadmeSection(section) = &self.graph[edge.source()] {
                    sections.push(section.clone());
                }
            }

            sections
        } else {
            vec![]
        }
    }

    /// Get statistics about the graph
    pub fn stats(&self) -> CrossRefStats {
        let mut repo_count = 0;
        let mut web_ref_count = 0;
        let mut book_count = 0;
        let mut section_count = 0;

        for node in self.graph.node_weights() {
            match node {
                NodeType::Repository(_) => repo_count += 1,
                NodeType::WebReference(_) => web_ref_count += 1,
                NodeType::Book(_) => book_count += 1,
                NodeType::ReadmeSection(_) => section_count += 1,
            }
        }

        CrossRefStats {
            total_nodes: self.graph.node_count(),
            total_edges: self.graph.edge_count(),
            repositories: repo_count,
            web_references: web_ref_count,
            books: book_count,
            readme_sections: section_count,
        }
    }
}

/// Statistics about cross-reference graph
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrossRefStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub repositories: usize,
    pub web_references: usize,
    pub books: usize,
    pub readme_sections: usize,
}

impl Default for CrossRefGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use od_core::{
        CanonicalData, Platform, PlatformInfo, PlatformStatus, QualityMetrics,
        RepositoryClassification, RepositoryMetadata, RepositorySource,
    };

    fn test_repo(id: &str, section: Option<&str>) -> od_core::Repository {
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
                description: String::new(),
                primary_language: "Rust".to_string(),
                license: None,
                license_spdx: None,
                stars: 100,
                topics: vec![],
                homepage: None,
                language_breakdown: None,
                secondary_languages: vec![],
            },
            classification: RepositoryClassification {
                categories: vec![],
                readme_sections: section.map(|s| vec![s.to_string()]).unwrap_or_default(),
                web_reference_topics: vec![],
                language_category: "Rust".to_string(),
                language_notes: None,
                readme_inclusion: section.is_some(),
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

    #[test]
    fn test_graph_creation() {
        let graph = CrossRefGraph::new();
        let stats = graph.stats();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_edges, 0);
    }

    #[test]
    fn test_node_addition() {
        let mut graph = CrossRefGraph::new();
        graph.add_node(NodeType::Repository("repo1".to_string()));
        graph.add_node(NodeType::WebReference("ref1".to_string()));
        let stats = graph.stats();
        assert_eq!(stats.repositories, 1);
        assert_eq!(stats.web_references, 1);
    }

    #[test]
    fn test_build_graph_from_repos() {
        let mut data = CanonicalData::new();
        data.repositories = vec![
            test_repo("owner/a", Some("Databases")),
            test_repo("owner/b", Some("Databases")),
            test_repo("owner/c", None),
        ];
        let graph = CrossRefGraph::build(&data).unwrap();
        let stats = graph.stats();
        assert_eq!(stats.repositories, 3);
        // Both repos share the "Databases" section node → 1 section node, 2 edges
        assert_eq!(stats.readme_sections, 1);
        assert!(stats.total_edges >= 2);
    }

    #[test]
    fn test_navigator_related_repos() {
        use crate::Navigator;
        let mut data = CanonicalData::new();
        data.repositories = vec![
            test_repo("owner/a", Some("Networking")),
            test_repo("owner/b", Some("Networking")),
        ];
        let nav = Navigator::new(&data).unwrap();
        // build_navigation_index uses web_references/books; for readme sections
        // we verify via stats that the graph was built with section nodes
        let stats = nav.stats();
        assert_eq!(stats.repositories, 2);
        assert_eq!(stats.readme_sections, 1, "both repos share one section node");
    }

    #[test]
    fn test_empty_graph() {
        let data = CanonicalData::new();
        let graph = CrossRefGraph::build(&data).unwrap();
        let stats = graph.stats();
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_edges, 0);
    }
}
