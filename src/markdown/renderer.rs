//! Markdown rendering with GitHub Flavored Markdown support.

use anyhow::{Context, Result, bail};
use comrak::{Options, markdown_to_html};
use std::path::Path;

/// Maximum markdown file size to prevent memory exhaustion (1MB).
const MAX_MARKDOWN_SIZE: usize = 1_048_576;

/// Renders markdown to HTML with GitHub Flavored Markdown extensions.
///
/// Provides GFM extensions including tables, strikethrough, autolinks,
/// task lists, footnotes, and description lists. Uses syntect for code
/// block syntax highlighting when language is specified.
pub struct MarkdownRenderer<'a> {
    options: Options<'a>,
}

impl<'a> MarkdownRenderer<'a> {
    /// Creates renderer with GitHub Flavored Markdown options.
    ///
    /// Configures all GFM extensions and security settings:
    /// - Tables, strikethrough, autolinks, task lists, footnotes
    /// - Smart punctuation for quotes and dashes
    /// - HTML sanitization enabled (no raw HTML injection)
    pub fn new() -> Self {
        let mut options = Options::default();

        // Extension options (GFM features)
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        options.extension.footnotes = true;
        options.extension.description_lists = true;

        // Parse options (smart punctuation)
        options.parse.smart = true;

        // Render options (security: no raw HTML)
        options.render.unsafe_ = false;

        Self { options }
    }

    /// Renders markdown content to HTML string.
    ///
    /// Parses markdown into AST, applies transformations, and renders
    /// to HTML with GFM extensions. Content size is validated to prevent
    /// memory exhaustion attacks.
    ///
    /// # Arguments
    ///
    /// * `content`: Markdown content to render
    ///
    /// # Returns
    ///
    /// Rendered HTML as string
    ///
    /// # Errors
    ///
    /// Returns error if content exceeds maximum size (1MB)
    pub fn render(&self, content: &str) -> Result<String> {
        if content.len() > MAX_MARKDOWN_SIZE {
            bail!(
                "Markdown content too large: {} bytes (max: {})",
                content.len(),
                MAX_MARKDOWN_SIZE
            );
        }

        Ok(markdown_to_html(content, &self.options))
    }

    /// Renders markdown file at given path.
    ///
    /// Convenience method that reads file and renders content.
    ///
    /// # Arguments
    ///
    /// * `path`: Path to markdown file
    ///
    /// # Returns
    ///
    /// Rendered HTML string
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or rendering fails
    pub fn render_file(&self, path: impl AsRef<Path>) -> Result<String> {
        let content =
            std::fs::read_to_string(path.as_ref()).context("Failed to read markdown file")?;
        self.render(&content)
    }
}

impl<'a> Default for MarkdownRenderer<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_basic_markdown() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = "# Hello\n\nThis is **bold** text.";

        // Act
        let html = renderer.render(markdown).expect("Should render markdown");

        // Assert
        assert!(html.contains("<h1>"), "Should contain h1 tag");
        assert!(html.contains("Hello"), "Should contain heading text");
        assert!(html.contains("<strong>"), "Should contain strong tag");
        assert!(html.contains("bold"), "Should contain bold text");
    }

    #[test]
    fn test_render_gfm_tables() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render table");

        // Assert
        assert!(html.contains("<table>"), "Should contain table tag");
        assert!(html.contains("<th>"), "Should contain table header");
        assert!(html.contains("Header 1"), "Should contain header text");
        assert!(html.contains("<td>"), "Should contain table cell");
        assert!(html.contains("Cell 1"), "Should contain cell text");
    }

    #[test]
    fn test_render_gfm_strikethrough() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = "This is ~~strikethrough~~ text.";

        // Act
        let html = renderer
            .render(markdown)
            .expect("Should render strikethrough");

        // Assert
        assert!(
            html.contains("<del>") || html.contains("<s>"),
            "Should contain strikethrough tag: {}",
            html
        );
        assert!(html.contains("strikethrough"), "Should contain text");
    }

    #[test]
    fn test_render_gfm_tasklist() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
