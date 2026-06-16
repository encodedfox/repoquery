//! Progress tracking for sync operations

use indicatif::{ProgressBar, ProgressStyle};

/// Progress tracker for sync operations
pub struct ProgressTracker {
    bar: Option<ProgressBar>,
    synced: usize,
    cached: usize,
    failed: usize,
}

impl ProgressTracker {
    /// Create new progress tracker
    pub fn new() -> Self {
        Self {
            bar: None,
            synced: 0,
            cached: 0,
            failed: 0,
        }
    }

    /// Start progress tracking with total count
    pub fn start(&mut self, total: usize) {
        let bar = ProgressBar::new(total as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("▓▓░"),
        );

        self.bar = Some(bar);
    }

    /// Increment synced count with repo name
    pub fn increment_synced(&mut self, repo_name: &str) {
        self.synced += 1;
        if let Some(bar) = &self.bar {
            bar.set_message(format!("✅ {}", repo_name));
            bar.inc(1);
        }
    }

    /// Increment cached count
    pub fn increment_cached(&mut self) {
        self.cached += 1;
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }

    /// Increment failed count
    pub fn increment_failed(&mut self) {
        self.failed += 1;
        if let Some(bar) = &self.bar {
            bar.set_message("❌ Failed");
            bar.inc(1);
        }
    }

    /// Finish progress tracking with summary
    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_with_message(format!(
                "Complete: ✅ {} synced, ⚠️ {} cached, ❌ {} failed",
                self.synced, self.cached, self.failed
            ));
        }
    }

    /// Get current counts
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.synced, self.cached, self.failed)
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker_counts() {
        let mut tracker = ProgressTracker::new();

        tracker.increment_synced("test/repo1");
        tracker.increment_synced("test/repo2");
        tracker.increment_cached();
        tracker.increment_failed();

        let (synced, cached, failed) = tracker.counts();
        assert_eq!(synced, 2);
        assert_eq!(cached, 1);
        assert_eq!(failed, 1);
    }

    #[test]
    fn test_progress_tracker_default() {
        let tracker = ProgressTracker::default();
        let (synced, cached, failed) = tracker.counts();
        assert_eq!(synced, 0);
        assert_eq!(cached, 0);
        assert_eq!(failed, 0);
    }
}