//! File listing table components

use maud::{Markup, html};

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
#[allow(dead_code)]
pub fn file_table(rows: Markup) -> Markup {
    html! {
        div class="file-table" {
            (rows)
        }
    }
}

/// Renders single file row in table
///
/// Displays file information in a grid layout with icon, name, commit message,
/// and timestamp. Handles hover states via CSS.
///
/// # Arguments
///
/// * `href`: Link target for row click
/// * `icon`: Icon markup (from icons module)
/// * `name`: File or directory name to display
/// * `commit_msg`: Commit message text
/// * `commit_msg_full`: Full commit message for title tooltip
/// * `commit_date`: Formatted timestamp string
///
/// # Returns
///
/// Clickable file row with all metadata displayed
#[allow(dead_code)]
pub fn file_row(
    href: &str,
    icon: Markup,
    name: &str,
    commit_msg: &str,
    commit_msg_full: &str,
    commit_date: &str,
) -> Markup {
    html! {
        a href=(href) class="file-row" {
            (icon)
            div class="file-link" { (name) }
            div class="commit-message" title=(commit_msg_full) {
                (commit_msg)
            }
            div class="commit-date" {
                (commit_date)
            }
        }
    }
}
