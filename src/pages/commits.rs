//! Commits page generation for commit history viewing

use maud::{Markup, html};

use crate::components::layout::page_wrapper;
use crate::components::nav::breadcrumb;
use crate::git::CommitInfo;
use crate::time::format_timestamp;

/// Generates HTML page displaying commit log for a reference
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
/// use gitkyl::pages::commits::generate;
/// use gitkyl::list_commits;
/// use std::path::Path;
///
/// let commits = list_commits(Path::new("."), None, Some(100))?;
/// let html = generate(&commits, "main", "my-repo");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate(commits: &[CommitInfo], ref_name: &str, repo_name: &str) -> Markup {
    // Commits pages are always at depth 2: commits/<branch>/index.html
    let depth = 2;
    let css_path = format!("{}assets/commits.css", "../".repeat(depth));
    let index_path = format!("{}index.html", "../".repeat(depth));

    let title = format!("Commits - {}", repo_name);

    page_wrapper(
        &title,
        &[&css_path],
        html! {
            (breadcrumb(repo_name, &index_path, &[("Commits", None)], ref_name))
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
                                                (format_timestamp(commit.date()))
                                            }
                                        }
                                    }
                                }
                            }
                        }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::CommitInfo;
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

        let branch_name = "main";
        let repo_name = "test-repo";

        // Act
        let html = generate(&mock_commits, branch_name, repo_name);

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
}
