use serde::{Deserialize, Serialize};

/// User-defined grouping of repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub repo_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Collection {
    pub fn new(id: String, name: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            description: None,
            repo_ids: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn add_repo(&mut self, repo_id: String) {
        if !self.repo_ids.contains(&repo_id) {
            self.repo_ids.push(repo_id);
            self.updated_at = chrono::Utc::now().to_rfc3339();
        }
    }

    pub fn remove_repo(&mut self, repo_id: &str) -> bool {
        let before = self.repo_ids.len();
        self.repo_ids.retain(|id| id != repo_id);
        if self.repo_ids.len() != before {
            self.updated_at = chrono::Utc::now().to_rfc3339();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_crud() {
        let mut c = Collection::new("c1".to_string(), "My List".to_string());
        assert!(c.repo_ids.is_empty());

        c.add_repo("repo-a".to_string());
        c.add_repo("repo-b".to_string());
        c.add_repo("repo-a".to_string()); // duplicate — ignored
        assert_eq!(c.repo_ids.len(), 2);

        assert!(c.remove_repo("repo-a"));
        assert_eq!(c.repo_ids, vec!["repo-b"]);
        assert!(!c.remove_repo("repo-a")); // already gone
    }

    #[test]
    fn test_collection_serde_roundtrip() {
        let mut c = Collection::new("c1".to_string(), "Test".to_string());
        c.description = Some("desc".to_string());
        c.add_repo("r1".to_string());
        let json = serde_json::to_string(&c).unwrap();
        let back: Collection = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "c1");
        assert_eq!(back.repo_ids, vec!["r1"]);
    }
}
