//! Repository metadata components

use maud::{Markup, html};

/// Renders repository header with name and owner
///
/// Displays repository name prominently with optional owner prefix.
/// Used at top of index page.
///
/// # Arguments
///
/// * `name`: Repository name
/// * `owner`: Optional owner name (appears before slash)
///
/// # Returns
///
/// Repository header markup
pub fn repo_header(name: &str, owner: Option<&str>) -> Markup {
    html! {
        header class="repo-header" {
            @if let Some(owner_name) = owner {
                span class="repo-owner" { (owner_name) " / " }
            }
            h1 class="repo-name" { (name) }
        }
    }
}

/// Renders branch selector dropdown
///
/// Shows available branches with active branch highlighted. When repository
/// has fewer than threshold branches, displays static badge instead.
///
/// # Arguments
///
/// * `branches`: Slice of branch names
/// * `current`: Name of currently active branch
/// * `min_for_selector`: Minimum branches to show dropdown (else shows badge)
///
/// # Returns
///
/// Branch selector or static badge markup
pub fn branch_selector(branches: &[&str], current: &str, min_for_selector: usize) -> Markup {
    html! {
        @if branches.len() >= min_for_selector {
            div class="branch-selector" {
                i class="ph ph-git-branch" {}
                @for branch in branches {
                    @if *branch == current {
                        span class="branch-name branch-active" { (branch) }
                    } @else {
                        span class="branch-name" { (branch) }
                    }
                }
                i class="ph ph-caret-down branch-caret" {}
            }
        } @else {
            div class="branch-selector" {
                i class="ph ph-git-branch" {}
                span { (current) }
            }
        }
    }
}