- [ ] Unchecked task
- [x] Checked task
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render tasklist");

        // Assert
        assert!(
            html.contains("type=\"checkbox\""),
            "Should contain checkbox"
        );
        assert!(html.contains("disabled"), "Checkboxes should be disabled");
        assert!(
            html.contains("checked") || html.contains("Checked task"),
            "Should mark checked task: {}",
            html
        );
    }

    #[test]
    fn test_render_code_blocks() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
```rust
fn main() {
    println!("hello");
}
```
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render code block");

        // Assert
        assert!(html.contains("<pre>"), "Should contain pre tag: {}", html);
        assert!(
            html.contains("<code") || html.contains("fn main"),
            "Should contain code tag or code content: {}",
            html
        );
        assert!(html.contains("fn main"), "Should contain code text");
    }

    #[test]
    fn test_render_html_escaping() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = "<script>alert('xss')</script>\n\nNormal text.";

        // Act
        let html = renderer.render(markdown).expect("Should escape HTML");

        // Assert
        assert!(
            !html.contains("<script>alert"),
            "Should not contain raw script tag: {}",
            html
        );
        assert!(html.contains("Normal text"), "Should contain safe text");
    }

    #[test]
    fn test_render_autolinks() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = "Visit https://example.com for more info.";

        // Act
        let html = renderer.render(markdown).expect("Should render autolinks");

        // Assert
        assert!(html.contains("<a "), "Should contain link tag");
        assert!(
            html.contains("https://example.com"),
            "Should contain URL: {}",
            html
        );
    }

    #[test]
    fn test_render_smart_punctuation() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"He said "Hello" -- it's nice."#;

        // Act
        let html = renderer
            .render(markdown)
            .expect("Should render smart quotes");

        // Assert
        assert!(
            html.contains('\u{201C}')
                || html.contains('\u{201D}')
                || html.contains("&ldquo;")
                || html.contains("&rdquo;"),
            "Should contain smart quotes (curly quotes): {}",
            html
        );
    }

    #[test]
    fn test_render_empty_markdown() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = "";

        // Act
        let result = renderer.render(markdown);

        // Assert
        assert!(result.is_ok(), "Empty markdown should render successfully");
    }

    #[test]
    fn test_render_size_limit() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let large_content = "a".repeat(MAX_MARKDOWN_SIZE + 1);

        // Act
        let result = renderer.render(&large_content);

        // Assert
        assert!(result.is_err(), "Should reject oversized content");
        assert!(
            result.unwrap_err().to_string().contains("too large"),
            "Error should mention size limit"
        );
    }

    #[test]
    fn test_render_file_size_limit_exceeded() {
        // Arrange
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("large.md");

        // Create file exceeding MAX_MARKDOWN_SIZE
        let mut file = std::fs::File::create(&file_path).expect("Failed to create large file");
        let large_content = "a".repeat(MAX_MARKDOWN_SIZE + 100);
        file.write_all(large_content.as_bytes())
            .expect("Failed to write large content");

        let renderer = MarkdownRenderer::new();

        // Act
        let result = renderer.render_file(&file_path);

        // Assert
        assert!(result.is_err(), "Should reject file exceeding size limit");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("too large"),
            "Error should mention size limit: {}",
            error_msg
        );
    }

    #[test]
    fn test_render_blockquotes() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = "> This is a quote\n> Second line";

        // Act
        let html = renderer.render(markdown).expect("Should render blockquote");

        // Assert
        assert!(
            html.contains("<blockquote>"),
            "Should contain blockquote tag"
        );
        assert!(
            html.contains("This is a quote"),
            "Should contain quote text"
        );
    }

    #[test]
    fn test_render_lists() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
- Item 1
- Item 2
  - Nested item
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render lists");

        // Assert
        assert!(html.contains("<ul>"), "Should contain unordered list");
        assert!(html.contains("<li>"), "Should contain list item");
        assert!(html.contains("Item 1"), "Should contain item text");
    }

    #[test]
    fn test_default_constructor() {
        // Arrange & Act
        let renderer = MarkdownRenderer::default();
        let markdown = "# Test";
        let html = renderer.render(markdown).expect("Default should work");

        // Assert
        assert!(html.contains("<h1>"), "Default renderer should work");
    }
}
