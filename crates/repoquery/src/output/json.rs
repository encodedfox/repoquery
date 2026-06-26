use super::Formatter;
use rq_core::Repository;

pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format_list(&self, repos: &[Repository]) -> String {
        serde_json::to_string_pretty(repos).unwrap_or_else(|_| "[]".to_string())
    }

    fn format_detail(&self, repo: &Repository) -> String {
        serde_json::to_string_pretty(repo).unwrap_or_else(|_| "{}".to_string())
    }
}
