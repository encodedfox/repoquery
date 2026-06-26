/// ASCII histogram bar chart renderer.
use std::cmp;

/// Render a horizontal bar chart with labels.
///
/// Each entry is a `(label, value)` pair. The chart auto-scales to the
/// terminal width (or a default of 80).
pub fn bar_chart(data: &[(&str, u64)], header: &str) -> String {
    if data.is_empty() {
        return String::new();
    }

    let term_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    let max_label = data.iter().map(|(l, _)| l.len()).max().unwrap_or(10);
    let max_val = data.iter().map(|(_, v)| v).max().copied().unwrap_or(1);
    let bar_max = term_width.saturating_sub(max_label + 12).max(10);

    let mut out = String::new();
    out.push_str(header);
    out.push('\n');
    out.push_str(&"-".repeat(cmp::min(term_width, header.len())));
    out.push('\n');

    for (label, value) in data {
        let bar_len = if max_val > 0 {
            let raw = (*value as f64 / max_val as f64) * bar_max as f64;
            if raw < 1.0 {
                1
            } else {
                raw as usize
            }
        } else {
            1
        };
        let bar: String = std::iter::repeat('█').take(bar_len).collect();
        out.push_str(&format!(
            "{:<width$} {:>8} {}\n",
            label,
            value,
            bar,
            width = max_label
        ));
    }

    out
}
