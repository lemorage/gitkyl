//! Repository metadata components

use maud::{Markup, html};

/// Data for repository header rendering
pub struct RepoHeaderData<'a> {
    pub name: &'a str,
    pub owner: Option<&'a str>,
    pub tag_count: usize,
    pub tags_href: Option<&'a str>,
}

/// Renders repository header with name, owner, and tag count
///
/// # Arguments
///
/// * `data`: Header data containing name, owner, and tag info
///
/// # Returns
///
/// Repository header markup
pub fn repo_header(data: RepoHeaderData<'_>) -> Markup {
    html! {
        header class="repo-header" {
            h1 class="repo-title" {
                @if let Some(owner_name) = data.owner {
                    span class="repo-owner" { (owner_name) " / " }
                }
                span class="repo-name" { (data.name) }
            }
            @if data.tag_count > 0 {
                @if let Some(href) = data.tags_href {
                    a href=(href) class="repo-tags-link" {
                        i class="ph ph-tag" {}
                        (data.tag_count)
                    }
                }
            }
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
/// * `depth`: Current page depth for relative path calculation
///
/// # Returns
///
/// Branch selector or static badge markup
pub fn branch_selector(
    branches: &[&str],
    current: &str,
    min_for_selector: usize,
    depth: usize,
) -> Markup {
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
                        @let href = format!("{}tree/{}/index.html", "../".repeat(depth), branch);
                        a class="branch-item" href=(href) {
                            span { (branch) }
                        }
                    }
                }
            }
        }
    }
}
