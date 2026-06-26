use super::Formatter;
use rq_core::Repository;

pub struct MarkdownFormatter;

impl Formatter for MarkdownFormatter {
    fn format_list(&self, repos: &[Repository]) -> String {
        if repos.is_empty() {
            return "No repositories found.".to_string();
        }
        let mut out = String::from("| Name | Language | Stars | Description |\n");
        out.push_str("|------|----------|-------|-------------|\n");
        for repo in repos {
            let desc = repo.metadata.description.replace('|', "\\|");
            let name = repo.metadata.full_name.replace('|', "\\|");
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                name,
                repo.metadata.primary_language,
                repo.metadata.stars,
                truncate_md(&desc, 60),
            ));
        }
        out
    }

    fn format_detail(&self, repo: &Repository) -> String {
        let mut out = String::new();
        out.push_str(&format!("## {}\n\n", repo.metadata.full_name));
        out.push_str(&format!("- **Owner**: {}\n", repo.metadata.owner));
        out.push_str(&format!("- **Stars**: {}\n", repo.metadata.stars));
        out.push_str(&format!(
            "- **Language**: {}\n",
            repo.metadata.primary_language
        ));
        out.push_str(&format!(
            "- **License**: {}\n",
            repo.metadata.license_spdx.as_deref().unwrap_or("N/A")
        ));
        out.push_str(&format!(
            "- **Quality**: {}/100\n",
            repo.quality_metrics.quality_score
        ));
        out.push_str(&format!(
            "- **Archived**: {}\n",
            repo.quality_metrics.archive_status
        ));
        if !repo.metadata.topics.is_empty() {
            out.push_str(&format!(
                "- **Topics**: {}\n",
                repo.metadata.topics.join(", ")
            ));
        }
        out.push_str(&format!("\n{}\n", repo.metadata.description));
        out
    }
}

fn truncate_md(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
