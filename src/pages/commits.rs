//! Commits page generation for commit history viewing

use maud::{Markup, html};

use crate::components::layout::page_wrapper;
use crate::components::nav::breadcrumb;
use crate::git::PaginatedCommits;
use crate::util::{calculate_depth, format_timestamp};

/// Generates HTML page displaying commit log for a reference
///
/// Creates a commit history page showing commit metadata in reverse
/// chronological order with relative timestamps and pagination controls.
///
/// # Arguments
///
/// * `paginated`: Paginated commit data with page metadata
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
/// use gitkyl::pages::commits::generate;
/// use gitkyl::list_commits_paginated;
/// use std::path::Path;
///
/// let paginated = list_commits_paginated(Path::new("."), Some("main"), 1, 35)?;
/// let html = generate(&paginated, "main", "my-repo");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate(paginated: &PaginatedCommits, ref_name: &str, repo_name: &str) -> Markup {
    let depth = calculate_depth(ref_name, "");
    let css_path = format!("{}assets/commits.css", "../".repeat(depth));
    let index_path = format!("{}index.html", "../".repeat(depth));

    let title = format!("{}/{}: commits", repo_name, ref_name);

    page_wrapper(
        &title,
        &[&css_path],
        html! {
            (breadcrumb(repo_name, &index_path, &[("Commits", None)], ref_name))
            main {
                        h1 { "Commit History" }
                        div class="commit-count" {
                            "Showing " (paginated.commits.len()) " commits"
                        }
                        @if paginated.commits.is_empty() {
                            p class="empty-state" { "No commits found" }
                        } @else {
                            ol class="commit-list" {
                                @for commit in &paginated.commits {
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
                                                (format_timestamp(commit.date()))
                                            }
                                        }
                                    }
                                }
                            }
                            (pagination_controls(paginated))
                        }
            }
        },
    )
}

