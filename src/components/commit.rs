//! Commit metadata display components

use maud::{Markup, html};

/// Renders commit metadata line
///
/// Displays author, hash, and timestamp in a horizontal layout with separators.
/// Used in commit history listings.
///
/// # Arguments
///
/// * `author`: Commit author name
/// * `hash`: Full commit hash (short hash derived internally)
/// * `time`: Formatted relative timestamp
///
/// # Returns
///
/// Commit metadata markup with author, hash, and time
pub fn commit_meta(author: &str, hash: &str, time: &str) -> Markup {
    let short_hash = if hash.len() >= 7 { &hash[..7] } else { hash };
    html! {
        div class="commit-meta" {
            span { (author) }
            span { "·" }
            code class="commit-hash" data-full=(hash) { (short_hash) }
            span { "·" }
            span { (time) }
        }
    }
}
