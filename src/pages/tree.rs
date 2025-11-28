//! Tree page generation for directory browsing

use anyhow::Result;
use maud::{DOCTYPE, Markup, html};
use std::path::Path;

use crate::git::TreeItem;
use crate::time::format_timestamp;

/// Generates HTML tree page for directory browsing
///
/// Creates a hierarchical directory view showing files and subdirectories
/// at the specified path. Displays file metadata and provides navigation
/// links to nested trees and blob pages. Includes parent directory navigation
/// when not at repository root.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `tree_path`: Directory path within repository (empty for root)
/// * `repo_name`: Repository name for page title
/// * `items`: Tree items (files and directories) at this level
///
/// # Returns
///
/// HTML markup for the tree page
///
/// # Examples
///
/// ```no_run
/// use gitkyl::pages::tree::generate;
/// use gitkyl::TreeItem;
/// use std::path::Path;
///
/// let items = vec![]; // Populate with TreeItem instances
/// let html = generate(Path::new("."), "main", "", "my-repo", &items)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate(
    _repo_path: impl AsRef<Path>,
    ref_name: &str,
    tree_path: &str,
    repo_name: &str,
    items: &[TreeItem],
) -> Result<Markup> {
    let path_components: Vec<&str> = if tree_path.is_empty() {
        vec![]
    } else {
        tree_path.split('/').filter(|s| !s.is_empty()).collect()
    };

    let depth = path_components.len() + 1;
    let index_path = "../".repeat(depth) + "index.html";

    Ok(html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (if tree_path.is_empty() { repo_name.to_string() } else { format!("{} - {}", tree_path, repo_name) }) " - Gitkyl" }
                script src="https://unpkg.com/@phosphor-icons/web" {}
                link rel="stylesheet" href=(format!("{}assets/tree.css", "../".repeat(depth)));
            }
            body {
                div class="container" {
                    header {
                        div class="breadcrumb" {
                            a href=(index_path) class="breadcrumb-link" { (repo_name) }
                            @if !path_components.is_empty() {
                                span class="breadcrumb-separator" { "/" }
                                @for (idx, component) in path_components.iter().enumerate() {
                                    @if idx == path_components.len() - 1 {
                                        span class="breadcrumb-current" { (*component) }
                                    } @else {
                                        @let partial_path = path_components[..=idx].join("/");
                                        a href=(format!("{}tree/{}/{}.html", "../".repeat(depth), ref_name, partial_path)) class="breadcrumb-link" {
                                            (*component)
                                        }
                                        span class="breadcrumb-separator" { "/" }
                                    }
                                }
                            }
                        }
                        div class="ref-info" {
                            span class="ref-label" { "ref: " }
                            span class="ref-name" { (ref_name) }
                        }
                    }
                    main class="tree-container" {
                        @if items.is_empty() && tree_path.is_empty() {
                            p class="empty-state" { "Empty directory" }
                        } @else {
                            div class="file-table" {
                                @if !tree_path.is_empty() {
                                    @let parent_path = if path_components.len() > 1 {
                                        path_components[..path_components.len() - 1].join("/")
                                    } else {
                                        String::new()
                                    };
                                    @let parent_href = if parent_path.is_empty() {
                                        format!("{}index.html", "../".repeat(depth))
                                    } else {
                                        format!("{}tree/{}/{}.html", "../".repeat(depth), ref_name, parent_path)
                                    };
                                    a href=(parent_href) class="file-row" {
                                        div class="icon-box" {
                                            i class="ph ph-arrow-up icon-folder" {}
                                        }
                                        div class="file-link" { ".." }
                                        div class="file-meta" { "" }
                                    }
                                }
                                @for item in items {
                                    @match item {
                                        TreeItem::File { entry, commit } => {
                                            @if let Some(path) = entry.path()
                                                && let Some(path_str) = path.to_str() {
                                                @let display_name = if tree_path.is_empty() {
                                                    path_str.to_string()
                                                } else if let Some(stripped) = path_str.strip_prefix(tree_path) {
                                                    stripped.trim_start_matches('/').to_string()
                                                } else {
                                                    path_str.to_string()
                                                };

                                                @let href = format!("{}blob/{}/{}.html", "../".repeat(depth), ref_name, path_str);

                                                @let icon_class = if display_name.to_lowercase().starts_with("readme") {
                                                    "ph ph-info"
                                                } else if display_name.ends_with(".rs") {
                                                    "ph ph-file-rs"
                                                } else if display_name.ends_with(".toml") || display_name.ends_with(".yaml") || display_name.ends_with(".yml") {
                                                    "ph ph-gear"
                                                } else {
                                                    "ph ph-file"
                                                };

                                                @let icon_modifier = if display_name.to_lowercase().starts_with("readme") {
                                                    Some("icon-readme")
                                                } else if display_name.ends_with(".rs") {
                                                    Some("icon-rust")
                                                } else if display_name.ends_with(".toml") || display_name.ends_with(".yaml") || display_name.ends_with(".yml") {
                                                    Some("icon-config")
                                                } else {
                                                    None
                                                };

                                                a href=(href) class="file-row" {
                                                    div class="icon-box" {
                                                        @if let Some(modifier) = icon_modifier {
                                                            i class=(format!("{} {}", icon_class, modifier)) {}
                                                        } @else {
                                                            i class=(icon_class) {}
                                                        }
                                                    }
                                                    div class="file-link" { (display_name) }
                                                    div class="commit-message" title=(commit.message_full()) {
                                                        (commit.message())
                                                    }
                                                    div class="commit-date" {
                                                        (format_timestamp(commit.date()))
                                                    }
                                                }
                                            }
                                        },
                                        TreeItem::Directory { name, full_path, commit } => {
                                            @let href = format!("{}tree/{}/{}.html", "../".repeat(depth), ref_name, full_path);
                                            a href=(href) class="file-row" {
                                                div class="icon-box" {
                                                    i class="ph-fill ph-folder icon-folder" {}
                                                }
                                                div class="file-link" { (name) }
                                                div class="commit-message" title=(commit.message_full()) {
                                                    (commit.message())
                                                }
                                                div class="commit-date" {
                                                    (format_timestamp(commit.date()))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    footer {
                        p {
                            "Generated by "
                            a href="https://github.com/lemorage/gitkyl" target="_blank" { "Gitkyl" }
                        }
                    }
                }
            }
        }
    })
}
