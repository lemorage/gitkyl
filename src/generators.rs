//! HTML page generators for repository content.

use anyhow::{Context, Result};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::git::{CommitInfo, FileEntry, read_blob};
use crate::highlight::highlight;

/// Generates HTML blob page with syntax highlighting.
///
/// Reads blob content from the repository at the specified reference and path,
/// applies tree-sitter syntax highlighting, and renders as HTML with line numbers.
/// The output follows GitHub's visual design patterns.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `file_path`: File path within repository tree
///
/// # Returns
///
/// HTML markup ready for writing to disk
///
/// # Errors
///
/// Returns error if:
/// - Blob cannot be read from repository
/// - File content contains invalid UTF8
/// - Syntax highlighting fails
///
/// # Examples
///
/// ```no_run
/// use gitkyl::generate_blob_page;
/// use std::path::Path;
///
/// let html = generate_blob_page(
///     Path::new("."),
///     "main",
///     Path::new("src/lib.rs")
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate_blob_page(
    repo_path: impl AsRef<Path>,
    ref_name: &str,
    file_path: impl AsRef<Path>,
) -> Result<Markup> {
    let content_bytes = read_blob(&repo_path, Some(ref_name), &file_path)
        .context("Failed to read blob from repository")?;

    let content = String::from_utf8(content_bytes).context("Blob contains invalid UTF8")?;

    let highlighted =
        highlight(&content, file_path.as_ref()).context("Failed to apply syntax highlighting")?;

    let path_str = file_path.as_ref().display().to_string();
    let path_components = extract_breadcrumb_components(&path_str);

    Ok(blob_page_markup(
        &path_str,
        &path_components,
        ref_name,
        &highlighted,
    ))
}

