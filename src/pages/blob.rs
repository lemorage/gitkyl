//! Blob page generation for file content viewing

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use maud::{Markup, PreEscaped, html};
use std::path::Path;

use crate::components::blame::blame_table;
use crate::components::code::{code_table, copy_button_script};
use crate::components::icons::is_markdown;
use crate::components::layout::page_wrapper;
use crate::components::nav::{breadcrumb, extract_breadcrumb_components};
use crate::components::view_toggle::{ViewMode, ViewTab, view_toggle};
use crate::filetype::{FileType, ImageFormat, detect_file_type};
use crate::git::{BlameLine, read_blob};
use crate::highlight::Highlighter;
use crate::markdown::MarkdownRenderer;
use crate::util::{calculate_depth, format_file_size};

/// File metadata for display in blob header
struct FileMetadata {
    line_count: usize,
    sloc: usize,
    file_size: usize,
}

impl FileMetadata {
    /// Creates metadata from file content
    fn from_content(content: &str, raw_size: usize) -> Self {
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len().max(1);
        let sloc = lines.iter().filter(|l| !l.trim().is_empty()).count();
        Self {
            line_count,
            sloc,
            file_size: raw_size,
        }
    }

    /// Formats metadata for display (e.g., "270 lines (214 loc) · 6.8 KB")
    fn display(&self) -> String {
        format!(
            "{} lines ({} loc) · {}",
            self.line_count,
            self.sloc,
            format_file_size(self.file_size)
        )
    }
}

/// Blob content variants for different view modes
enum BlobContent<'a> {
    /// Syntax highlighted source code
    Code {
        lines: &'a [String],
        metadata: &'a FileMetadata,
    },
    /// Rendered markdown HTML
    Preview { html: &'a str },
    /// Blame annotations with syntax highlighted code
    Blame {
        blame_lines: &'a [BlameLine],
        highlighted: &'a [String],
        metadata: &'a FileMetadata,
    },
}

impl BlobContent<'_> {
    /// Returns the view mode for this content
    fn view_mode(&self) -> ViewMode {
        match self {
            BlobContent::Code { .. } => ViewMode::Code,
            BlobContent::Preview { .. } => ViewMode::Preview,
            BlobContent::Blame { .. } => ViewMode::Blame,
        }
    }
}

/// Builds view toggle tabs for blob pages
fn build_view_tabs(file_path: &Path, active: ViewMode) -> Vec<ViewTab> {
    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if is_markdown(file_path) {
        let preview = format!("{}.html", file_name);
        let code = format!("{}.source.html", file_name);
        let blame = format!("{}.blame.html", file_name);

        match active {
            ViewMode::Preview => vec![
                ViewTab::new(ViewMode::Preview, None),
                ViewTab::new(ViewMode::Code, Some(code)),
                ViewTab::new(ViewMode::Blame, Some(blame)),
            ],
            ViewMode::Code => vec![
                ViewTab::new(ViewMode::Preview, Some(preview)),
                ViewTab::new(ViewMode::Code, None),
                ViewTab::new(ViewMode::Blame, Some(blame)),
            ],
            ViewMode::Blame => vec![
                ViewTab::new(ViewMode::Preview, Some(preview)),
                ViewTab::new(ViewMode::Code, Some(code)),
                ViewTab::new(ViewMode::Blame, None),
            ],
        }
    } else {
        let code = format!("{}.html", file_name);
        let blame = format!("{}.blame.html", file_name);

        match active {
            ViewMode::Preview => vec![
                ViewTab::new(ViewMode::Code, Some(code)),
                ViewTab::new(ViewMode::Blame, Some(blame)),
            ],
            ViewMode::Code => vec![
                ViewTab::new(ViewMode::Code, None),
                ViewTab::new(ViewMode::Blame, Some(blame)),
            ],
            ViewMode::Blame => vec![
                ViewTab::new(ViewMode::Code, Some(code)),
                ViewTab::new(ViewMode::Blame, None),
            ],
        }
    }
}

