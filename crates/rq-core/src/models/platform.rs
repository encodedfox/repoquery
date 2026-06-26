//! Platform and migration tracking models

use serde::{Deserialize, Serialize};

/// Supported code hosting platforms
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    #[serde(rename = "github")]
    GitHub,
    #[serde(rename = "codeberg")]
    Codeberg,
    #[serde(rename = "gitlab")]
    GitLab,
    #[serde(rename = "gitea")]
    Gitea,
    #[serde(rename = "aws-codecommit")]
    AwsCodeCommit,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::GitHub => write!(f, "GitHub"),
            Platform::Codeberg => write!(f, "Codeberg"),
            Platform::GitLab => write!(f, "GitLab"),
            Platform::Gitea => write!(f, "Gitea"),
            Platform::AwsCodeCommit => write!(f, "AWS CodeCommit"),
        }
    }
}

/// Repository status on a platform
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformStatus {
    Active,
    Archived,
    Deprecated,
}

/// Information about a repository on a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub url: String,
    pub status: PlatformStatus,
    pub is_primary: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Migration status for repositories hosted on multiple platforms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    /// Repository exists on single platform only
    None,
    /// Active on multiple platforms (mirror)
    Mirror,
    /// Moved from one platform to another
    Migrated,
    /// Original platform archived, moved to new platform
    ArchivedMigrated,
}

/// Historical record of platform migrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationHistory {
    pub date: String,
    pub from: Platform,
    pub to: Platform,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Complete migration tracking record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub repo_id: String,
    pub migration_status: MigrationStatus,
    pub platforms: Vec<PlatformInfo>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub migration_history: Vec<MigrationHistory>,
}

impl MigrationRecord {
    /// Detect migration status from platform information
    pub fn detect_status(platforms: &[PlatformInfo]) -> MigrationStatus {
        let active_count = platforms
            .iter()
            .filter(|p| p.status == PlatformStatus::Active)
            .count();

        let archived_count = platforms
            .iter()
            .filter(|p| p.status == PlatformStatus::Archived)
            .count();

        match (platforms.len(), active_count, archived_count) {
            (1, _, _) => MigrationStatus::None,
            (n, a, _) if n > 1 && a == n => MigrationStatus::Mirror,
            (n, a, ar) if n > 1 && a > 0 && ar > 0 => MigrationStatus::ArchivedMigrated,
            (n, a, _) if n > 1 && a > 0 => MigrationStatus::Migrated,
            _ => MigrationStatus::None,
        }
    }

    /// Get the primary platform URL
    pub fn primary_url(&self) -> Option<&str> {
        self.platforms
            .iter()
            .find(|p| p.is_primary)
            .map(|p| p.url.as_str())
    }

    /// Get all alternative platform URLs
    pub fn alternative_urls(&self) -> Vec<(&Platform, &str)> {
        self.platforms
            .iter()
            .filter(|p| !p.is_primary)
            .map(|p| (&p.platform, p.url.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_status_detection_single_platform() {
        let platforms = vec![PlatformInfo {
            platform: Platform::GitHub,
            url: "https://github.com/test/repo".to_string(),
            status: PlatformStatus::Active,
            is_primary: true,
            migration_date: None,
            last_verified: None,
            notes: None,
        }];

        let status = MigrationRecord::detect_status(&platforms);
        assert_eq!(status, MigrationStatus::None);
    }

    #[test]
    fn test_migration_status_detection_mirror() {
        let platforms = vec![
            PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test/repo".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: None,
                last_verified: None,
                notes: None,
            },
            PlatformInfo {
                platform: Platform::Codeberg,
                url: "https://codeberg.org/test/repo".to_string(),
                status: PlatformStatus::Active,
                is_primary: false,
                migration_date: None,
                last_verified: None,
                notes: Some("Mirror".to_string()),
            },
        ];

        let status = MigrationRecord::detect_status(&platforms);
        assert_eq!(status, MigrationStatus::Mirror);
    }

    #[test]
    fn test_migration_status_detection_archived_migrated() {
        let platforms = vec![
            PlatformInfo {
                platform: Platform::GitHub,
                url: "https://github.com/test/repo".to_string(),
                status: PlatformStatus::Archived,
                is_primary: false,
                migration_date: Some("2024-01-15".to_string()),
                last_verified: None,
                notes: Some("Moved to Codeberg".to_string()),
            },
            PlatformInfo {
                platform: Platform::Codeberg,
                url: "https://codeberg.org/test/repo".to_string(),
                status: PlatformStatus::Active,
                is_primary: true,
                migration_date: Some("2024-01-15".to_string()),
                last_verified: None,
                notes: None,
            },
        ];

        let status = MigrationRecord::detect_status(&platforms);
        assert_eq!(status, MigrationStatus::ArchivedMigrated);
    }
}
