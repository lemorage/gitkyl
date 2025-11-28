//! Blob page generation for file content viewing

use anyhow::{Context, Result};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::path::Path;

use crate::components::nav::extract_breadcrumb_components;
use crate::git::read_blob;
use crate::highlight::Highlighter;
use crate::markdown::MarkdownRenderer;

/// Generates HTML blob page with syntax highlighting
///
/// Reads blob content from the repository at the specified reference and path,
/// applies syntect syntax highlighting, and renders as HTML with line numbers.
/// The output follows GitHub's visual design patterns.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `file_path`: File path within repository tree
/// * `repo_name`: Repository name for breadcrumb navigation
/// * `theme`: Syntax highlighting theme name
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
/// - Theme cannot be loaded
///
/// # Examples
///
/// ```no_run
/// use gitkyl::pages::blob::generate;
/// use std::path::Path;
///
/// let html = generate(
///     Path::new("."),
///     "main",
///     Path::new("src/lib.rs"),
///     "my-repo",
///     "Catppuccin-Latte"
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate(
    repo_path: impl AsRef<Path>,
    ref_name: &str,
    file_path: impl AsRef<Path>,
    repo_name: &str,
    theme: &str,
) -> Result<Markup> {
    let path_str = file_path.as_ref().display().to_string();

    let content_bytes = read_blob(&repo_path, Some(ref_name), &file_path)
        .with_context(|| format!("Failed to read blob from repository: {}", path_str))?;

    let content = String::from_utf8(content_bytes)
        .with_context(|| format!("Blob contains invalid UTF8: {}", path_str))?;

    let highlighter = Highlighter::with_theme(theme)
        .with_context(|| format!("Failed to create syntax highlighter with theme: {}", theme))?;

    let highlighted_lines = highlighter
        .highlight(&content, file_path.as_ref())
        .with_context(|| format!("Failed to apply syntax highlighting: {}", path_str))?;

    let path_components = extract_breadcrumb_components(&path_str);

    Ok(blob_page_markup(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        &highlighted_lines,
    ))
}

/// Generates HTML blob page with rendered markdown
///
/// Reads markdown content from repository and renders using GitHub Flavored
/// Markdown with tables, strikethrough, autolinks, and task lists. Link
/// resolution is not yet implemented (links remain as is).
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `file_path`: Path to markdown file within repository tree
/// * `repo_name`: Repository name for breadcrumb navigation
///
/// # Returns
///
/// HTML markup with rendered markdown content
///
/// # Errors
///
/// Returns error if:
/// - Blob cannot be read from repository
/// - File content contains invalid UTF8
/// - Markdown rendering fails
///
/// # Examples
///
/// ```no_run
/// use gitkyl::pages::blob::generate_markdown;
/// use std::path::Path;
///
/// let html = generate_markdown(
///     Path::new("."),
///     "main",
///     Path::new("README.md"),
///     "my-repo"
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn generate_markdown(
    repo_path: impl AsRef<Path>,
    ref_name: &str,
    file_path: impl AsRef<Path>,
    repo_name: &str,
) -> Result<Markup> {
    let path_str = file_path.as_ref().display().to_string();

    let content_bytes = read_blob(&repo_path, Some(ref_name), &file_path)
        .with_context(|| format!("Failed to read blob from repository: {}", path_str))?;

    let content = String::from_utf8(content_bytes)
        .with_context(|| format!("Blob contains invalid UTF8: {}", path_str))?;

    let renderer = MarkdownRenderer::new();
    let rendered_html = renderer
        .render(&content)
        .with_context(|| format!("Failed to render markdown: {}", path_str))?;

    let path_components = extract_breadcrumb_components(&path_str);

    Ok(markdown_blob_page_markup(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        &rendered_html,
    ))
}

/// Renders blob page HTML structure
fn blob_page_markup(
    file_path: &str,
    breadcrumb_components: &[&str],
    ref_name: &str,
    repo_name: &str,
    highlighted_lines: &[String],
) -> Markup {
    let line_count = highlighted_lines.len().max(1);

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
                            a href=(index_path) class="breadcrumb-link" { (repo_name) }
                            @for (idx, component) in breadcrumb_components.iter().enumerate() {
                                span class="breadcrumb-separator" { "/" }
                                @if idx == breadcrumb_components.len() - 1 {
                                    span class="breadcrumb-current" { (*component) }
                                } @else {
                                    @let partial_path = breadcrumb_components[..=idx].join("/");
                                    @let tree_href = format!("{}tree/{}/{}.html", "../".repeat(depth), ref_name, partial_path);
                                    a href=(tree_href) class="breadcrumb-link" { (*component) }
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
                                    @for line in highlighted_lines {
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

/// Renders markdown blob page HTML structure
fn markdown_blob_page_markup(
    file_path: &str,
    breadcrumb_components: &[&str],
    ref_name: &str,
    repo_name: &str,
    rendered_html: &str,
) -> Markup {
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
                link rel="stylesheet" href=(format!("{}assets/markdown.css", "../".repeat(depth)));
            }
            body {
                div class="container" {
                    header {
                        div class="breadcrumb" {
                            a href=(index_path) class="breadcrumb-link" { (repo_name) }
                            @for (idx, component) in breadcrumb_components.iter().enumerate() {
                                span class="breadcrumb-separator" { "/" }
                                @if idx == breadcrumb_components.len() - 1 {
                                    span class="breadcrumb-current" { (*component) }
                                } @else {
                                    @let partial_path = breadcrumb_components[..=idx].join("/");
                                    @let tree_href = format!("{}tree/{}/{}.html", "../".repeat(depth), ref_name, partial_path);
                                    a href=(tree_href) class="breadcrumb-link" { (*component) }
                                }
                            }
                        }
                        div class="ref-info" {
                            span class="ref-label" { "ref: " }
                            span class="ref-name" { (ref_name) }
                        }
                    }
                    main class="markdown-content latte" {
                        (PreEscaped(rendered_html))
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