/// Generates HTML blob page based on file type
///
/// Detects file type (text, image, or binary) and dispatches to the appropriate
/// renderer. Text files get syntax highlighting, images are displayed inline,
/// and binary files show an informative message. All file types return success
/// to prevent warnings for normal repository content.
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
/// Returns error only if:
/// - Blob cannot be read from repository
/// - Page rendering fails unexpectedly
///
/// Normal binary files and images do not return errors.
///
/// # Examples
///
/// ```no_run
/// use gitkyl::pages::blob::generate;
/// use std::path::Path;
///
/// // Text file with syntax highlighting
/// let html = generate(
///     Path::new("."),
///     "main",
///     Path::new("src/lib.rs"),
///     "my-repo",
///     "Catppuccin-Latte"
/// )?;
///
/// // Image file displays inline
/// let html = generate(
///     Path::new("."),
///     "main",
///     Path::new("logo.png"),
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

    let file_type = detect_file_type(&content_bytes, file_path.as_ref());

    match file_type {
        FileType::Text => generate_text_blob(
            &content_bytes,
            file_path.as_ref(),
            ref_name,
            repo_name,
            theme,
        ),
        FileType::Image(format) => generate_image_blob(
            &content_bytes,
            format,
            file_path.as_ref(),
            ref_name,
            repo_name,
        ),
        FileType::Binary => {
            generate_binary_blob(&content_bytes, file_path.as_ref(), ref_name, repo_name)
        }
    }
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

    Ok(render_blob_page(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        BlobContent::Preview {
            html: &rendered_html,
        },
    ))
}

/// Generates text blob with syntax highlighting.
///
/// Converts blob bytes to UTF-8 string and applies syntect syntax highlighting
/// based on file extension. Returns formatted HTML with line numbers.
///
/// # Errors
///
/// Returns error if content contains invalid UTF-8 or highlighting fails
fn generate_text_blob(
    bytes: &[u8],
    file_path: &Path,
    ref_name: &str,
    repo_name: &str,
    theme: &str,
) -> Result<Markup> {
    let raw_size = bytes.len();
    let content = String::from_utf8(bytes.to_vec())
        .with_context(|| format!("Text file contains invalid UTF-8: {}", file_path.display()))?;

    let metadata = FileMetadata::from_content(&content, raw_size);

    let highlighter = Highlighter::with_theme(theme)
        .or_else(|_| Highlighter::new())
        .context("Failed to create highlighter")?;

    let highlighted_lines = highlighter
        .highlight(&content, file_path)
        .with_context(|| format!("Failed to highlight: {}", file_path.display()))?;

    let path_str = file_path.display().to_string();
    let path_components = extract_breadcrumb_components(&path_str);

    Ok(render_blob_page(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        BlobContent::Code {
            lines: &highlighted_lines,
            metadata: &metadata,
        },
    ))
}

/// Renders image blob page with embedded image display
///
/// Creates HTML page displaying the image with metadata. Image data is
/// embedded as base64 data URL for self-contained static HTML.
fn generate_image_blob(
    bytes: &[u8],
    format: ImageFormat,
    file_path: &Path,
    ref_name: &str,
    repo_name: &str,
) -> Result<Markup> {
    let path_str = file_path.display().to_string();
    let path_components = extract_breadcrumb_components(&path_str);

    Ok(image_blob_page_markup(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        bytes,
        format,
    ))
}

/// Renders binary blob page with informative message
///
/// Creates HTML page indicating file contains binary data. Displays
/// breadcrumb navigation, file icon, and file size.
fn generate_binary_blob(
    bytes: &[u8],
    file_path: &Path,
    ref_name: &str,
    repo_name: &str,
) -> Result<Markup> {
    let path_str = file_path.display().to_string();
    let path_components = extract_breadcrumb_components(&path_str);

    Ok(binary_blob_page_markup(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        bytes.len(),
    ))
}

