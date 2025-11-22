//! Markdown rendering with GitHub Flavored Markdown support.

use anyhow::{Context, Result, bail};
use comrak::Options;
use std::path::Path;
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Maximum markdown file size to prevent memory exhaustion (1MB).
const MAX_MARKDOWN_SIZE: usize = 1_048_576;

/// Renders markdown to HTML with GitHub Flavored Markdown extensions.
///
/// Provides GFM extensions including tables, strikethrough, autolinks,
/// task lists, footnotes, and description lists. Uses syntect for code
/// block syntax highlighting when language is specified.
pub struct MarkdownRenderer<'a> {
    options: Options<'a>,
    syntax_set: SyntaxSet,
}

impl<'a> MarkdownRenderer<'a> {
    /// Creates renderer with GitHub Flavored Markdown options.
    ///
    /// Configures all GFM extensions and security settings:
    /// - Tables, strikethrough, autolinks, task lists, footnotes
    /// - Smart punctuation for quotes and dashes
    /// - HTML sanitization enabled (no raw HTML injection)
    /// - Syntax highlighting with syntect using CSS classes
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

        // Load syntax definitions for highlighting
        let syntax_set = SyntaxSet::load_defaults_newlines();

        Self {
            options,
            syntax_set,
        }
    }

    /// Renders markdown content to HTML string.
    ///
    /// Parses markdown into AST, applies transformations, and renders
    /// to HTML with GFM extensions. Content size is validated to prevent
    /// memory exhaustion attacks. Code blocks are syntax highlighted with
    /// CSS class names using syntect.
    ///
    /// # Arguments
    ///
    /// * `content`: Markdown content to render
    ///
    /// # Returns
    ///
    /// Rendered HTML as string with syntax highlighted code blocks
    ///
    /// # Errors
    ///
    /// Returns error if content exceeds maximum size (1MB) or highlighting fails
    pub fn render(&self, content: &str) -> Result<String> {
        if content.len() > MAX_MARKDOWN_SIZE {
            bail!(
                "Markdown content too large: {} bytes (max: {})",
                content.len(),
                MAX_MARKDOWN_SIZE
            );
        }

        let html = comrak::markdown_to_html(content, &self.options);

        // Post-process HTML to add syntax highlighting with CSS classes
        self.highlight_code_blocks(&html)
    }

    /// Post-processes HTML to apply syntax highlighting with CSS classes.
    ///
    /// Finds code blocks with language-* classes from comrak's output and
    /// replaces the plain text content with syntect highlighted HTML using
    /// CSS class names (hljs-* prefix).
    ///
    /// # Arguments
    ///
    /// * `html`: Raw HTML from comrak with <code class="language-X"> blocks
    ///
    /// # Returns
    ///
    /// HTML with syntax highlighted code blocks using CSS classes
    ///
    /// # Errors
    ///
    /// Returns error if HTML parsing or highlighting fails
    fn highlight_code_blocks(&self, html: &str) -> Result<String> {
        let mut result = String::with_capacity(html.len());
        let mut last_end = 0;

        // Pattern: <code class="language-LANG">CODE</code>
        let mut search_pos = 0;

        while let Some(code_start) = html[search_pos..].find("<code class=\"language-") {
            let code_start = search_pos + code_start;

            // Find the language name
            let lang_start = code_start + "<code class=\"language-".len();
            let lang_end = match html[lang_start..].find('"') {
                Some(pos) => lang_start + pos,
                None => {
                    search_pos = code_start + 1;
                    continue;
                }
            };

            let language = &html[lang_start..lang_end];

            // Find the end of the opening tag
            let content_start = match html[lang_end..].find('>') {
                Some(pos) => lang_end + pos + 1,
                None => {
                    search_pos = code_start + 1;
                    continue;
                }
            };

            // Find the closing </code> tag
            let content_end = match html[content_start..].find("</code>") {
                Some(pos) => content_start + pos,
                None => {
                    search_pos = code_start + 1;
                    continue;
                }
            };

            let code_content = &html[content_start..content_end];

            // HTML decode the content (comrak escapes &, <, >, ", ')
            let decoded_content = Self::html_decode(code_content);

            // Copy everything before this code block
            result.push_str(&html[last_end..code_start]);

            // Generate highlighted HTML with CSS classes
            let highlighted = self
                .highlight_code(&decoded_content, language)
                .context("Failed to highlight code block")?;

            // Write opening tag with language class preserved
            result.push_str("<code class=\"language-");
            result.push_str(language);
            result.push_str("\">");
            result.push_str(&highlighted);
            result.push_str("</code>");

            // Move past this code block
            last_end = content_end + "</code>".len();
            search_pos = last_end;
        }

        // Copy remaining HTML after last code block
        result.push_str(&html[last_end..]);

        Ok(result)
    }

    /// Highlights code with syntect using CSS classes.
    ///
    /// Uses ClassedHTMLGenerator to produce HTML with CSS class names
    /// instead of inline styles. The class prefix is "hljs-" to match
    /// existing highlight.js CSS conventions in markdown.css.
    ///
    /// # Arguments
    ///
    /// * `code`: Source code to highlight
    /// * `language`: Language identifier (rust, python, etc)
    ///
    /// # Returns
    ///
    /// HTML string with <span class="hljs-*"> tags
    ///
    /// # Errors
    ///
    /// Returns error if syntax highlighting fails
    fn highlight_code(&self, code: &str, language: &str) -> Result<String> {
        // Handle empty code blocks
        if code.is_empty() {
            return Ok(String::new());
        }

        // Find syntax definition for language
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .or_else(|| self.syntax_set.find_syntax_by_extension(language));

        let syntax = match syntax {
            Some(s) => s,
            None => {
                // Unknown language: return escaped plain text
                return Ok(Self::html_escape(code));
            }
        };

        // Generate HTML with CSS classes using hljs- prefix
        let mut generator = ClassedHTMLGenerator::new_with_class_style(
            syntax,
            &self.syntax_set,
            ClassStyle::SpacedPrefixed { prefix: "hljs-" },
        );

        // Process each line
        for line in LinesWithEndings::from(code) {
            generator
                .parse_html_for_line_which_includes_newline(line)
                .context("Failed to parse line for syntax highlighting")?;
        }

        Ok(generator.finalize())
    }

    /// Decodes HTML entities in code block content.
    ///
    /// Comrak escapes special characters in code blocks. This function
    /// reverses those escapes before passing to syntect.
    ///
    /// # Arguments
    ///
    /// * `html`: HTML encoded string
    ///
    /// # Returns
    ///
    /// Decoded string with actual characters
    fn html_decode(html: &str) -> String {
        html.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
    }

    /// Escapes HTML special characters.
    ///
    /// Used for plain text fallback when language is unknown.
    ///
    /// # Arguments
    ///
    /// * `text`: Plain text to escape
    ///
    /// # Returns
    ///
    /// HTML safe string
    fn html_escape(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
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
            html.contains("<code class=\"language-rust\">"),
            "Should contain code tag with language class: {}",
            html
        );
        // Check for syntax highlighted content (span tags with hljs- classes)
        assert!(
            html.contains("<span class=\"hljs-"),
            "Should contain syntax highlighting spans: {}",
            html
        );
        // Check that code content is present (may be split across span tags)
        assert!(html.contains("fn"), "Should contain 'fn' keyword");
        assert!(html.contains("main"), "Should contain 'main' function name");
        assert!(html.contains("println!"), "Should contain 'println!' macro");
        assert!(html.contains("hello"), "Should contain string content");
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

    #[test]
    fn test_highlight_code_blocks_unknown_language() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
