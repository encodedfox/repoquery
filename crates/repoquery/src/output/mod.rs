pub mod chart;
pub mod csv;
pub mod json;
pub mod markdown;
pub mod table;

use rq_core::Repository;

pub trait Formatter {
    fn format_list(&self, repos: &[Repository]) -> String;
    fn format_detail(&self, repo: &Repository) -> String;
}

pub enum OutputFormat {
    Table,
    Json,
    Markdown,
    Csv,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "md" | "markdown" => OutputFormat::Markdown,
            "csv" => OutputFormat::Csv,
            _ => OutputFormat::Table,
        }
    }

    pub fn formatter(&self) -> Box<dyn Formatter> {
        match self {
            OutputFormat::Table => Box::new(table::TableFormatter),
            OutputFormat::Json => Box::new(json::JsonFormatter),
            OutputFormat::Markdown => Box::new(markdown::MarkdownFormatter),
            OutputFormat::Csv => Box::new(csv::CsvFormatter),
        }
    }
}