/// Renders blob page with different view modes
fn render_blob_page(
    file_path: &str,
    breadcrumb_components: &[&str],
    ref_name: &str,
    repo_name: &str,
    content: BlobContent,
) -> Markup {
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

    let file_path_obj = Path::new(file_path);
    let file_name = file_path_obj
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    let is_markdown_file = is_markdown(file_path_obj);
    let view_mode = content.view_mode();
    let (metadata, has_copy_button, title_suffix) = match &content {
        BlobContent::Code { metadata, .. } => {
            let suffix = if is_markdown_file { " (source)" } else { "" };
            (Some(*metadata), true, suffix)
        }
        BlobContent::Preview { .. } => (None, false, ""),
        BlobContent::Blame { metadata, .. } => (Some(*metadata), false, " (blame)"),
    };

    let view_tabs = build_view_tabs(file_path_obj, view_mode);
    let title = format!("{}/{}: {}{}", repo_name, ref_name, file_path, title_suffix);
    let icon_class = if is_markdown_file {
        "ph ph-file-md"
    } else {
        "ph ph-file-code"
    };

    let markdown_css = format!("{}assets/markdown.css", "../".repeat(depth));
    let css_files: Vec<&str> = match &content {
        BlobContent::Preview { .. } => vec![css_path.as_str(), markdown_css.as_str()],
        _ => vec![css_path.as_str()],
    };

    page_wrapper(
        &title,
        &css_files,
        html! {
            (breadcrumb(repo_name, &index_path, &breadcrumb_data, ref_name))
            div class="blob-card" {
                div class="blob-header" {
                    div class="blob-header-left" {
                        i class=(icon_class) {}
                        span class="blob-filename" { (file_name) }
                        @if let Some(meta) = metadata {
                            span class="blob-meta" { (meta.display()) }
                        }
                    }
                    (view_toggle(&view_tabs))
                    @if has_copy_button {
                        button class="action-btn copy-btn" type="button" title="Copy file contents" {
                            i class="ph ph-copy" {}
                        }
                    }
                }
                @match content {
                    BlobContent::Code { lines, .. } => {
                        (code_table(lines))
                    }
                    BlobContent::Preview { html } => {
                        main class="markdown-content latte" {
                            (PreEscaped(html))
                        }
                    }
                    BlobContent::Blame { blame_lines, highlighted, .. } => {
                        (blame_table(blame_lines, highlighted, depth))
                    }
                }
            }
            @if has_copy_button {
                (copy_button_script())
            }
        },
    )
}

/// Generates HTML blob page with syntax highlighted markdown source
///
/// Creates a source view of markdown files with syntax highlighting,
/// complementing the rendered view. Includes a link back to the rendered version.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `file_path`: Path to markdown file within repository tree
/// * `repo_name`: Repository name for breadcrumb navigation
/// * `theme`: Syntax highlighting theme name
///
/// # Returns
///
/// HTML markup with syntax highlighted markdown source
///
/// # Errors
///
/// Returns error if:
/// - Blob cannot be read from repository
/// - File content contains invalid UTF8
/// - Syntax highlighting fails
pub fn generate_markdown_source(
    repo_path: impl AsRef<Path>,
    ref_name: &str,
    file_path: impl AsRef<Path>,
    repo_name: &str,
    theme: &str,
) -> Result<Markup> {
    let path_str = file_path.as_ref().display().to_string();

    let content_bytes = read_blob(&repo_path, Some(ref_name), &file_path)
        .with_context(|| format!("Failed to read blob from repository: {}", path_str))?;

    let raw_size = content_bytes.len();

    let content = String::from_utf8(content_bytes)
        .with_context(|| format!("Blob contains invalid UTF8: {}", path_str))?;

    let metadata = FileMetadata::from_content(&content, raw_size);

    let highlighter = Highlighter::with_theme(theme)
        .or_else(|_| Highlighter::new())
        .context("Failed to create highlighter")?;

    let highlighted_lines = highlighter
        .highlight(&content, file_path.as_ref())
        .with_context(|| format!("Failed to highlight: {}", path_str))?;

    let path_components = extract_breadcrumb_components(&path_str);

    Ok(render_blob_page(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        BlobContent::Code {
            lines: &highlighted_lines,
            metadata: &metadata,
        },
    ))
}

/// Generates HTML blame page with commit attribution per line
///
/// Creates a blame view showing which commit last modified each line of the file.
/// Each line displays the commit hash, author avatar, author name, and relative
/// timestamp alongside syntax highlighted code.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `file_path`: Path to file within repository tree
/// * `repo_name`: Repository name for breadcrumb navigation
/// * `theme`: Syntax highlighting theme name
///
/// # Returns
///
/// HTML markup with blame annotations and highlighted code
///
/// # Errors
///
/// Returns error if:
/// - Blob cannot be read from repository
/// - File content contains invalid UTF8
/// - Blame computation fails
/// - Syntax highlighting fails
pub fn generate_blame(
    repo_path: impl AsRef<Path>,
    ref_name: &str,
    file_path: impl AsRef<Path>,
    repo_name: &str,
    theme: &str,
) -> Result<Markup> {
    let path_str = file_path.as_ref().display().to_string();

    let content_bytes = read_blob(&repo_path, Some(ref_name), &file_path)
        .with_context(|| format!("Failed to read blob from repository: {}", path_str))?;

    let raw_size = content_bytes.len();

    let content = String::from_utf8(content_bytes)
        .with_context(|| format!("Blob contains invalid UTF8: {}", path_str))?;

    let metadata = FileMetadata::from_content(&content, raw_size);

    let highlighter = Highlighter::with_theme(theme)
        .or_else(|_| Highlighter::new())
        .context("Failed to create highlighter")?;

    let highlighted_lines = highlighter
        .highlight(&content, file_path.as_ref())
        .with_context(|| format!("Failed to highlight: {}", path_str))?;

    let blame_result = crate::git::blame_file(&repo_path, Some(ref_name), &file_path)
        .with_context(|| format!("Failed to compute blame: {}", path_str))?;

    let path_components = extract_breadcrumb_components(&path_str);

    Ok(render_blob_page(
        &path_str,
        &path_components,
        ref_name,
        repo_name,
        BlobContent::Blame {
            blame_lines: &blame_result.lines,
            highlighted: &highlighted_lines,
            metadata: &metadata,
        },
    ))
}

