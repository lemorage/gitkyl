//! Commit metadata display components

use maud::{Markup, html};

/// Renders commit badge with message and hash
///
/// Displays commit message with short hash in a compact format.
/// Used in index page repo controls section.
///
/// # Arguments
///
/// * `message`: Commit message summary (first line)
/// * `hash`: Short commit hash (7 characters)
///
/// # Returns
///
/// Commit badge markup with message and hash
pub fn commit_badge(message: &str, hash: &str) -> Markup {
    html! {
        div class="commit-line" {
            span class="avatar-placeholder" {}
            span class="repo-commit-message" { (message) }
        }
        div class="commit-meta" {
            code class="commit-hash" { (hash) }
        }
    }
}

/// Renders commit metadata line
///
/// Displays author, hash, and timestamp in a horizontal layout with separators.
/// Used in commit history listings.
///
/// # Arguments
///
/// * `author`: Commit author name
/// * `hash`: Short commit hash
/// * `time`: Formatted relative timestamp
///
/// # Returns
///
/// Commit metadata markup with author, hash, and time
pub fn commit_meta(author: &str, hash: &str, time: &str) -> Markup {
    html! {
        div class="commit-meta" {
            span { (author) }
            span { "·" }
            code class="commit-hash" { (hash) }
            span { "·" }
            span { (time) }
        }
    }
}
