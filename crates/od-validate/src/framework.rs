//! Validation framework and core types

use od_core::CanonicalData;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Validation error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationErrorCode {
    /// E001: Duplicate repository URLs
    DuplicateRepo,
    /// E002: Missing license information
    MissingLicense,
    /// E003: Invalid URL format
    InvalidUrl,
    /// E004: Broken cross-reference
    BrokenCrossRef,
    /// E005: Platform information mismatch
    PlatformMismatch,
    /// E006: Missing required metadata
    MissingMetadata,
    /// E007: External data inconsistency
    ExternalDataInconsistent,
    /// E008: Duplicate repository name
    DuplicateRepositoryName,
}

impl std::fmt::Display for ValidationErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateRepo => write!(f, "E001"),
            Self::MissingLicense => write!(f, "E002"),
            Self::InvalidUrl => write!(f, "E003"),
            Self::BrokenCrossRef => write!(f, "E004"),
            Self::PlatformMismatch => write!(f, "E005"),
            Self::MissingMetadata => write!(f, "E006"),
            Self::ExternalDataInconsistent => write!(f, "E007"),
            Self::DuplicateRepositoryName => write!(f, "E008"),
        }
    }
}

/// Location information for validation issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationLocation {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

/// Individual validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: ValidationErrorCode,
    pub severity: Severity,
    pub message: String,
    pub location: ValidationLocation,
    pub suggestion: String,
    pub auto_fixable: bool,
}

/// Result from a validation check
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self { issues: vec![] }
    }

    pub fn single(issue: ValidationIssue) -> Self {
        Self {
            issues: vec![issue],
        }
    }

    pub fn multiple(issues: Vec<ValidationIssue>) -> Self {
        Self { issues }
    }

    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }
}

/// Validation rule trait
pub trait ValidationRule: Send + Sync {
    /// Name of the validation rule
    fn name(&self) -> &str;

    /// Default severity for this rule
    fn default_severity(&self) -> Severity;

    /// Run the validation check
    fn check(&self, data: &CanonicalData) -> ValidationResult;
}

/// Statistics from validation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub repos_by_platform: HashMap<String, usize>,
    pub archived_count: usize,
    pub missing_licenses: usize,
    pub stale_references: usize,
    pub migration_count: usize,
}

/// Summary of validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub total_repos: usize,
}

/// Complete validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub timestamp: String,
    pub summary: ValidationSummary,
    pub issues: Vec<ValidationIssue>,
    pub metrics: ValidationMetrics,
}

impl ValidationReport {
    /// Create new validation report from issues and data
    pub fn new(issues: Vec<ValidationIssue>, data: &CanonicalData) -> Self {
        let mut errors = 0;
        let mut warnings = 0;
        let mut info = 0;

        for issue in &issues {
            match issue.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
                Severity::Info => info += 1,
            }
        }

        // Calculate metrics
        let mut repos_by_platform: HashMap<String, usize> = HashMap::new();
        let mut archived_count = 0;
        let mut missing_licenses = 0;
        let mut migration_count = 0;

        for repo in &data.repositories {
            // Count by platform
            for platform in &repo.platforms {
                let platform_name = format!("{}", platform.platform);
                *repos_by_platform.entry(platform_name).or_insert(0) += 1;
            }

            // Count archived
            if repo.quality_metrics.archive_status {
                archived_count += 1;
            }

            // Count missing licenses
            if repo.metadata.license.is_none() {
                missing_licenses += 1;
            }

            // Count migrations
            if repo.platforms.len() > 1 {
                migration_count += 1;
            }
        }

        let stale_references = data.web_references.iter().filter(|r| r.is_stale()).count();

        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            summary: ValidationSummary {
                errors,
                warnings,
                info,
                total_repos: data.repositories.len(),
            },
            issues,
            metrics: ValidationMetrics {
                repos_by_platform,
                archived_count,
                missing_licenses,
                stale_references,
                migration_count,
            },
        }
    }

    /// Save report to JSON file
    pub fn to_json_file(&self, path: &std::path::Path) -> Result<(), crate::ValidateError> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::ValidateError::Other(e.to_string()))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        Ok(())
    }

    /// Check if validation passed (no errors)
    pub fn passed(&self) -> bool {
        self.summary.errors == 0
    }
}

/// Validator that runs multiple validation rules
pub struct Validator {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl Validator {
    /// Create new validator
    pub fn new() -> Self {
        Self { rules: vec![] }
    }

    /// Add a validation rule
    pub fn add_rule<R: ValidationRule + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(rule));
    }

    /// Run all validation rules
    pub fn validate(&self, data: &CanonicalData) -> ValidationReport {
        tracing::info!("Running validation with {} rules", self.rules.len());

        let mut all_issues = Vec::new();

        for rule in &self.rules {
            tracing::debug!("Running rule: {}", rule.name());
            let result = rule.check(data);
            all_issues.extend(result.issues);
        }

        tracing::info!("Validation complete: {} issues found", all_issues.len());

        ValidationReport::new(all_issues, data)
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_has_errors() {
        let result = ValidationResult::single(ValidationIssue {
            code: ValidationErrorCode::DuplicateRepo,
            severity: Severity::Error,
            message: "Test error".to_string(),
            location: ValidationLocation {
                file: "test.yml".to_string(),
                line: Some(10),
                section: None,
            },
            suggestion: "Fix it".to_string(),
            auto_fixable: false,
        });

        assert!(result.has_errors());
    }

    #[test]
    fn test_validation_result_no_errors() {
        let result = ValidationResult::single(ValidationIssue {
            code: ValidationErrorCode::MissingLicense,
            severity: Severity::Warning,
            message: "Test warning".to_string(),
            location: ValidationLocation {
                file: "test.yml".to_string(),
                line: None,
                section: None,
            },
            suggestion: "Add license".to_string(),
            auto_fixable: false,
        });

        assert!(!result.has_errors());
    }

    #[test]
    fn test_validation_report_passed() {
        let data = CanonicalData::new();
        let report = ValidationReport::new(vec![], &data);

        assert!(report.passed());
        assert_eq!(report.summary.errors, 0);
    }
}
