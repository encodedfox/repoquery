use super::Formatter;
use rq_core::Repository;

pub struct CsvFormatter;

impl Formatter for CsvFormatter {
    fn format_list(&self, repos: &[Repository]) -> String {
        let mut out = String::from("name,owner,language,stars,license,description,topics\n");
        for repo in repos {
            out.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                csv_escape(&repo.metadata.full_name),
                csv_escape(&repo.metadata.owner),
                csv_escape(&repo.metadata.primary_language),
                repo.metadata.stars,
                csv_escape(repo.metadata.license_spdx.as_deref().unwrap_or("")),
                csv_escape(&repo.metadata.description),
                csv_escape(&repo.metadata.topics.join(";")),
            ));
        }
        out
    }

    fn format_detail(&self, repo: &Repository) -> String {
        self.format_list(&[repo.clone()])
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
