//! Tag listing and detail page generation

use maud::{Markup, html};

use crate::avatar;
use crate::components::layout::page_wrapper;
use crate::components::nav::breadcrumb;
use crate::git::TagInfo;
use crate::util::format_timestamp;

/// Generates the tags listing page
///
/// Displays all repository tags sorted by date (newest first).
/// Shows tag name, short OID, message (if annotated), and date.
///
/// # Arguments
///
/// * `repo_name`: Repository name for page title and breadcrumb
/// * `tags`: Slice of TagInfo sorted by date
///
/// # Returns
///
/// Complete HTML page as Markup
pub fn generate_list(repo_name: &str, tags: &[TagInfo]) -> Markup {
    let css_path = "../assets/tags.css";
    let index_path = "../index.html";

    page_wrapper(
        &format!("{}: tags", repo_name),
        &[css_path],
        html! {
            (breadcrumb(repo_name, index_path, &[("Tags", None)], "tags"))

            main.repo-card {
                div.repo-controls {
                    div.control-left {
                        h1.page-title {
                            i.ph.ph-tag {}
                            "Tags"
                        }
                    }
                    div.control-right {
                        span.badge { (tags.len()) " tags" }
                    }
                }

                @if tags.is_empty() {
                    div.empty-state {
                        p { "No tags found in this repository." }
                    }
                } @else {
                    div.file-table {
                        @for tag in tags {
                            a.file-row href=(format!("{}.html", tag.name)) {
                                div.cell-name {
                                    i.ph.ph-tag {}
                                    span.name-text { (tag.name) }
                                }
                                div.cell-message {
                                    @if let Some(ref message) = tag.message {
                                        (message.trim().lines().next().unwrap_or(""))
                                    } @else {
                                        span.faint { "No message" }
                                    }
                                }
                                div.cell-meta {
                                    span.oid { (tag.short_oid) }
                                    span.date {
                                        @if let Some(date) = tag.date {
                                            (format_timestamp(date))
                                        } @else {
                                            "-"
                                        }
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

/// Generates a tag detail page
///
/// Shows tag information with commit details and browse link.
///
/// # Arguments
///
/// * `repo_name`: Repository name
/// * `tag`: Tag information
/// * `commit_message`: Full commit message at tag
/// * `commit_author`: Commit author name
/// * `commit_date`: Commit timestamp
///
/// # Returns
///
/// Complete HTML page as Markup
pub fn generate_detail(
    repo_name: &str,
    tag: &TagInfo,
    commit_message: &str,
    commit_author: &str,
    commit_date: i64,
) -> Markup {
    let css_path = "../assets/tags.css";
    let index_path = "../index.html";

    page_wrapper(
        &format!("{}: {}", repo_name, tag.name),
        &[css_path],
        html! {
            (breadcrumb(
                repo_name,
                index_path,
                &[("Tags", Some("index.html".to_string())), (&tag.name, None)],
                &tag.name
            ))

            main.repo-card {
                div.repo-controls {
                    div.control-left {
                        h1.page-title {
                            i.ph.ph-tag {}
                            (tag.name)
                        }
                    }
                    div.control-right {
                        span.badge {
                            i.ph.ph-git-commit {}
                            "Tag"
                        }
                    }
                }

                div.detail-content {
                    div.commit-info {
                        (avatar::render(commit_author, 40))
                        div.commit-details {
                            div.commit-author-line {
                                span.commit-author { (commit_author) }
                                span.commit-date { (format_timestamp(commit_date)) }
                            }
                            p.commit-message-text { (commit_message) }
                        }
                    }

                    div.detail-section {
                        div.detail-grid {
                            span.detail-label { "Commit" }
                            span.detail-value.mono { (tag.target_oid) }

                            @if let Some(ref tagger) = tag.tagger {
                                span.detail-label { "Tagger" }
                                span.detail-value { (tagger) }
                            }

                            @if let Some(date) = tag.date {
                                span.detail-label { "Tagged" }
                                span.detail-value { (format_timestamp(date)) }
                            }
                        }
                    }

                    @if let Some(ref message) = tag.message {
                        @if !message.trim().is_empty() {
                            div.tag-message-section {
                                div.tag-message-label { "Tag Message" }
                                p.tag-message-text { (message.trim()) }
                            }
                        }
                    }
                }
            }

            div.action-bar {
                a.browse-link href=(format!("../tree/{}/index.html", tag.name)) {
                    "Browse files"
                    i.ph.ph-arrow-right {}
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_list_empty() {
        // Arrange
        let tags: Vec<TagInfo> = vec![];

        // Act
        let html = generate_list("test-repo", &tags);

        // Assert
        let html_str = html.into_string();
        assert!(
            html_str.contains("No tags found"),
            "Should show empty state"
        );
        assert!(
            html_str.contains("repo-card"),
            "Should use repo-card container"
        );
    }

    #[test]
    fn test_generate_list_with_tags() {
        // Arrange
        let tags = vec![TagInfo::new(
            "v1.0.0".to_string(),
            "abc123def456".to_string(),
            Some("First release".to_string()),
            Some("Author <author@example.com>".to_string()),
            Some(1234567890),
        )];

        // Act
        let html = generate_list("test-repo", &tags);

        // Assert
        let html_str = html.into_string();
        assert!(html_str.contains("v1.0.0"), "Should contain tag name");
        assert!(html_str.contains("abc12"), "Should contain short OID");
        assert!(html_str.contains("First release"), "Should contain message");
        assert!(
            html_str.contains("file-table"),
            "Should use file-table structure"
        );
        assert!(html_str.contains("ph-tag"), "Should have tag icon");
    }

    #[test]
    fn test_generate_detail() {
        // Arrange
        let tag = TagInfo::new(
            "v2.0.0".to_string(),
            "def456abc123".to_string(),
            Some("Major release".to_string()),
            Some("Tagger <tagger@example.com>".to_string()),
            Some(1234567890),
        );

        // Act
        let html = generate_detail(
            "test-repo",
            &tag,
            "Commit message here",
            "Commit Author",
            1234567890,
        );

        // Assert
        let html_str = html.into_string();
        assert!(html_str.contains("repo-card"), "Should use repo-card");
        assert!(html_str.contains("v2.0.0"), "Should contain tag name");
        assert!(html_str.contains("def456abc123"), "Should contain full OID");
        assert!(
            html_str.contains("Major release"),
            "Should contain tag message"
        );
        assert!(
            html_str.contains("Commit message here"),
            "Should contain commit message"
        );
        assert!(html_str.contains("action-bar"), "Should have action bar");
        assert!(html_str.contains("browse-link"), "Should have browse link");
        assert!(html_str.contains("avatar"), "Should have avatar");
    }
}
