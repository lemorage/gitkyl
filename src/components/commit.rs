//! Commit attribution display components

use maud::{Markup, html};

use crate::git::CommitInfo;

/// Renders commit attribution with all authors visible and committer indicator.
///
/// Shows author and co-authors inline. When committer differs from author,
/// displays a dagger (†) with CSS tooltip showing committer name.
pub fn attribution(commit: &CommitInfo) -> Markup {
    let has_co_authors = !commit.co_authors().is_empty();
    let has_different_committer = commit.author() != commit.committer();

    if !has_co_authors && !has_different_committer {
        return html! {
            span class="attribution" { (commit.author()) }
        };
    }

    let mut authors = vec![commit.author().to_string()];
    authors.extend(commit.co_authors().iter().cloned());
    let author_list = authors.join(", ");

    if has_different_committer {
        let tooltip = format!("Committer: {}", commit.committer());
        html! {
            span class="attribution" {
                (author_list)
                span class="attribution-dagger" data-tooltip=(tooltip) { "†" }
            }
        }
    } else {
        html! {
            span class="attribution" { (author_list) }
        }
    }
}

/// Renders commit hash with CSS tooltip showing full hash.
pub fn commit_hash(hash: &str) -> Markup {
    let short = if hash.len() >= 7 { &hash[..7] } else { hash };
    html! {
        code class="commit-hash" data-full=(hash) { (short) }
    }
}
