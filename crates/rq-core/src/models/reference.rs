//! Web reference and documentation link models

use serde::{Deserialize, Serialize};

/// Type of reference content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Article,
    Documentation,
    Book,
    Video,
    Catalog,
    Tutorial,
    Paper,
}

/// Difficulty level of content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DifficultyLevel {
    Introductory,
    Intermediate,
    Advanced,
}

/// Status of a web reference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReferenceStatus {
    Active,
    Proposed,
    Deprecated,
}

/// Web reference entry for documentation and learning resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebReference {
    pub id: String,
    pub title: String,
    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    pub category: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,

    pub content_type: ContentType,
    pub difficulty: DifficultyLevel,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified: Option<String>,

    /// Related web references (by ID)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related_references: Vec<String>,

    /// Related repository IDs
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub related_repos: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,

    #[serde(default = "default_status")]
    pub status: ReferenceStatus,
}

fn default_status() -> ReferenceStatus {
    ReferenceStatus::Active
}

impl WebReference {
    /// Check if reference is stale (>2 years since last verified)
    pub fn is_stale(&self) -> bool {
        if let Some(last_verified) = &self.last_verified {
            if let Ok(verified_date) = chrono::NaiveDate::parse_from_str(last_verified, "%Y-%m-%d")
            {
                let threshold = chrono::Utc::now().date_naive() - chrono::Days::new(730); // 2 years
                return verified_date < threshold;
            }
        }
        // If never verified or invalid date, consider stale
        true
    }

    /// Check if reference is AI/ML related
    pub fn is_aiml_related(&self) -> bool {
        self.category.to_lowercase().contains("ai")
            || self.category.to_lowercase().contains("ml")
            || self.tags.iter().any(|t| {
                let t = t.to_lowercase();
                t.contains("ai")
                    || t.contains("ml")
                    || t.contains("llm")
                    || t.contains("machine-learning")
                    || t.contains("deep-learning")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stale_reference_detection() {
        let ref_stale = WebReference {
            id: "test-ref".to_string(),
            title: "Old Article".to_string(),
            url: "https://example.com".to_string(),
            author: None,
            category: "Test".to_string(),
            subcategory: None,
            content_type: ContentType::Article,
            difficulty: DifficultyLevel::Intermediate,
            publication_date: None,
            last_verified: Some("2020-01-01".to_string()),
            related_references: vec![],
            related_repos: vec![],
            tags: vec![],
            status: ReferenceStatus::Active,
        };

        assert!(ref_stale.is_stale());
    }

    #[test]
    fn test_aiml_detection_by_category() {
        let ref_aiml = WebReference {
            id: "test-ref".to_string(),
            title: "AI Article".to_string(),
            url: "https://example.com".to_string(),
            author: None,
            category: "AI/ML".to_string(),
            subcategory: None,
            content_type: ContentType::Article,
            difficulty: DifficultyLevel::Intermediate,
            publication_date: None,
            last_verified: Some("2024-01-01".to_string()),
            related_references: vec![],
            related_repos: vec![],
            tags: vec![],
            status: ReferenceStatus::Active,
        };

        assert!(ref_aiml.is_aiml_related());
    }

    #[test]
    fn test_aiml_detection_by_tags() {
        let ref_aiml = WebReference {
            id: "test-ref".to_string(),
            title: "ML Article".to_string(),
            url: "https://example.com".to_string(),
            author: None,
            category: "General".to_string(),
            subcategory: None,
            content_type: ContentType::Article,
            difficulty: DifficultyLevel::Intermediate,
            publication_date: None,
            last_verified: Some("2024-01-01".to_string()),
            related_references: vec![],
            related_repos: vec![],
            tags: vec!["machine-learning".to_string(), "deep-learning".to_string()],
            status: ReferenceStatus::Active,
        };

        assert!(ref_aiml.is_aiml_related());
    }
}