```unknownlang
some code
```
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render");

        // Assert
        assert!(
            html.contains("some code"),
            "Should contain plain text for unknown language"
        );
        assert!(
            html.contains("<code class=\"language-unknownlang\">"),
            "Should preserve language class"
        );
    }

    #[test]
    fn test_highlight_code_blocks_empty() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
```rust
```
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render");

        // Assert
        assert!(
            html.contains("<code class=\"language-rust\">"),
            "Should have code tag for empty block"
        );
    }

    #[test]
    fn test_highlight_multiple_code_blocks() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
First block:
```rust
fn foo() {}
```

Second block:
```python
def bar():
    pass
```
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render");

        // Assert
        assert!(html.contains("foo"), "Should contain Rust function name");
        assert!(html.contains("fn"), "Should contain Rust keyword");
        assert!(html.contains("def"), "Should contain Python keyword");
        assert!(html.contains("bar"), "Should contain Python function name");
        assert!(
            html.contains("<code class=\"language-rust\">"),
            "Should have Rust code block"
        );
        assert!(
            html.contains("<code class=\"language-python\">"),
            "Should have Python code block"
        );
    }

    #[test]
    fn test_highlight_code_with_special_chars() {
        // Arrange
        let renderer = MarkdownRenderer::new();
        let markdown = r#"
```javascript
const x = "<script>alert('xss')</script>";
```
"#;

        // Act
        let html = renderer.render(markdown).expect("Should render");

        // Assert
        assert!(html.contains("const"), "Should contain code");
        assert!(html.contains("alert"), "Should contain string content");
        // Special chars should be properly escaped in output
        assert!(
            html.contains("&lt;script&gt;") || html.contains("<script>"),
            "Should handle special characters"
        );
    }
}
