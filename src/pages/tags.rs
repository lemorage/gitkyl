//! Tag listing and detail page generation

use maud::{Markup, html};

use crate::components::layout::page_wrapper;
use crate::components::nav::breadcrumb;
use crate::git::TagInfo;
use crate::util::format_timestamp;

/// Generates the tags listing page
///
/// Displays all repository tags sorted by date (newest first).
/// Shows tag name, short OID, message (if annotated), tagger, and date.
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

            main {
                h1 { "Tags" }
                p.tag-count {
                    (tags.len()) " " (if tags.len() == 1 { "tag" } else { "tags" })
                }

                @if tags.is_empty() {
                    div.empty-state {
                        p { "No tags found in this repository." }
                    }
                } @else {
                    ul.tag-list {
                        @for tag in tags {
                            li.tag-entry {
                                div.tag-header {
                                    a.tag-name href=(format!("{}.html", tag.name)) {
                                        (tag.name)
                                    }
                                    code.tag-commit { (tag.short_oid) }
                                }

                                @if let Some(ref message) = tag.message {
                                    p.tag-message { (message.trim()) }
                                }

                                div.tag-meta {
                                    @if let Some(ref tagger) = tag.tagger {
                                        span.tag-tagger { (tagger) }
                                    }
                                    @if let Some(date) = tag.date {
                                        span.tag-date {
                                            @if tag.tagger.is_some() { " · " }
                                            (format_timestamp(date))
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
/// Shows tag information and the commit it points to.
///
/// # Arguments
///
/// * `repo_name`: Repository name
/// * `tag`: Tag information
/// * `commit_message`: Full commit message at tag
/// * `commit_author`: Commit author
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

            main {
                h1 { "Tag: " (tag.name) }

                section.tag-info-card {
                    h2 { "Tag Information" }

                    div.info-grid {
                        div.info-row {
                            span.info-label { "Commit" }
                            code.info-value { (tag.target_oid) }
                        }

                        @if let Some(ref message) = tag.message {
                            div.info-row {
                                span.info-label { "Message" }
                                p.info-value { (message.trim()) }
                            }
                        }

                        @if let Some(ref tagger) = tag.tagger {
                            div.info-row {
                                span.info-label { "Tagged by" }
                                span.info-value { (tagger) }
                            }
                        }

                        @if let Some(date) = tag.date {
                            div.info-row {
                                span.info-label { "Tagged" }
                                span.info-value { (format_timestamp(date)) }
                            }
                        }
                    }
                }

                section.commit-info-card {
                    h2 { "Commit at Tag" }

                    div.commit-details {
                        p.commit-message-full { (commit_message) }

                        div.commit-meta-info {
                            span.commit-author { (commit_author) }
                            span.commit-date-full { " · " (format_timestamp(commit_date)) }
                        }

                        a.browse-link href=(format!("../tree/{}/index.html", tag.name)) {
                            "Browse files at this tag →"
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
        assert!(html_str.contains("0 tags"), "Should show zero count");
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
        assert!(html_str.contains("abc123d"), "Should contain short OID");
        assert!(html_str.contains("First release"), "Should contain message");
        assert!(html_str.contains("1 tag"), "Should show singular count");
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
        assert!(
            html_str.contains("Tag: v2.0.0"),
            "Should contain tag name in title"
        );
        assert!(html_str.contains("def456abc123"), "Should contain full OID");
        assert!(
            html_str.contains("Major release"),
            "Should contain tag message"
        );
        assert!(
            html_str.contains("Commit message here"),
            "Should contain commit message"
        );
        assert!(
            html_str.contains("Browse files at this tag"),
            "Should have browse link"
        );
    }
}
