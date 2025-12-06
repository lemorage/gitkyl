//! Blob page generation for file content viewing

use anyhow::{Context, Result};
use maud::{Markup, PreEscaped, html};
use std::path::Path;

use crate::components::layout::page_wrapper;
use crate::components::nav::{breadcrumb, extract_breadcrumb_components};
use crate::git::read_blob;
use crate::highlight::Highlighter;
use crate::markdown::MarkdownRenderer;
use crate::path::calculate_depth;

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
    let depth = calculate_depth(ref_name, file_path);
    let index_path = "../".repeat(depth) + "index.html";
    let css_path = format!("{}assets/blob.css", "../".repeat(depth));

    let breadcrumb_data: Vec<(&str, Option<String>)> = breadcrumb_components
        .iter()
        .enumerate()
        .map(|(idx, &component)| {
            if idx == breadcrumb_components.len() - 1 {
                (component, None)
            } else {
                let partial_path = breadcrumb_components[..=idx].join("/");
                let link = format!(
                    "{}tree/{}/{}.html",
                    "../".repeat(depth),
                    ref_name,
                    partial_path
                );
                (component, Some(link))
            }
        })
        .collect();

    page_wrapper(
        file_path,
        &[&css_path],
        html! {
            (breadcrumb(repo_name, &index_path, &breadcrumb_data, ref_name))
            main class="blob-container" {
                div class="line-numbers" {
                    @for line_num in 1..=line_count {
                        a href=(format!("#L{}", line_num)) class="line-number" { (line_num) }
                    }
                }
                pre class="code-content latte" {
                    code {
                        @for (idx, line) in highlighted_lines.iter().enumerate() {
                            span class="code-line" id=(format!("L{}", idx + 1)) {
                                (PreEscaped(line))
                            }
                        }
                    }
                }
            }
        },
    )
}

/// Renders markdown blob page HTML structure
fn markdown_blob_page_markup(
    file_path: &str,
    breadcrumb_components: &[&str],
    ref_name: &str,
    repo_name: &str,
    rendered_html: &str,
) -> Markup {
    let depth = calculate_depth(ref_name, file_path);
    let index_path = "../".repeat(depth) + "index.html";
    let css_path = format!("{}assets/blob.css", "../".repeat(depth));
    let markdown_css_path = format!("{}assets/markdown.css", "../".repeat(depth));

    let breadcrumb_data: Vec<(&str, Option<String>)> = breadcrumb_components
        .iter()
        .enumerate()
        .map(|(idx, &component)| {
            if idx == breadcrumb_components.len() - 1 {
                (component, None)
            } else {
                let partial_path = breadcrumb_components[..=idx].join("/");
                let link = format!(
                    "{}tree/{}/{}.html",
                    "../".repeat(depth),
                    ref_name,
                    partial_path
                );
                (component, Some(link))
            }
        })
        .collect();

    page_wrapper(
        file_path,
        &[&css_path, &markdown_css_path],
        html! {
            (breadcrumb(repo_name, &index_path, &breadcrumb_data, ref_name))
            main class="markdown-content latte" {
                (PreEscaped(rendered_html))
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_repo() -> anyhow::Result<TempDir> {
        let dir = TempDir::new()?;
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()?;
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()?;
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()?;
        Ok(dir)
    }

    fn git_commit(path: &Path) -> anyhow::Result<()> {
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "test"])
            .current_dir(path)
            .output()?;
        Ok(())
    }

    #[test]
    fn test_generate_rust_file() {
        let repo = create_test_repo().unwrap();
        fs::write(repo.path().join("test.rs"), "fn main() {}").unwrap();
        git_commit(repo.path()).unwrap();

        let html = generate(
            repo.path(),
            "HEAD",
            Path::new("test.rs"),
            "test-repo",
            "base16-ocean.dark",
        )
        .unwrap();

        let html_str = html.into_string();
        assert!(html_str.contains("test-repo"));
        assert!(html_str.contains("test.rs"));
    }

    #[test]
    fn test_generate_markdown() {
        let repo = create_test_repo().unwrap();
        fs::write(repo.path().join("README.md"), "# Test\nContent").unwrap();
        git_commit(repo.path()).unwrap();

        let html =
            generate_markdown(repo.path(), "HEAD", Path::new("README.md"), "test-repo").unwrap();

        let html_str = html.into_string();
        assert!(html_str.contains("test-repo"));
        assert!(html_str.contains("README.md"));
        assert!(html_str.contains("<h1"));
    }
}
