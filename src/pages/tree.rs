//! Tree page generation for directory browsing

use anyhow::Result;
use maud::{Markup, html};
use std::path::Path;

use crate::components::file_list::{file_row, file_table};
use crate::components::icons::file_icon;
use crate::components::layout::page_wrapper;
use crate::components::nav::breadcrumb;
use crate::git::TreeItem;
use crate::util::{calculate_depth, format_timestamp};

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

    let depth = calculate_depth(ref_name, tree_path);
    let index_path = "../".repeat(depth) + "index.html";

    // Build breadcrumb data from path_components
    let breadcrumb_data: Vec<(&str, Option<String>)> = if path_components.is_empty() {
        vec![]
    } else {
        path_components
            .iter()
            .enumerate()
            .map(|(idx, &component)| {
                if idx == path_components.len() - 1 {
                    (component, None) // Current directory, no link
                } else {
                    let partial_path = path_components[..=idx].join("/");
                    let link = format!(
                        "{}tree/{}/{}.html",
                        "../".repeat(depth),
                        ref_name,
                        partial_path
                    );
                    (component, Some(link))
                }
            })
            .collect()
    };

    let title = if tree_path.is_empty() {
        format!("{}/{}", repo_name, ref_name)
    } else {
        format!("{}/{}: {}", repo_name, ref_name, tree_path)
    };

    let css_path = format!("{}assets/tree.css", "../".repeat(depth));

    Ok(page_wrapper(
        &title,
        &[&css_path],
        html! {
            (breadcrumb(repo_name, &index_path, &breadcrumb_data, ref_name))
            main class="tree-container" {
                @if items.is_empty() && tree_path.is_empty() {
                    p class="empty-state" { "Empty directory" }
                } @else {
                    (file_table(html! {
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
                            // Parent directory link with custom icon
                            (file_row(
                                &parent_href,
                                html! { div class="icon-box" { i class="ph ph-arrow-up icon-folder" {} } },
                                "..",
                                "",
                                "",
                                ""
                            ))
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

                                        (file_row(
                                            &href,
                                            file_icon(&display_name),
                                            &display_name,
                                            commit.message(),
                                            commit.message_full(),
                                            &format_timestamp(commit.date())
                                        ))
                                    }
                                },
                                TreeItem::Directory { name, full_path, commit } => {
                                    @let href = format!("{}tree/{}/{}.html", "../".repeat(depth), ref_name, full_path);
                                    (file_row(
                                        &href,
                                        file_icon(&format!("{}/", full_path)),
                                        name,
                                        commit.message(),
                                        commit.message_full(),
                                        &format_timestamp(commit.date())
                                    ))
                                }
                            }
                        }
                    }))
                }
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommitInfo, list_files};

    #[test]
    fn test_generate_empty() {
        let items: Vec<TreeItem> = vec![];
        let html = generate(Path::new("."), "main", "", "test-repo", &items).unwrap();

        let html_str = html.into_string();
        assert!(html_str.contains("test-repo"));
        assert!(html_str.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_generate_with_files() {
        use std::fs;
        use std::process::Command;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        fs::write(dir.path().join("file.txt"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "test"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let files = list_files(dir.path(), None).unwrap();
        let commit = CommitInfo::new(
            "abc123".into(),
            "test".into(),
            "test".into(),
            "Test".into(),
            1234567890,
        );

        let items: Vec<TreeItem> = files
            .into_iter()
            .map(|entry| TreeItem::File {
                entry,
                commit: commit.clone(),
            })
            .collect();

        let html = generate(dir.path(), "HEAD", "", "test-repo", &items).unwrap();

        let html_str = html.into_string();
        assert!(html_str.contains("test-repo"));
        assert!(html_str.contains("file.txt"));
    }
}
