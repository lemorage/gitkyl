//! Diff display components for commit pages

use crate::git::{ChangeType, ChangedFile, DiffHunk, DiffLine, DiffLineType, FileStats};
use maud::{Markup, html};

/// Renders file statistics summary with additions and deletions count
pub fn file_stats_summary(files: &[ChangedFile]) -> Markup {
    let total_additions: usize = files.iter().map(|f| f.stats.additions).sum();
    let total_deletions: usize = files.iter().map(|f| f.stats.deletions).sum();

    html! {
        div class="diff-stats" {
            span class="diff-stats-count" {
                (files.len()) " file" (if files.len() != 1 { "s" } else { "" }) " changed"
            }
            @if total_additions > 0 {
                span class="diff-stats-additions" {
                    "+" (total_additions)
                }
            }
            @if total_deletions > 0 {
                span class="diff-stats-deletions" {
                    "-" (total_deletions)
                }
            }
        }
    }
}

/// Renders list of changed files with file stats
pub fn changed_files_list(files: &[ChangedFile]) -> Markup {
    html! {
        div class="changed-files" {
            @for file in files {
                div class="changed-file-entry" {
                    a href=(format!("#diff-{}", sanitize_path(&file.path))) class="file-link" {
                        span class="file-path" { (file.path) }
                    }
                    span class="file-stats" {
                        @if file.change_type == ChangeType::Binary {
                            span class="binary-badge" { "Binary" }
                        } @else {
                            @if file.stats.additions > 0 {
                                span class="stat-additions" { "+" (file.stats.additions) }
                            }
                            @if file.stats.deletions > 0 {
                                span class="stat-deletions" { "-" (file.stats.deletions) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Renders full diff view with all changed files and their hunks
pub fn diff_view(files: &[ChangedFile]) -> Markup {
    html! {
        div class="diff-container" {
            @for file in files {
                (file_diff(file))
            }
        }
    }
}

/// Renders single file diff with header and hunks
fn file_diff(file: &ChangedFile) -> Markup {
    html! {
        div class="file-diff" id=(format!("diff-{}", sanitize_path(&file.path))) {
            div class="diff-file-header" {
                span class="diff-file-path" { (file.path) }
                (file_stats(&file.stats, &file.change_type))
            }
            @if file.change_type == ChangeType::Binary {
                div class="binary-diff-notice" {
                    "Binary file changed"
                }
            } @else {
                @for hunk in &file.hunks {
                    (diff_hunk(hunk))
                }
            }
        }
    }
}

/// Renders file statistics badge
fn file_stats(stats: &FileStats, change_type: &ChangeType) -> Markup {
    html! {
        span class="file-stats-badge" {
            @match change_type {
                ChangeType::Added => {
                    span class="stat-additions" { "+" (stats.additions) }
                }
                ChangeType::Deleted => {
                    span class="stat-deletions" { "-" (stats.deletions) }
                }
                ChangeType::Modified => {
                    @if stats.additions > 0 {
                        span class="stat-additions" { "+" (stats.additions) }
                    }
                    @if stats.deletions > 0 {
                        span class="stat-deletions" { "-" (stats.deletions) }
                    }
                }
                ChangeType::Binary => {
                    span class="binary-badge" { "Binary" }
                }
            }
        }
    }
}

/// Renders single diff hunk with header and lines
fn diff_hunk(hunk: &DiffHunk) -> Markup {
    html! {
        div class="diff-hunk" {
            div class="hunk-header" {
                (hunk.header)
            }
            table class="hunk-content" {
                tbody {
                    @for line in &hunk.lines {
                        (diff_line(line))
                    }
                }
            }
        }
    }
}

/// Renders single diff line with line numbers and content
fn diff_line(line: &DiffLine) -> Markup {
    let (line_class, prefix) = match line.line_type {
        DiffLineType::Addition => ("diff-line-add", "+"),
        DiffLineType::Deletion => ("diff-line-del", "-"),
        DiffLineType::Context => ("diff-line-ctx", " "),
    };

    html! {
        tr class=(line_class) {
            @if let Some(old_num) = line.old_line_num {
                td class="line-num old-line-num" { (old_num) }
            } @else {
                td class="line-num old-line-num empty" {}
            }
            @if let Some(new_num) = line.new_line_num {
                td class="line-num new-line-num" { (new_num) }
            } @else {
                td class="line-num new-line-num empty" {}
            }
            td class="line-prefix" { (prefix) }
            td class="line-content" { (line.content) }
        }
    }
}

/// Sanitizes file path for use in HTML IDs
fn sanitize_path(path: &str) -> String {
    path.replace(['/', '.', ' '], "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path() {
        assert_eq!(sanitize_path("src/main.rs"), "src-main-rs");
        assert_eq!(sanitize_path("path/to/file.txt"), "path-to-file-txt");
        assert_eq!(sanitize_path("test file.md"), "test-file-md");
    }

    #[test]
    fn test_file_stats_summary_single_file() {
        let files = vec![ChangedFile {
            path: "test.rs".to_string(),
            change_type: ChangeType::Modified,
            stats: FileStats {
                additions: 5,
                deletions: 3,
            },
            hunks: vec![],
        }];

        let html = file_stats_summary(&files).into_string();
        assert!(html.contains("1 file changed"));
        assert!(html.contains("+5"));
        assert!(html.contains("-3"));
    }

    #[test]
    fn test_file_stats_summary_multiple_files() {
        let files = vec![
            ChangedFile {
                path: "test1.rs".to_string(),
                change_type: ChangeType::Modified,
                stats: FileStats {
                    additions: 5,
                    deletions: 3,
                },
                hunks: vec![],
            },
            ChangedFile {
                path: "test2.rs".to_string(),
                change_type: ChangeType::Added,
                stats: FileStats {
                    additions: 10,
                    deletions: 0,
                },
                hunks: vec![],
            },
        ];

        let html = file_stats_summary(&files).into_string();
        assert!(html.contains("2 files changed"));
        assert!(html.contains("+15"));
        assert!(html.contains("-3"));
    }

    #[test]
    fn test_binary_file_display() {
        let file = ChangedFile {
            path: "image.png".to_string(),
            change_type: ChangeType::Binary,
            stats: FileStats {
                additions: 0,
                deletions: 0,
            },
            hunks: vec![],
        };

        let html = file_diff(&file).into_string();
        assert!(html.contains("image.png"));
        assert!(html.contains("Binary file changed"));
    }
}
