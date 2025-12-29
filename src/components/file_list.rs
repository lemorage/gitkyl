//! File listing table components

use maud::{Markup, html};

use crate::git::CommitInfo;

/// Wraps file rows in table container
///
/// Provides semantic file table structure with consistent styling.
/// The container handles the card styling while individual rows are
/// rendered by `file_row`.
///
/// # Arguments
///
/// * `rows`: Markup containing individual file row elements
///
/// # Returns
///
/// File table wrapper with rows
pub fn file_table(rows: Markup) -> Markup {
    html! {
        div class="file-table" {
            (rows)
        }
    }
}

/// Formats detailed commit tooltip for file rows.
///
/// Shows author with email, co-authors, committer (if different), hash, and message.
fn format_tooltip(commit: &CommitInfo) -> String {
    let mut lines = vec![format!(
        "Author: {} <{}>",
        commit.author(),
        commit.author_email()
    )];

    for co in commit.co_authors() {
        lines.push(format!("Co-author: {}", co));
    }

    if commit.author() != commit.committer() {
        lines.push(format!("Committer: {}", commit.committer()));
    }

    lines.push(format!("Commit: {}", commit.oid()));

    let msg = commit.message_full().trim();
    if !msg.is_empty() {
        lines.push(String::new());
        lines.push(msg.to_string());
    }

    lines.join("\n")
}

/// Renders single file row in table
///
/// Displays file information in a grid layout with icon, name, commit message,
/// and timestamp. Tooltip shows detailed commit metadata when commit is provided.
///
/// # Arguments
///
/// * `href`: Link target for row click
/// * `icon`: Icon markup (from icons module)
/// * `name`: File or directory name to display
/// * `commit`: Optional commit information for tooltip and message display
/// * `formatted_date`: Pre-formatted timestamp string
///
/// # Returns
///
/// Clickable file row with all metadata displayed
pub fn file_row(
    href: &str,
    icon: Markup,
    name: &str,
    commit: Option<&CommitInfo>,
    formatted_date: &str,
) -> Markup {
    let (tooltip, message) = match commit {
        Some(c) => (format_tooltip(c), c.message().to_string()),
        None => (String::new(), String::new()),
    };

    html! {
        a href=(href) class="file-row" {
            div class="file-name-cell" {
                (icon)
                span { (name) }
            }
            @if !tooltip.is_empty() {
                div class="commit-message" title=(tooltip) {
                    (message)
                }
            } @else {
                div class="commit-message" {}
            }
            div class="commit-date" {
                (formatted_date)
            }
        }
    }
}
