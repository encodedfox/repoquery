//! Book metadata and relationship models

use serde::{Deserialize, Serialize};

/// Relationship between books
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BookRelationship {
    /// Books that should be read together
    Companion,
    /// Book that should be read before this one
    Prerequisite,
    /// Newer edition or follow-up
    Successor,
}

/// Related book reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedBook {
    pub id: String,
    pub relationship: BookRelationship,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Book entry for learning resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: String,
    pub title: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,

    pub authors: Vec<String>,
    pub category: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_year: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,

    /// Related books
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related_books: Vec<RelatedBook>,

    /// Topics this book could expand into
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub expansion_topics: Vec<String>,

    /// Related web reference IDs
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related_web_references: Vec<String>,

    /// Related repository IDs
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related_repos: Vec<String>,
}

impl Book {
    /// Get full title including subtitle if present
    pub fn full_title(&self) -> String {
        if let Some(subtitle) = &self.subtitle {
            format!("{}: {}", self.title, subtitle)
        } else {
            self.title.clone()
        }
    }

    /// Get formatted author string
    pub fn authors_string(&self) -> String {
        match self.authors.len() {
            0 => "Unknown".to_string(),
            1 => self.authors[0].clone(),
            2 => format!("{} and {}", self.authors[0], self.authors[1]),
            _ => {
                let last = self.authors.last().unwrap();
                let others = &self.authors[..self.authors.len() - 1];
                format!("{}, and {}", others.join(", "), last)
            }
        }
    }

    /// Check if book has expansion opportunities
    pub fn has_expansion_topics(&self) -> bool {
        !self.expansion_topics.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_title_with_subtitle() {
        let book = Book {
            id: "test-book".to_string(),
            title: "Designing Software Architectures".to_string(),
            subtitle: Some("A Practical Approach".to_string()),
            authors: vec!["Author Name".to_string()],
            category: "Architecture".to_string(),
            subcategory: None,
            isbn: None,
            publication_year: Some(2016),
            edition: None,
            related_books: vec![],
            expansion_topics: vec![],
            related_web_references: vec![],
            related_repos: vec![],
        };

        assert_eq!(
            book.full_title(),
            "Designing Software Architectures: A Practical Approach"
        );
    }

    #[test]
    fn test_authors_string_multiple() {
        let book = Book {
            id: "test-book".to_string(),
            title: "Test Book".to_string(),
            subtitle: None,
            authors: vec![
                "Author One".to_string(),
                "Author Two".to_string(),
                "Author Three".to_string(),
            ],
            category: "Test".to_string(),
            subcategory: None,
            isbn: None,
            publication_year: None,
            edition: None,
            related_books: vec![],
            expansion_topics: vec![],
            related_web_references: vec![],
            related_repos: vec![],
        };

        assert_eq!(
            book.authors_string(),
            "Author One, Author Two, and Author Three"
        );
    }

    #[test]
    fn test_has_expansion_topics() {
        let mut book = Book {
            id: "test-book".to_string(),
            title: "Test Book".to_string(),
            subtitle: None,
            authors: vec!["Author".to_string()],
            category: "Test".to_string(),
            subcategory: None,
            isbn: None,
            publication_year: None,
            edition: None,
            related_books: vec![],
            expansion_topics: vec![],
            related_web_references: vec![],
            related_repos: vec![],
        };

        assert!(!book.has_expansion_topics());

        book.expansion_topics
            .push("distributed-systems".to_string());
        assert!(book.has_expansion_topics());
    }
}