/// Renders image blob page with embedded data URL
fn image_blob_page_markup(
    file_path: &str,
    breadcrumb_components: &[&str],
    ref_name: &str,
    repo_name: &str,
    image_bytes: &[u8],
    format: ImageFormat,
) -> Markup {
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

    let data_url = format!(
        "data:{};base64,{}",
        format.mime_type(),
        STANDARD.encode(image_bytes)
    );
    let file_size = format_file_size(image_bytes.len());
    let title = format!("{}/{}: {}", repo_name, ref_name, file_path);

    page_wrapper(
        &title,
        &[&css_path],
        html! {
            (breadcrumb(repo_name, &index_path, &breadcrumb_data, ref_name))
            main class="blob-container image-blob" {
                div class="image-meta" {
                    span class="file-info" {
                        strong { (format.extension().to_uppercase()) }
                        " · "
                        (file_size)
                    }
                }
                div class="image-display" {
                    img src=(data_url) alt=(file_path) loading="lazy" {}
                }
            }
        },
    )
}

/// Renders binary blob page with file information
fn binary_blob_page_markup(
    file_path: &str,
    breadcrumb_components: &[&str],
    ref_name: &str,
    repo_name: &str,
    file_size_bytes: usize,
) -> Markup {
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

    let file_size = format_file_size(file_size_bytes);
    let title = format!("{}/{}: {}", repo_name, ref_name, file_path);

    page_wrapper(
        &title,
        &[&css_path],
        html! {
            (breadcrumb(repo_name, &index_path, &breadcrumb_data, ref_name))
            main class="blob-container binary-blob" {
                div class="binary-message" {
                    div class="binary-icon" {
                        i class="ph ph-file-x" {}
                    }
                    h2 { "Binary file" }
                    p class="binary-info" {
                        "This file contains binary data and cannot be displayed as text."
                    }
                    p class="file-details" {
                        strong { "Size: " }
                        (file_size)
                    }
                }
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

    #[test]
    fn test_generate_image_png() {
        let repo = create_test_repo().unwrap();
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        fs::write(repo.path().join("test.png"), png_header).unwrap();
        git_commit(repo.path()).unwrap();

        let html = generate(
            repo.path(),
            "HEAD",
            Path::new("test.png"),
            "test-repo",
            "base16-ocean.dark",
        )
        .unwrap();

        let html_str = html.into_string();
        assert!(html_str.contains("test-repo"));
        assert!(html_str.contains("test.png"));
        assert!(html_str.contains("image-blob"));
        assert!(html_str.contains("data:image/png;base64,"));
        assert!(html_str.contains("PNG"));
    }

    #[test]
    fn test_generate_binary_file() {
        let repo = create_test_repo().unwrap();
        let binary_data = [0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
        fs::write(repo.path().join("data.bin"), binary_data).unwrap();
        git_commit(repo.path()).unwrap();

        let html = generate(
            repo.path(),
            "HEAD",
            Path::new("data.bin"),
            "test-repo",
            "base16-ocean.dark",
        )
        .unwrap();

        let html_str = html.into_string();
        assert!(html_str.contains("test-repo"));
        assert!(html_str.contains("data.bin"));
        assert!(html_str.contains("binary-blob"));
        assert!(html_str.contains("Binary file"));
        assert!(html_str.contains("binary data"));
        assert!(html_str.contains("7 bytes"));
    }
}
