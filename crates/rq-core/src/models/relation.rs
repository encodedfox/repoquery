use serde::{Deserialize, Serialize};

/// How the user relates to a repository
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Starred,
    Owned,
    Forked,
    Watching,
    OrgMember,
    Contributed,
    ManuallyAdded,
}

impl std::fmt::Display for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Relation::Starred => write!(f, "starred"),
            Relation::Owned => write!(f, "owned"),
            Relation::Forked => write!(f, "forked"),
            Relation::Watching => write!(f, "watching"),
            Relation::OrgMember => write!(f, "org_member"),
            Relation::Contributed => write!(f, "contributed"),
            Relation::ManuallyAdded => write!(f, "manually_added"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_serde_roundtrip() {
        let variants = [
            Relation::Starred,
            Relation::Owned,
            Relation::Forked,
            Relation::Watching,
            Relation::OrgMember,
            Relation::Contributed,
            Relation::ManuallyAdded,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let back: Relation = serde_json::from_str(&json).unwrap();
            assert_eq!(v, &back);
        }
    }

    #[test]
    fn test_relation_display() {
        assert_eq!(Relation::OrgMember.to_string(), "org_member");
        assert_eq!(Relation::ManuallyAdded.to_string(), "manually_added");
    }
}
