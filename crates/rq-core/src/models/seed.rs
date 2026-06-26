use serde::{Deserialize, Serialize};

/// Platform types supported for seed-based discovery
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformKind {
    GitHub,
    GitLab,
    Codeberg,
    Gitea,
    Bitbucket,
    Sourcehut,
}

impl std::fmt::Display for PlatformKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::GitHub => "github",
                Self::GitLab => "gitlab",
                Self::Codeberg => "codeberg",
                Self::Gitea => "gitea",
                Self::Bitbucket => "bitbucket",
                Self::Sourcehut => "sourcehut",
            }
        )
    }
}

impl std::str::FromStr for PlatformKind {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "github" => Ok(Self::GitHub),
            "gitlab" => Ok(Self::GitLab),
            "codeberg" => Ok(Self::Codeberg),
            "gitea" => Ok(Self::Gitea),
            "bitbucket" => Ok(Self::Bitbucket),
            "sourcehut" => Ok(Self::Sourcehut),
            _ => Err(format!("Unknown platform kind: {}", s)),
        }
    }
}

/// Status of a seed user in the expansion pipeline
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedStatus {
    Pending,
    Active,
    Completed,
    Failed,
}

/// A seed user for graph expansion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    pub id: String,
    pub platform: PlatformKind,
    pub username: String,
    pub status: SeedStatus,
    /// Current traversal depth reached
    pub depth: u32,
    /// Maximum traversal depth allowed
    pub max_depth: u32,
    pub added_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

/// Edge types for traversal relationships
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Follows,
    Stars,
    ContributesTo,
    MemberOf,
    Forked,
}

/// A discovered edge in the expansion graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalEdge {
    pub id: i64,
    pub seed_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub relation_type: EdgeType,
    pub depth: u32,
    pub discovered_at: String,
}

/// Status of a token in the FGAT pool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenStatus {
    Available,
    InUse,
    Exhausted,
    Revoked,
}

/// A Fine-Grained Access Token for API access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FgatToken {
    pub id: String,
    pub platform: PlatformKind,
    pub token_hash: String,
    pub status: TokenStatus,
    pub requests_used: u64,
    pub rate_limit_limit: Option<u32>,
    pub rate_limit_remaining: Option<u32>,
    pub rate_limit_reset_at: Option<String>,
    pub last_used_at: Option<String>,
    pub added_at: String,
    pub expires_at: Option<String>,
    pub notes: Option<String>,
}

/// A normalized domain for cross-platform indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedDomain {
    pub domain: String,
    pub canonical_domain: String,
    pub platform: Option<PlatformKind>,
    pub is_verified: bool,
    pub confidence_score: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// A unified identity across platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedIdentity {
    pub id: String,
    pub canonical_username: String,
    pub primary_domain: String,
    pub confidence_score: f64,
    pub created_at: String,
}

/// A platform-specific alias for a unified identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityAlias {
    pub unified_id: String,
    pub platform: PlatformKind,
    pub username: String,
    pub verified: bool,
    pub verification_method: Option<String>,
}

/// A repository indexed by domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRepository {
    pub id: String,
    pub domain: String,
    pub repo_path: String,
    pub full_name: String,
    pub platform: PlatformKind,
    pub metadata_json: String,
    pub indexed_at: String,
}