/// Generates HTML page displaying commit log for a reference.
///
/// Creates a commit history page showing commit metadata in reverse
/// chronological order with relative timestamps.
///
/// # Arguments
///
/// * `commits`: Vector of commit information to display
/// * `ref_name`: Reference name (branch/tag) for page title
/// * `repo_name`: Repository name for navigation
///
/// # Returns
///
/// Rendered HTML markup
///
/// # Examples
///
/// ```no_run
/// use gitkyl::{list_commits, generate_commits_page};
/// use std::path::Path;
///
/// let commits = list_commits(Path::new("."), None, Some(100))?;
/// let html = generate_commits_page(&commits, "main", "my-repo");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate_commits_page(commits: &[CommitInfo], ref_name: &str, repo_name: &str) -> Markup {
    // Commits pages are always at depth 2: commits/<branch>/index.html
    let depth = 2;
    let css_path = format!("{}assets/commits.css", "../".repeat(depth));
    let index_path = format!("{}index.html", "../".repeat(depth));

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Commits - " (repo_name) " - Gitkyl" }
                link rel="stylesheet" href=(css_path);
            }
            body {
                div class="container" {
                    header {
                        div class="breadcrumb" {
                            a href=(index_path) class="breadcrumb-link" { "Repository" }
                            span class="breadcrumb-separator" { "/" }
                            span class="breadcrumb-current" { "Commits" }
                            span class="ref-badge" { (ref_name) }
                        }
                    }
                    main {
                        h1 { "Commit History" }
                        div class="commit-count" {
                            "Showing " (commits.len()) " commits"
                        }
                        @if commits.is_empty() {
                            p class="empty-state" { "No commits found" }
                        } @else {
                            ol class="commit-list" {
                                @for commit in commits {
                                    li class="commit-entry" {
                                        div class="commit-header" {
                                            span class="commit-hash" {
                                                code { (commit.short_oid()) }
                                            }
                                            span class="commit-message" { (commit.message()) }
                                        }
                                        div class="commit-meta" {
                                            span class="commit-author" {
                                                (commit.author())
                                            }
                                            span class="commit-date" {
                                                (format_timestamp(commit.date(), SystemTime::now()))
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
    }
}

/// Represents an item in a directory tree view.
///
/// Distinguishes between regular files (git blobs) and directories (git trees)
/// with proper semantic representation and commit metadata.
#[derive(Debug, Clone)]
pub enum TreeItem {
    /// Regular file with its last modifying commit
    File {
        entry: FileEntry,
        commit: CommitInfo,
    },
    /// Directory with its most recent commit
    Directory {
        name: String,
        full_path: String,
        commit: CommitInfo,
    },
}

/// Generates HTML tree page for directory browsing.
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
/// use gitkyl::{generate_tree_page, TreeItem};
/// use std::path::Path;
///
/// let items = vec![]; // Populate with TreeItem instances
/// let html = generate_tree_page(Path::new("."), "main", "", "my-repo", &items)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate_tree_page(
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
                            a href=(index_path) class="breadcrumb-link" { "Repository" }
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
                                                    div class="file-meta" { (format_timestamp(commit.date(), SystemTime::now())) }
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
                                                div class="file-meta" { (format_timestamp(commit.date(), SystemTime::now())) }
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

/// Formats Unix timestamp as human readable relative time.
///
/// Converts Unix timestamp to relative time strings like "2 days ago"
/// or "3 weeks ago" for improved readability.
///
/// # Arguments
///
/// * `seconds`: Unix timestamp in seconds since epoch
///
/// # Returns
///
/// Human readable relative time string
fn format_timestamp(seconds: i64, now: SystemTime) -> String {
    let timestamp = UNIX_EPOCH + Duration::from_secs(seconds as u64);

    if let Ok(duration) = now.duration_since(timestamp) {
        let days = duration.as_secs() / 86400;
        if days == 0 {
            return "today".to_string();
        } else if days == 1 {
            return "yesterday".to_string();
        } else if days < 7 {
            return format!("{} days ago", days);
        } else if days < 30 {
            return format!("{} weeks ago", days / 7);
        } else if days < 365 {
            return format!("{} months ago", days / 30);
        } else {
            return format!("{} years ago", days / 365);
        }
    }

    "unknown".to_string()
}

/// Extracts breadcrumb path components from file path.
fn extract_breadcrumb_components(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Renders blob page HTML structure.
fn blob_page_markup(
    file_path: &str,
    breadcrumb_components: &[&str],
    ref_name: &str,
    highlighted_code: &str,
) -> Markup {
    let lines: Vec<&str> = highlighted_code.lines().collect();
    let line_count = lines.len().max(1);

    // Calculate relative path back to index.html based on depth
    // Depth = blob/ + branch/ + path directories
    let depth = breadcrumb_components.len() + 1;
    let index_path = "../".repeat(depth) + "index.html";

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (file_path) " - Gitkyl" }
                link rel="stylesheet" href=(format!("{}assets/blob.css", "../".repeat(depth)));
            }
            body {
                div class="container" {
                    header {
                        div class="breadcrumb" {
                            a href=(index_path) class="breadcrumb-link" { "Repository" }
                            @for (idx, component) in breadcrumb_components.iter().enumerate() {
                                span class="breadcrumb-separator" { "/" }
                                @if idx == breadcrumb_components.len() - 1 {
                                    span class="breadcrumb-current" { (*component) }
                                } @else {
                                    span class="breadcrumb-link" { (*component) }
                                }
                            }
                        }
                        div class="ref-info" {
                            span class="ref-label" { "ref: " }
                            span class="ref-name" { (ref_name) }
                        }
                    }
                    main class="blob-container" {
                        div class="line-numbers" {
                            @for line_num in 1..=line_count {
                                div class="line-number" { (line_num) }
                            }
                        }
                        div class="code-content" {
                            pre {
                                code {
                                    @for line in lines {
                                        div class="code-line" {
                                            (PreEscaped(line))
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Helper function for creating test commits
    fn create_test_commit(oid: String, message: String, author: String, date: i64) -> CommitInfo {
        CommitInfo::new_for_test(
            oid.clone(),
            oid[..7].to_string(),
            author.clone(),
            format!("{}@example.com", author.to_lowercase().replace(' ', "")),
            author.clone(),
            date,
            message.clone(),
            message,
        )
    }

    #[test]
    fn test_extract_breadcrumb_components_simple() {
        // Arrange
        let path = "src/main.rs";

        // Act
        let components = extract_breadcrumb_components(path);

        // Assert
        assert_eq!(components, vec!["src", "main.rs"]);
    }

    #[test]
    fn test_extract_breadcrumb_components_nested() {
        // Arrange
        let path = "src/generators/html.rs";

        // Act
        let components = extract_breadcrumb_components(path);

        // Assert
        assert_eq!(components, vec!["src", "generators", "html.rs"]);
    }

    #[test]
    fn test_extract_breadcrumb_components_single() {
        // Arrange
        let path = "README.md";

        // Act
        let components = extract_breadcrumb_components(path);

        // Assert
        assert_eq!(components, vec!["README.md"]);
    }

    #[test]
    fn test_extract_breadcrumb_components_empty() {
        // Arrange
        let path = "";

        // Act
        let components = extract_breadcrumb_components(path);

        // Assert
        assert!(components.is_empty());
    }

    #[test]
    fn test_blob_page_markup_structure() {
        // Arrange
        let file_path = "src/lib.rs";
        let breadcrumb = vec!["src", "lib.rs"];
        let ref_name = "main";
        let code = "<span class=\"hl-keyword\">fn</span> main() {}";

        // Act
        let html = blob_page_markup(file_path, &breadcrumb, ref_name, code);
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("src/lib.rs"));
        assert!(html_string.contains("main"));
        assert!(html_string.contains("hl-keyword"));
        assert!(html_string.contains("Repository"));
        assert!(html_string.contains("Gitkyl"));
    }

    #[test]
    fn test_blob_page_markup_breadcrumb() {
        // Arrange
        let file_path = "tests/integration/test.rs";
        let breadcrumb = vec!["tests", "integration", "test.rs"];
        let ref_name = "develop";
        let code = "test code";

        // Act
        let html = blob_page_markup(file_path, &breadcrumb, ref_name, code);
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("tests"));
        assert!(html_string.contains("integration"));
        assert!(html_string.contains("test.rs"));
        assert!(html_string.contains("breadcrumb-separator"));
    }

    #[test]
    fn test_blob_page_markup_ref_info() {
        // Arrange
        let file_path = "config.toml";
        let breadcrumb = vec!["config.toml"];
        let ref_name = "feature/new-parser";
        let code = "content";

        // Act
        let html = blob_page_markup(file_path, &breadcrumb, ref_name, code);
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("ref:"));
        assert!(html_string.contains("feature/new-parser"));
        assert!(html_string.contains("ref-name"));
    }

    #[test]
    fn test_blob_page_markup_line_numbers() {
        // Arrange
        let file_path = "test.rs";
        let breadcrumb = vec!["test.rs"];
        let ref_name = "main";
        let code = "line 1\nline 2\nline 3";

        // Act
        let html = blob_page_markup(file_path, &breadcrumb, ref_name, code);
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("line-number"));
        assert!(html_string.contains("line 1"));
        assert!(html_string.contains("line 2"));
        assert!(html_string.contains("line 3"));
    }

    #[test]
    fn test_blob_page_markup_empty_code() {
        // Arrange
        let file_path = "empty.txt";
        let breadcrumb = vec!["empty.txt"];
        let ref_name = "main";
        let code = "";

        // Act
        let html = blob_page_markup(file_path, &breadcrumb, ref_name, code);
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("empty.txt"));
        assert!(html_string.contains("blob-container"));
    }

    #[test]
    fn test_blob_page_markup_single_line() {
        // Arrange
        let file_path = "single.txt";
        let breadcrumb = vec!["single.txt"];
        let ref_name = "main";
        let code = "single line";

        // Act
        let html = blob_page_markup(file_path, &breadcrumb, ref_name, code);
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("single line"));
        assert!(html_string.contains("line-number"));
    }

    #[test]
    fn test_generate_blob_page_integration() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "HEAD";
        let file_path = Path::new("Cargo.toml");

        // Act
        let result = generate_blob_page(&repo_path, ref_name, file_path);

        // Assert
        assert!(
            result.is_ok(),
            "Should generate blob page for existing file"
        );
        let html = result.unwrap().into_string();
        assert!(html.contains("Cargo.toml"));
        assert!(html.contains("blob-container"));
    }

    #[test]
    fn test_generate_blob_page_rust_syntax() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "HEAD";
        let file_path = Path::new("src/lib.rs");

        // Act
        let result = generate_blob_page(&repo_path, ref_name, file_path);

        // Assert
        assert!(
            result.is_ok(),
            "Should generate blob page with syntax highlighting"
        );
        let html = result.unwrap().into_string();
        assert!(html.contains("src/lib.rs"));
        assert!(
            html.contains("hl-") || html.contains("line-number"),
            "Should contain highlighting or line numbers"
        );
    }

    #[test]
    fn test_generate_blob_page_nonexistent_file() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "HEAD";
        let file_path = Path::new("nonexistent_file_12345.txt");

        // Act
        let result = generate_blob_page(&repo_path, ref_name, file_path);

        // Assert
        assert!(result.is_err(), "Should fail for nonexistent file");
    }

    #[test]
    fn test_generate_blob_page_invalid_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "invalid_ref_that_does_not_exist_12345";
        let file_path = Path::new("Cargo.toml");

        // Act
        let result = generate_blob_page(&repo_path, ref_name, file_path);

        // Assert
        assert!(result.is_err(), "Should fail for invalid reference");
    }

    #[test]
    fn test_generate_commits_page_structure() {
        // Arrange
        let commits = vec![
            create_test_commit(
                "a".repeat(40),
                "Initial commit".to_string(),
                "Test Author".to_string(),
                1704067200,
            ),
            create_test_commit(
                "b".repeat(40),
                "Second commit".to_string(),
                "Another Author".to_string(),
                1704153600,
            ),
        ];

        // Act
        let html = generate_commits_page(&commits, "main", "test-repo");
        let html_str = html.into_string();

        // Assert
        assert!(
            html_str.contains("Commit History"),
            "Should contain page title"
        );
        assert!(
            html_str.contains("Showing 2 commits"),
            "Should show commit count"
        );
        assert!(
            html_str.contains(&"a".repeat(7)),
            "Should display first commit short hash"
        );
        assert!(
            html_str.contains("Initial commit"),
            "Should display first commit message"
        );
        assert!(html_str.contains("Test Author"), "Should display author");
    }

    #[test]
    fn test_generate_commits_page_empty() {
        // Arrange
        let commits = vec![];

        // Act
        let html = generate_commits_page(&commits, "main", "empty-repo");
        let html_str = html.into_string();

        // Assert
        assert!(
            html_str.contains("Showing 0 commits"),
            "Should handle empty commit list"
        );
        assert!(
            html_str.contains("No commits found"),
            "Should show empty state message"
        );
    }

    #[test]
    fn test_generate_commits_page_special_characters() {
        // Arrange
        let commits = vec![create_test_commit(
            "c".repeat(40),
            "Fix <script> & \"quotes\"".to_string(),
            "Test <User>".to_string(),
            1704067200,
        )];

        // Act
        let html = generate_commits_page(&commits, "main", "test-repo");
        let html_str = html.into_string();

        // Assert
        assert!(
            html_str.contains("&lt;script&gt;"),
            "Should HTML escape angle brackets in message"
        );
        assert!(html_str.contains("&amp;"), "Should HTML escape ampersands");
        assert!(html_str.contains("&quot;"), "Should HTML escape quotes");
        assert!(
            html_str.contains("&lt;User&gt;"),
            "Should HTML escape author name"
        );
    }

    #[test]
    fn test_format_timestamp_relative() {
        // Arrange: Current time minus 2 days
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_days_ago = (now - 172800) as i64;

        // Act
        let formatted = format_timestamp(two_days_ago, SystemTime::now());

        // Assert
        assert_eq!(formatted, "2 days ago", "Should format as relative days");
    }

    #[test]
    fn test_format_timestamp_today() {
        // Arrange: Current time
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Act
        let formatted = format_timestamp(now, SystemTime::now());

        // Assert
        assert_eq!(formatted, "today", "Should format current time as today");
    }

    #[test]
    fn test_format_timestamp_yesterday() {
        // Arrange: Current time minus 1 day
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let yesterday = (now - 86400) as i64;

        // Act
        let formatted = format_timestamp(yesterday, SystemTime::now());

        // Assert
        assert_eq!(formatted, "yesterday", "Should format yesterday correctly");
    }

    #[test]
    fn test_format_timestamp_weeks() {
        // Arrange: Current time minus 14 days
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_weeks_ago = (now - 1209600) as i64;

        // Act
        let formatted = format_timestamp(two_weeks_ago, SystemTime::now());

        // Assert
        assert_eq!(formatted, "2 weeks ago", "Should format weeks correctly");
    }

    #[test]
    fn test_format_timestamp_months() {
        // Arrange: Current time minus 60 days
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let sixty_days_ago = (now - 5184000) as i64;

        // Act
        let formatted = format_timestamp(sixty_days_ago, SystemTime::now());

        // Assert
        assert_eq!(formatted, "2 months ago", "Should format months correctly");
    }

    #[test]
    fn test_format_timestamp_years() {
        // Arrange: Current time minus 730 days
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_years_ago = (now - 63072000) as i64;

        // Act
        let formatted = format_timestamp(two_years_ago, SystemTime::now());

        // Assert
        assert_eq!(formatted, "2 years ago", "Should format years correctly");
    }

    #[test]
    fn test_format_timestamp_future() {
        // Arrange: Future timestamp
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let future = (now + 86400) as i64;

        // Act
        let formatted = format_timestamp(future, SystemTime::now());

        // Assert
        assert_eq!(
            formatted, "unknown",
            "Should return unknown for future timestamps"
        );
    }

    #[test]
    fn test_generate_commits_page_long_message() {
        // Arrange: Create commit with 5000 character message
        let long_message = "A".repeat(5000);
        let commits = vec![create_test_commit(
            "d".repeat(40),
            long_message.clone(),
            "Test Author".to_string(),
            1704067200,
        )];

        // Act
        let html = generate_commits_page(&commits, "main", "test-repo");
        let html_str = html.into_string();

        // Assert
        assert!(
            html_str.contains(&long_message),
            "Should render very long commit message"
        );
        assert!(
            html_str.contains("commit-message"),
            "Should use commit-message class for styling"
        );
        assert!(
            html_str.len() > 5000,
            "HTML should contain the full message"
        );
    }

    #[test]
    fn test_generate_commits_page_multiline_message() {
        // Arrange: Commit with multi-line message (only first line should display)
        let commits = vec![create_test_commit(
            "e".repeat(40),
            "First line summary".to_string(),
            "Test Author".to_string(),
            1704067200,
        )];

        // Act
        let html = generate_commits_page(&commits, "main", "test-repo");
        let html_str = html.into_string();

        // Assert
        assert!(
            html_str.contains("First line summary"),
            "Should display first line of commit message"
        );
    }

    #[test]
    fn test_generate_commits_page_message_with_special_unicode() {
        // Arrange: Commit with emoji and special Unicode characters
        let unicode_message = "🚀 Add feature with 日本語 and العربية support 🎉";
        let commits = vec![create_test_commit(
            "f".repeat(40),
            unicode_message.to_string(),
            "Test Author 👾".to_string(),
            1704067200,
        )];

        // Act
        let html = generate_commits_page(&commits, "main", "test-repo");
        let html_str = html.into_string();

        // Assert
        assert!(
            html_str.contains("🚀"),
            "Should handle emoji in commit message"
        );
        assert!(
            html_str.contains("日本語"),
            "Should handle Chinese characters"
        );
        assert!(
            html_str.contains("العربية"),
            "Should handle Arabic characters"
        );
        assert!(
            html_str.contains("👾"),
            "Should handle emoji in author name"
        );
        // Verify UTF-8 encoding is preserved
        assert!(
            html_str.contains("charset=\"utf-8\""),
            "Should declare UTF-8 encoding"
        );
    }

    #[test]
    fn test_generate_commits_page_empty_message() {
        // Arrange: Commit with empty message
        let commits = vec![create_test_commit(
            "g".repeat(40),
            String::new(), // Empty message
            "Test Author".to_string(),
            1704067200,
        )];

        // Act
        let html = generate_commits_page(&commits, "main", "test-repo");
        let html_str = html.into_string();

        // Assert
        assert!(
            html_str.contains("Showing 1 commits"),
            "Should still show commit count"
        );
        assert!(
            html_str.contains(&"g".repeat(7)),
            "Should display commit hash even with empty message"
        );
        assert!(
            html_str.contains("Test Author"),
            "Should display author even with empty message"
        );
    }

    #[test]
    fn test_generate_tree_page_structure() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::git::list_files(&repo_path, None).expect("Should list files");
        let file = files.first().expect("Should have at least one file");
        let commit = create_test_commit(
            "a".repeat(40),
            "Test commit".to_string(),
            "Test Author".to_string(),
            1704067200,
        );
        let items = vec![TreeItem::File {
            entry: file.clone(),
            commit,
        }];

        // Act
        let result = generate_tree_page(&repo_path, "main", "", "test-repo", &items);

        // Assert
        assert!(result.is_ok(), "Should generate tree page");
        let html = result.unwrap().into_string();
        assert!(
            html.contains("tree-container"),
            "Should have tree container"
        );
        assert!(html.contains("file-table"), "Should have file table");
        assert!(html.contains("breadcrumb"), "Should have breadcrumb");
    }

    #[test]
    fn test_generate_tree_page_empty() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let items = vec![];

        // Act
        let result = generate_tree_page(&repo_path, "main", "", "test-repo", &items);

        // Assert
        assert!(result.is_ok(), "Should handle empty directory");
        let html = result.unwrap().into_string();
        assert!(
            html.contains("Empty directory"),
            "Should show empty state message"
        );
    }

    #[test]
    fn test_generate_tree_page_nested_path() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::git::list_files(&repo_path, None).expect("Should list files");
        let file = files
            .iter()
            .find(|f| {
                f.path()
                    .and_then(|p| p.to_str())
                    .map_or(false, |s| s.starts_with("src/"))
            })
            .expect("Should have src files");
        let commit = create_test_commit(
            "b".repeat(40),
            "Test commit".to_string(),
            "Test Author".to_string(),
            1704067200,
        );
        let items = vec![TreeItem::File {
            entry: file.clone(),
            commit,
        }];

        // Act
        let result = generate_tree_page(&repo_path, "main", "src", "test-repo", &items);

        // Assert
        assert!(result.is_ok(), "Should generate nested tree page");
        let html = result.unwrap().into_string();
        assert!(html.contains("src"), "Should show directory name");
        assert!(html.contains("breadcrumb"), "Should have breadcrumb");
        assert!(html.contains(".."), "Should have parent directory link");
    }
}
