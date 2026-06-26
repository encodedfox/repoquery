use super::Formatter;
use rq_core::Repository;
use std::cmp;

pub struct TableFormatter;

impl Formatter for TableFormatter {
    fn format_list(&self, repos: &[Repository]) -> String {
        if repos.is_empty() {
            return "No repositories found.".to_string();
        }
        let term_width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);

        let name_w = 30.min(term_width / 3);
        let lang_w = 12.min(term_width / 6);
        let stars_w = 8;
        let desc_w = term_width
            .saturating_sub(name_w + lang_w + stars_w + 6)
            .max(20);

        let mut out = String::new();
        let hdr = format!(
            "{:<name_w$} {:<lang_w$} {:>stars_w$} {:<desc_w$}",
            "Name",
            "Language",
            "Stars",
            "Description",
            name_w = name_w,
            lang_w = lang_w,
            stars_w = stars_w,
            desc_w = desc_w,
        );
        out.push_str(&hdr);
        out.push('\n');
        out.push_str(&"-".repeat(cmp::min(term_width, hdr.len())));
        out.push('\n');

        for repo in repos {
            let name = truncate(&repo.metadata.full_name, name_w);
            let lang = truncate(&repo.metadata.primary_language, lang_w);
            let stars = format!("{:>8}", repo.metadata.stars);
            let desc = truncate(&repo.metadata.description, desc_w);
            out.push_str(&format!(
                "{:<name_w$} {:<lang_w$} {:>stars_w$} {:<desc_w$}\n",
                name,
                lang,
                stars,
                desc,
                name_w = name_w,
                lang_w = lang_w,
                stars_w = stars_w,
                desc_w = desc_w,
            ));
        }
        out
    }

    fn format_detail(&self, repo: &Repository) -> String {
        let mut out = String::new();
        out.push_str(&format!("Name:        {}\n", repo.metadata.full_name));
        out.push_str(&format!("Owner:       {}\n", repo.metadata.owner));
        out.push_str(&format!("Stars:       {}\n", repo.metadata.stars));
        out.push_str(&format!(
            "Language:    {}\n",
            repo.metadata.primary_language
        ));
        out.push_str(&format!(
            "License:     {}\n",
            repo.metadata.license_spdx.as_deref().unwrap_or("N/A")
        ));
        out.push_str(&format!("Description: {}\n", repo.metadata.description));
        out.push_str(&format!(
            "Quality:     {}/100\n",
            repo.quality_metrics.quality_score
        ));
        out.push_str(&format!(
            "Archived:    {}\n",
            repo.quality_metrics.archive_status
        ));
        if !repo.metadata.topics.is_empty() {
            out.push_str(&format!(
                "Topics:      {}\n",
                repo.metadata.topics.join(", ")
            ));
        }
        if let Some(ref fp) = repo.fork_parent {
            out.push_str(&format!("Fork of:     {}\n", fp));
        }
        out
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
