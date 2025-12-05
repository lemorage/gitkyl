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

/// Renders branch selector with navigation links
///
/// Shows available branches with active branch highlighted. Each branch is a
/// clickable link to that branch's index page. When repository has fewer than
/// threshold branches, displays static badge instead.
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
    if branches.len() < min_for_selector {
        return html! {
            div class="branch-info" {
                i class="ph ph-git-branch" {}
                span class="branch-name branch-active" { (current) }
            }
        };
    }

    html! {
        div class="branch-selector" {
            div class="branch-button" {
                i class="ph ph-git-branch" {}
                span class="branch-name branch-active" { (current) }
                i class="ph ph-caret-down branch-caret" {}
            }
            div class="branch-dropdown" {
                @for branch in branches {
                    @if *branch == current {
                        div class="branch-item branch-current" {
                            i class="ph ph-check" {}
                            span { (branch) }
                        }
                    } @else {
                        a class="branch-item" href=(format!("tree/{}/index.html", branch)) {
                            span { (branch) }
                        }
                    }
                }
            }
        }
    }
}