/// Generates pagination controls for commit history navigation
///
/// Renders previous/next page links with proper disabled states.
/// Only renders if there are pages to navigate to (has_prev or has_next).
///
/// # Arguments
///
/// * `paginated`: Paginated commit data with page metadata
///
/// # Returns
///
/// Rendered HTML markup for pagination controls
fn pagination_controls(paginated: &PaginatedCommits) -> Markup {
    let has_prev = paginated.page > 1;
    let has_next = paginated.has_more;

    if !has_prev && !has_next {
        return html! {};
    }

    html! {
        nav class="pagination" {
            @if has_prev {
                a class="pagination-prev" href=(format!("page-{}.html", paginated.page - 1)) {
                    "← Previous"
                }
            } @else {
                span class="pagination-prev disabled" {
                    "← Previous"
                }
            }

            span class="pagination-info" {
                "Page " (paginated.page)
            }

            @if has_next {
                a class="pagination-next" href=(format!("page-{}.html", paginated.page + 1)) {
                    "Next →"
                }
            } @else {
                span class="pagination-next disabled" {
                    "Next →"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{CommitInfo, PaginatedCommits};
    use std::fs;

    #[test]
    fn test_commits_page_generation_workflow() {
        use tempfile;

        // Arrange: Create mock commits with known data
        let temp_dir = tempfile::tempdir().expect("Should create temp directory");
        let output = temp_dir.path();

        let mock_commits = vec![
            CommitInfo::new(
                "abc123".to_string(),
                "Add feature X".to_string(),
                "Add feature X\n\nDetailed description.".to_string(),
                "Alice".to_string(),
                1234567890,
            ),
            CommitInfo::new(
                "def456".to_string(),
                "Fix bug Y".to_string(),
                "Fix bug Y".to_string(),
                "Bob".to_string(),
                1234567800,
            ),
            CommitInfo::new(
                "ghi789".to_string(),
                "Refactor module Z".to_string(),
                "Refactor module Z".to_string(),
                "Carol".to_string(),
                1234567700,
            ),
        ];

        let paginated = PaginatedCommits::new(mock_commits.clone(), 1, 100, false);
        let branch_name = "main";
        let repo_name = "test-repo";

        // Act
        let html = generate(&paginated, branch_name, repo_name);

        let commits_dir = output.join("commits").join(branch_name);
        fs::create_dir_all(&commits_dir).expect("Should create commits directory");

        let commits_path = commits_dir.join("index.html");
        fs::write(&commits_path, html.into_string()).expect("Should write commits page");

        // Assert
        assert!(commits_path.exists(), "Commits page should be created");

        let content = fs::read_to_string(&commits_path).expect("Should read commits page");

        assert!(
            content.contains("Commit History"),
            "Should contain commit log title"
        );
        assert!(content.contains(branch_name), "Should contain branch name");
        assert!(
            content.contains("Add feature X"),
            "Should contain first commit message"
        );
        assert!(
            content.contains("Alice"),
            "Should contain first commit author"
        );
        assert_eq!(mock_commits.len(), 3, "Should have exactly 3 test commits");
    }

    #[test]
    fn test_pagination_controls_first_page_with_more() {
        // Arrange: First page with more commits available
        let commits = vec![CommitInfo::new(
            "a".into(),
            "msg".into(),
            "msg".into(),
            "author".into(),
            123,
        )];
        let paginated = PaginatedCommits::new(commits, 1, 10, true);

        // Act
        let html = pagination_controls(&paginated).into_string();

        // Assert
        assert!(html.contains("pagination"), "Should render pagination");
        assert!(html.contains("disabled"), "Previous should be disabled");
        assert!(html.contains("page-2.html"), "Next should link to page 2");
        assert!(html.contains("Page 1"), "Should show current page");
    }

    #[test]
    fn test_pagination_controls_middle_page() {
        // Arrange: Middle page with both prev and next
        let commits = vec![CommitInfo::new(
            "a".into(),
            "msg".into(),
            "msg".into(),
            "author".into(),
            123,
        )];
        let paginated = PaginatedCommits::new(commits, 5, 10, true);

        // Act
        let html = pagination_controls(&paginated).into_string();

        // Assert
        assert!(
            html.contains("page-4.html"),
            "Previous should link to page 4"
        );
        assert!(html.contains("page-6.html"), "Next should link to page 6");
        assert!(html.contains("Page 5"), "Should show current page");
        assert!(!html.contains("disabled"), "No button should be disabled");
    }

    #[test]
    fn test_pagination_controls_last_page() {
        // Arrange: Last page with no more commits
        let commits = vec![CommitInfo::new(
            "a".into(),
            "msg".into(),
            "msg".into(),
            "author".into(),
            123,
        )];
        let paginated = PaginatedCommits::new(commits, 3, 10, false);

        // Act
        let html = pagination_controls(&paginated).into_string();

        // Assert
        assert!(
            html.contains("page-2.html"),
            "Previous should link to page 2"
        );
        assert!(html.contains("disabled"), "Next should be disabled");
        assert!(html.contains("Page 3"), "Should show current page");
    }

    #[test]
    fn test_pagination_controls_single_page() {
        // Arrange: Only one page, no navigation needed
        let commits = vec![CommitInfo::new(
            "a".into(),
            "msg".into(),
            "msg".into(),
            "author".into(),
            123,
        )];
        let paginated = PaginatedCommits::new(commits, 1, 10, false);

        // Act
        let html = pagination_controls(&paginated).into_string();

        // Assert
        assert!(
            html.is_empty(),
            "Should not render pagination for single page"
        );
    }

    #[test]
    fn test_pagination_controls_only_next() {
        // Arrange: First page with more pages (edge case verification)
        let commits = vec![CommitInfo::new(
            "a".into(),
            "msg".into(),
            "msg".into(),
            "author".into(),
            123,
        )];
        let paginated = PaginatedCommits::new(commits, 1, 10, true);

        // Act
        let html = pagination_controls(&paginated).into_string();

        // Assert
        assert!(
            html.contains("pagination-prev disabled"),
            "Prev button exists but disabled"
        );
        assert!(
            html.contains("<a class=\"pagination-next\""),
            "Next button is link"
        );
    }
}
