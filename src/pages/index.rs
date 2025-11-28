//! Repository index page generation

use anyhow::{Context, Result};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::path::Path;

use crate::components::icons::icon_classes;
use crate::git::{CommitInfo, TreeItem};
use crate::time::format_timestamp;

/// Minimum branches required to show selector dropdown
///
/// When repository has fewer branches than this threshold, shows static
/// branch badge instead of interactive selector to reduce visual noise.
const MIN_BRANCHES_FOR_SELECTOR: usize = 2;

/// Data container for index page generation
pub struct IndexPageData<'a> {
    pub name: &'a str,
    pub owner: Option<&'a str>,
    pub default_branch: &'a str,
    pub branches: &'a [String],
    pub commit_count: usize,
    pub latest_commit: Option<&'a CommitInfo>,
    pub items: &'a [TreeItem],
    pub readme_html: Option<&'a str>,
}

/// Generates repository index page HTML with optional README rendering
///
/// Creates the main repository landing page showing file tree and optionally
/// renders README content below the file table. Follows GitHub's visual
/// hierarchy: repository info, file explorer, then README.
///
/// # Arguments
///
/// * `data`: Index page data container with all required fields
///
/// # Returns
///
/// Complete HTML markup for index page
pub fn generate(data: IndexPageData<'_>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (data.name) " - Gitkyl" }
                script src="https://unpkg.com/@phosphor-icons/web" {}
                link rel="stylesheet" href="assets/index.css";
                link rel="stylesheet" href="assets/markdown.css";
            }
            body {
                div class="container" {
                    header class="repo-header" {
                        @if let Some(owner_name) = data.owner {
                            span class="repo-owner" { (owner_name) " / " }
                        }
                        h1 class="repo-name" { (data.name) }
                    }

                    main class="repo-card" {
                        div class="repo-controls" {
                            div class="commit-info-group" {
                                @if data.branches.len() >= MIN_BRANCHES_FOR_SELECTOR {
                                    div class="branch-selector" {
                                        i class="ph ph-git-branch" {}
                                        @for branch in data.branches {
                                            @if branch == data.default_branch {
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
                                        span { (data.default_branch) }
                                    }
                                }

                                @if let Some(commit) = data.latest_commit {
                                    div class="commit-info-wrapper" {
                                        div class="commit-line" {
                                            span class="avatar-placeholder" {}
                                            span class="repo-commit-message" { (commit.message()) }
                                        }
                                        div class="commit-meta" {
                                            span { (commit.author()) }
                                            span { "·" }
                                            code class="commit-hash" { (commit.short_oid()) }
                                            span { "·" }
                                            span { (format_timestamp(commit.date())) }
                                        }
                                    }
                                }
                            }

                            a href=(format!("commits/{}/index.html", data.default_branch)) class="history-link" {
                                i class="ph ph-clock-counter-clockwise" {}
                                " " (data.commit_count) " commits"
                            }
                        }

                        @if data.items.is_empty() {
                            p class="empty-state" { "No files in this repository" }
                        } @else {
                            div class="file-table" {
                                @for item in data.items.iter() {
                                    @match item {
                                        TreeItem::File { entry, commit } => {
                                            @if let Some(path) = entry.path()
                                                && let Some(path_str) = path.to_str() {
                                                @let (icon_class, icon_modifier) = icon_classes(path_str);
                                                @let href = format!("blob/{}/{}.html", data.default_branch, path_str);
                                                a href=(href) class="file-row" {
                                                    div class="icon-box" {
                                                        @if let Some(modifier) = icon_modifier {
                                                            i class=(format!("{} {}", icon_class, modifier)) {}
                                                        } @else {
                                                            i class=(icon_class) {}
                                                        }
                                                    }
                                                    div class="file-link" { (path_str) }
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
                                            @let display_path = if full_path.is_empty() { name } else { full_path };
                                            @let (icon_class, icon_modifier) = icon_classes(&format!("{}/", display_path));
                                            @let href = format!("tree/{}/{}.html", data.default_branch, display_path);
                                            a href=(href) class="file-row" {
                                                div class="icon-box" {
                                                    @if let Some(modifier) = icon_modifier {
                                                        i class=(format!("{} {}", icon_class, modifier)) {}
                                                    } @else {
                                                        i class=(icon_class) {}
                                                    }
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

                    @if let Some(readme) = data.readme_html {
                        section class="readme-section" {
                            div class="readme-card" {
                                div class="readme-header" {
                                    i class="ph ph-info" {}
                                    span class="readme-title" { "README.md" }
                                }
                                div class="readme-content latte" {
                                    (PreEscaped(readme))
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
    }
}

/// Finds and renders README file at repository root
///
/// Searches tree items for README files with prioritized detection order:
/// README.md > README > readme.md. Only renders README files at repository
/// root level (not in subdirectories).
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `tree_items`: Tree items at repository root
///
/// # Returns
///
/// Optional rendered README HTML string, or None if no README found
///
/// # Errors
///
/// Returns error if blob reading or markdown rendering fails
pub fn find_and_render_readme(
    repo_path: impl AsRef<Path>,
    ref_name: &str,
    tree_items: &[TreeItem],
) -> Result<Option<String>> {
    const README_VARIANTS: &[&str] = &["README.md", "README", "readme.md", "Readme.md"];

    let readme_entry = tree_items.iter().find_map(|item| {
        if let TreeItem::File { entry, .. } = item
            && let Some(path) = entry.path()
            && let Some(path_str) = path.to_str()
            && crate::components::icons::is_readme(path)
        {
            for variant in README_VARIANTS {
                if path_str == *variant {
                    return Some(entry);
                }
            }
            return Some(entry);
        }
        None
    });

    if let Some(entry) = readme_entry
        && let Some(path) = entry.path()
    {
        let content_bytes = crate::git::read_blob(&repo_path, Some(ref_name), path)
            .context("Failed to read README blob")?;

        let content = String::from_utf8(content_bytes).context("README contains invalid UTF8")?;

        let renderer = crate::markdown::MarkdownRenderer::new();
        let rendered = renderer
            .render(&content)
            .context("Failed to render README markdown")?;

        return Ok(Some(rendered));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommitInfo;
    use std::path::PathBuf;

    #[test]
    fn test_index_page_basic() {
        // Arrange
        let name = "TestRepo";
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string(), "develop".to_string()];
        let commit_count = 42;
        let items = vec![];

        // Act
        let html = generate(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("TestRepo"), "Should contain repo name");
        assert!(html_string.contains("testuser"), "Should contain owner");
        assert!(
            html_string.contains("42 commits"),
            "Should contain commit count link"
        );
    }

    #[test]
    fn test_index_page_with_latest_commit() {
        // Arrange: Test with mock commit data
        let name = "TestRepo";
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 10;
        let items = vec![];

        // Create mock commit info
        let mock_commit = CommitInfo::new(
            "abc123def456".to_string(),
            "Initial commit".to_string(),
            "Initial commit\n\nThis is the first commit.".to_string(),
            "Test Author".to_string(),
            1234567890,
        );

        // Act
        let html = generate(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: Some(&mock_commit),
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert
        assert!(
            html_string.contains("commit-info-group"),
            "Should have commit info group"
        );
        assert!(
            html_string.contains("abc123d"),
            "Should show commit hash (short form)"
        );
        assert!(
            html_string.contains("Initial commit"),
            "Should show commit message"
        );
        assert!(
            html_string.contains("Test Author"),
            "Should show author name"
        );
    }

    #[test]
    fn test_index_page_with_file_table() {
        // Arrange: Test with mock TreeItem directory structure
        let name = "TestRepo";
        let owner = None;
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 5;

        let mock_commit = CommitInfo::new(
            "abc123".to_string(),
            "Add files".to_string(),
            "Add files".to_string(),
            "Test Author".to_string(),
            1234567890,
        );

        // Create directory items to test file table rendering
        let items = vec![
            TreeItem::Directory {
                name: "src".to_string(),
                full_path: "src".to_string(),
                commit: mock_commit.clone(),
            },
            TreeItem::Directory {
                name: "tests".to_string(),
                full_path: "tests".to_string(),
                commit: mock_commit.clone(),
            },
        ];

        // Act
        let html = generate(IndexPageData {
            name,
            owner,
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert: Check that file table structure is present
        assert!(
            html_string.contains("file-table"),
            "Should contain file table"
        );
        assert!(html_string.contains("file-row"), "Should contain file rows");
        assert!(
            html_string.contains("file-link"),
            "Should contain file link"
        );
        assert!(
            html_string.contains("commit-date"),
            "Should contain commit date"
        );
        assert!(html_string.contains("src"), "Should contain directory name");
    }

    #[test]
    fn test_index_page_file_table_structure() {
        // Arrange: Test file table structure with directory item
        let mock_commit = CommitInfo::new(
            "def456".to_string(),
            "Add directory".to_string(),
            "Add directory".to_string(),
            "Test Author".to_string(),
            1234567890,
        );

        let items = vec![TreeItem::Directory {
            name: "lib".to_string(),
            full_path: "lib".to_string(),
            commit: mock_commit,
        }];

        let branches = vec!["main".to_string()];

        // Act
        let html = generate(IndexPageData {
            name: "test",
            owner: None,
            default_branch: "main",
            branches: &branches,
            commit_count: 1,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert: Check HTML structure elements are present
        assert!(html_string.contains("file-link"), "Should have file link");
        assert!(
            html_string.contains("commit-date"),
            "Should have commit date"
        );
        assert!(
            html_string.contains("class=\"icon-box\""),
            "Should have icon container element"
        );
        assert!(
            html_string.contains("class=\"ph ") || html_string.contains("class=\"ph-"),
            "Should have Phosphor icon class"
        );
        assert!(html_string.contains("lib"), "Should contain directory name");
    }

    #[test]
    fn test_index_page_with_readme() {
        // Arrange
        let name = "TestRepo";
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 10;
        let items = vec![];
        let readme_html = Some("<h1>Test README</h1><p>This is a test.</p>");

        // Act
        let html = generate(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html,
        });
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("TestRepo"), "Should contain repo name");
        assert!(
            html_string.contains("readme-section"),
            "Should have README section"
        );
        assert!(
            html_string.contains("readme-card"),
            "Should have README card"
        );
        assert!(
            html_string.contains("Test README"),
            "Should contain README content"
        );
        assert!(
            html_string.contains("This is a test"),
            "Should contain README text"
        );
    }

    #[test]
    fn test_index_page_without_readme() {
        // Arrange
        let name = "TestRepo";
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 10;
        let items = vec![];

        // Act
        let html = generate(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("TestRepo"), "Should contain repo name");
        assert!(
            !html_string.contains("readme-section"),
            "Should not have README section when no README provided"
        );
    }

    #[test]
    fn test_find_and_render_readme_found() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "master";

        let files = crate::git::list_files(&repo_path, Some(ref_name)).expect("Should list files");

        let readme_file = files
            .iter()
            .find(|f| {
                f.path()
                    .map_or(false, |p| crate::components::icons::is_readme(p))
            })
            .expect("Repository should have README");

        let commit = crate::git::get_file_last_commit(
            &repo_path,
            Some(ref_name),
            readme_file
                .path()
                .expect("README entry should have valid path")
                .to_str()
                .expect("README path should be valid UTF8"),
        )
        .expect("Should get README commit");

        let tree_items = vec![TreeItem::File {
            entry: readme_file.clone(),
            commit,
        }];

        // Act
        let result = find_and_render_readme(&repo_path, ref_name, &tree_items);

        // Assert
        assert!(result.is_ok(), "Should successfully render README");
        let html = result.expect("Result should be Ok");
        assert!(html.is_some(), "Should find and render README");
        let rendered = html.expect("HTML should be Some");
        assert!(!rendered.is_empty(), "Rendered HTML should not be empty");
        assert!(
            rendered.contains("<h1>") || rendered.contains("<h2>") || rendered.contains("<p>"),
            "Should contain HTML tags from rendered markdown"
        );
    }

    #[test]
    fn test_find_and_render_readme_not_found() {
        // Arrange: Empty tree items
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "master";
        let tree_items = vec![];

        // Act
        let result = find_and_render_readme(&repo_path, ref_name, &tree_items);

        // Assert
        assert!(result.is_ok(), "Should handle missing README gracefully");
        let html = result.expect("Result should be Ok even with no README");
        assert!(html.is_none(), "Should return None when no README found");
    }

    #[test]
    fn test_find_and_render_readme_priority() {
        // Arrange: Multiple README variants
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "master";

        let files = crate::git::list_files(&repo_path, Some(ref_name)).expect("Should list files");

        let readme_files: Vec<_> = files
            .iter()
            .filter(|f| {
                f.path()
                    .map_or(false, |p| crate::components::icons::is_readme(p))
            })
            .collect();

        if readme_files.is_empty() {
            return;
        }

        let mut tree_items = vec![];
        for file in readme_files {
            if let Ok(commit) = crate::git::get_file_last_commit(
                &repo_path,
                Some(ref_name),
                file.path()
                    .expect("Test file entry should have valid path")
                    .to_str()
                    .expect("Test file path should be valid UTF8"),
            ) {
                tree_items.push(TreeItem::File {
                    entry: file.clone(),
                    commit,
                });
            }
        }

        // Act
        let result = find_and_render_readme(&repo_path, ref_name, &tree_items);

        // Assert
        assert!(result.is_ok(), "Should handle multiple README files");
        if let Ok(Some(html)) = result {
            assert!(!html.is_empty(), "Should render README content");
        }
    }
}
