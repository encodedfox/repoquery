//! README update mechanism with HTML comment injection

use regex::Regex;

/// README updater that preserves manual content
pub struct ReadmeUpdater {
    _marker_start: Regex,
    _marker_end: Regex,
}

impl ReadmeUpdater {
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            _marker_start: Regex::new(r"<!--\s*AUTO-GENERATED:START[^>]*-->")?,
            _marker_end: Regex::new(r"<!--\s*AUTO-GENERATED:END\s*-->")?,
        })
    }

    /// Update README with generated content between markers
    pub fn update(&self, readme: &str, section: &str, content: &str) -> String {
        let marker_start = format!("<!-- AUTO-GENERATED:START {} -->", section);
        let marker_end = "<!-- AUTO-GENERATED:END -->";

        if readme.contains(&marker_start) {
            // Replace existing marked section
            let re = Regex::new(&format!(
                r"(?s){}.*?{}",
                regex::escape(&marker_start),
                regex::escape(marker_end)
            ))
            .unwrap();

            let replacement = format!("{}\n{}\n{}", marker_start, content, marker_end);
            re.replace(readme, replacement.as_str()).to_string()
        } else {
            // Append new section at end
            format!(
                "{}\n\n{}\n{}\n{}\n",
                readme, marker_start, content, marker_end
            )
        }
    }
}

impl Default for ReadmeUpdater {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_existing_section() {
        let updater = ReadmeUpdater::new().unwrap();
        let readme = "# README\n<!-- AUTO-GENERATED:START test -->\nOld content\n<!-- AUTO-GENERATED:END -->\n";

        let result = updater.update(readme, "test", "New content");

        assert!(result.contains("New content"));
        assert!(!result.contains("Old content"));
    }

    #[test]
    fn test_add_new_section() {
        let updater = ReadmeUpdater::new().unwrap();
        let readme = "# README\nManual content";

        let result = updater.update(readme, "test", "Generated content");

        assert!(result.contains("Manual content"));
        assert!(result.contains("Generated content"));
        assert!(result.contains("<!-- AUTO-GENERATED:START test -->"));
    }
}
