//! Commit detail page generation with full diff display

use anyhow::{Context, Result};
use maud::{Markup, PreEscaped, html};
use std::path::Path;

use crate::components::diff::{changed_files_list, diff_view, file_stats_summary};
use crate::components::layout::page_wrapper;
use crate::git::{CommitDiff, CommitInfo, get_commit_by_oid, get_commit_diff};
use crate::util::format_timestamp;

/// Generates commit detail page HTML showing metadata and full diff
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `commit_oid`: Full commit hash to display
/// * `repo_name`: Repository name for title
/// * `ref_name`: Branch/tag name for breadcrumb context
///
/// # Returns
///
/// Complete HTML page as string
///
/// # Errors
///
/// Returns error if:
/// - Commit cannot be found
/// - Diff computation fails
/// - Commit metadata cannot be read
pub fn generate_commit_page(
    repo_path: impl AsRef<Path>,
    commit_oid: &str,
    repo_name: &str,
    ref_name: Option<&str>,
) -> Result<String> {
    let repo_path = repo_path.as_ref();

    // Get commit metadata
    let commit =
        get_commit_by_oid(repo_path, commit_oid).context("Failed to retrieve commit metadata")?;

    // Get commit diff
    let diff = get_commit_diff(repo_path, commit_oid).context("Failed to compute commit diff")?;

    let page_title = format!(
        "{} - Commit {} - {}",
        commit.message(),
        commit.short_oid(),
        repo_name
    );

    // Commit pages are always at dist/commit/{hash}.html (depth 1)
    let css_path = "../assets/commit.css";
    let depth = 1;

    let content = commit_detail_content(&commit, &diff, repo_name, ref_name, depth);

    let html = page_wrapper(&page_title, &[css_path], content);

    Ok(html.into_string())
}

/// Renders commit detail page content with metadata and diff
fn commit_detail_content(
    commit: &CommitInfo,
    diff: &CommitDiff,
    repo_name: &str,
    ref_name: Option<&str>,
    depth: usize,
) -> Markup {
    html! {
        div class="commit-container" {
            (commit_breadcrumb(repo_name, commit, ref_name, depth))
            (commit_header(commit, diff))
            @if !diff.changed_files.is_empty() {
                (files_changed_section(&diff.changed_files))
                (diff_view(&diff.changed_files))
            } @else {
                div class="no-changes" {
                    "No file changes in this commit"
                }
            }
        }
        (copy_hash_script())
    }
}

/// Renders breadcrumb navigation for commit page
fn commit_breadcrumb(
    repo_name: &str,
    commit: &CommitInfo,
    ref_name: Option<&str>,
    depth: usize,
) -> Markup {
    let prefix = "../".repeat(depth);

    html! {
        nav class="breadcrumb" {
            a href=(format!("{}index.html", prefix)) class="breadcrumb-link" {
                (repo_name)
            }
            span class="breadcrumb-separator" { "/" }
            @if let Some(branch) = ref_name {
                a href=(format!("{}commits/{}/page-1.html", prefix, branch)) class="breadcrumb-link" {
                    "commits"
                }
                span class="breadcrumb-separator" { "/" }
            } @else {
                span class="breadcrumb-item" { "commit" }
                span class="breadcrumb-separator" { "/" }
            }
            span class="breadcrumb-item-active" {
                (commit.short_oid())
            }
        }
    }
}

/// Renders commit metadata header section
fn commit_header(commit: &CommitInfo, diff: &CommitDiff) -> Markup {
    html! {
        div class="commit-header-card" {
            div class="commit-message-section" {
                h1 class="commit-message-title" {
                    (commit.message())
                }
                @if commit.message() != commit.message_full() {
                    pre class="commit-message-body" {
                        (commit.message_full().trim_start_matches(commit.message()).trim())
                    }
                }
            }

            div class="commit-metadata" {
                div class="metadata-row" {
                    span class="metadata-label" { "Author:" }
                    span class="metadata-value" {
                        (commit.author())
                        @if !commit.author_email().is_empty() {
                            " <" (commit.author_email()) ">"
                        }
                    }
                }

                @if commit.author() != commit.committer() {
                    div class="metadata-row" {
                        span class="metadata-label" { "Committer:" }
                        span class="metadata-value" {
                            (commit.committer())
                        }
                    }
                }

                @if !commit.co_authors().is_empty() {
                    div class="metadata-row" {
                        span class="metadata-label" { "Co-authors:" }
                        span class="metadata-value" {
                            @for (idx, co_author) in commit.co_authors().iter().enumerate() {
                                @if idx > 0 { ", " }
                                (co_author)
                            }
                        }
                    }
                }

                div class="metadata-row" {
                    span class="metadata-label" { "Date:" }
                    span class="metadata-value" {
                        (format_timestamp(commit.date()))
                    }
                }

                div class="metadata-row" {
                    span class="metadata-label" { "Commit:" }
                    span class="metadata-value commit-hash" {
                        code { (commit.oid()) }
                        button class="copy-hash-btn" title="Copy commit hash" {
                            i class="ph ph-copy" {}
                        }
                    }
                }

                @if let Some(parent_oid) = &diff.parent_oid {
                    div class="metadata-row" {
                        span class="metadata-label" { "Parent:" }
                        span class="metadata-value" {
                            a href=(format!("{}.html", parent_oid)) class="parent-link" {
                                code { (if parent_oid.len() >= 7 { &parent_oid[..7] } else { parent_oid }) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Renders files changed summary section
fn files_changed_section(files: &[crate::git::ChangedFile]) -> Markup {
    html! {
        div class="files-changed-section" {
            div class="files-changed-header" {
                h2 { "Files changed" }
                (file_stats_summary(files))
            }
            (changed_files_list(files))
        }
    }
}

/// Renders copy hash button script
pub fn copy_hash_script() -> Markup {
    html! {
        script {
            (PreEscaped(r#"
document.querySelector('.copy-hash-btn')?.addEventListener('click', async function() {
    const hash = this.parentElement.querySelector('code').textContent;
    try {
        await navigator.clipboard.writeText(hash);
        const icon = this.querySelector('i');
        icon.className = 'ph ph-check';
        setTimeout(() => {
            icon.className = 'ph ph-copy';
        }, 2000);
    } catch (e) {
        console.error('Copy failed:', e);
    }
});
"#))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::list_commits;
    use std::path::PathBuf;

    #[test]
    fn test_generate_commit_page_real_repo() {
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Get first commit from repo
        let commits = list_commits(&repo_path, None, Some(1)).expect("Should list commits");
        assert!(!commits.is_empty(), "Repository should have commits");

        let commit = &commits[0];

        // Generate page
        let result = generate_commit_page(&repo_path, commit.oid(), "gitkyl", Some("master"));

        assert!(result.is_ok(), "Should generate commit page");
        let html = result.unwrap();

        // Verify HTML contains expected elements
        assert!(html.contains("<!DOCTYPE html>"), "Should have DOCTYPE");
        assert!(
            html.contains(&commit.short_oid()),
            "Should contain commit hash"
        );
        assert!(html.contains(commit.author()), "Should contain author");
    }

    #[test]
    fn test_generate_commit_page_invalid_commit() {
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let invalid_oid = "0000000000000000000000000000000000000000";

        let result = generate_commit_page(&repo_path, invalid_oid, "gitkyl", None);

        assert!(result.is_err(), "Should fail for invalid commit");
    }
}
