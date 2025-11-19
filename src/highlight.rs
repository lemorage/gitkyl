//! Syntax highlighting with syntect.
//!
//! Uses TextMate grammars and Sublime Text themes for high quality
//! syntax highlighting across languages. Provides inline styled
//! HTML output using professionally designed color schemes.

use anyhow::{Context, Result};
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Syntax highlighting engine with lazy-loaded syntaxes and themes.
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    /// Creates a new highlighter with default syntaxes and theme.
    ///
    /// Loads all default syntaxes (75+ languages) and uses the InspiredGitHub
    /// theme which provides clean, professional highlighting suitable for
    /// light backgrounds.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitkyl::Highlighter;
    ///
    /// let highlighter = Highlighter::new();
    /// ```
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["InspiredGitHub"].clone();

        Self { syntax_set, theme }
    }

    /// Highlights source code with syntax highlighting.
    ///
    /// Detects the programming language from the file extension and applies
    /// appropriate syntax highlighting line by line. If the language is
    /// unsupported or detection fails, returns plain escaped HTML.
    ///
    /// # Arguments
    ///
    /// * `code`: Source code to highlight
    /// * `path`: File path used for language detection via extension
    ///
    /// # Returns
    ///
    /// Vector of HTML strings, one per line, with inline styled spans.
    /// All HTML special characters are properly escaped.
    ///
    /// # Errors
    ///
    /// Returns error if syntect highlighting fails unexpectedly.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitkyl::Highlighter;
    /// use std::path::Path;
    ///
    /// let highlighter = Highlighter::new();
    /// let lines = highlighter.highlight("fn main() {}", Path::new("main.rs"))?;
    /// assert!(!lines.is_empty());
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn highlight(&self, code: &str, path: &Path) -> Result<Vec<String>> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");

        let syntax = self
            .syntax_set
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut result = Vec::new();

        for line in LinesWithEndings::from(code) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .context("Failed to highlight line")?;
            let html = styled_line_to_highlighted_html(&ranges[..], IncludeBackground::No)
                .context("Failed to convert styled line to HTML")?;
            result.push(html);
        }

        Ok(result)
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Highlights source code with syntax highlighting.
///
/// Convenience function that creates a highlighter and highlights the code.
/// For repeated highlighting operations, create a Highlighter instance
/// to reuse loaded syntaxes and themes.
///
/// # Arguments
///
/// * `code`: Source code to highlight
/// * `path`: File path used for language detection
///
/// # Returns
///
/// Vector of HTML strings, one per line, with inline styled syntax highlighting.
///
/// # Errors
///
/// Returns error if syntect highlighting fails.
///
/// # Examples
///
/// ```no_run
/// use gitkyl::highlight;
/// use std::path::Path;
///
/// let lines = highlight("let x = 42;", Path::new("test.rs"))?;
/// assert!(!lines.is_empty());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn highlight(code: &str, path: &Path) -> Result<Vec<String>> {
    let highlighter = Highlighter::new();
    highlighter.highlight(code, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_new() {
        // Arrange & Act
        let highlighter = Highlighter::new();

        // Assert: Verify syntax set has default syntaxes loaded
        assert!(
            highlighter
                .syntax_set
                .find_syntax_by_extension("rs")
                .is_some(),
            "Should have Rust syntax loaded"
        );
        assert!(
            highlighter
                .syntax_set
                .find_syntax_by_extension("py")
                .is_some(),
            "Should have Python syntax loaded"
        );
    }

    #[test]
    fn test_highlight_rust() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "fn main() { println!(\"hello\"); }";
        let path = Path::new("test.rs");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight Rust code");

        // Assert
        assert!(!lines.is_empty(), "Should return lines");
        let html = lines.join("");
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("fn"), "Should contain original code");
        assert!(html.contains("main"), "Should contain original code");
    }

    #[test]
    fn test_highlight_python() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "def hello():\n    print('world')";
        let path = Path::new("test.py");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight Python code");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("def"), "Should contain original code");
        assert!(html.contains("hello"), "Should contain original code");
    }

    #[test]
    fn test_highlight_javascript() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "function greet() { console.log('hi'); }";
        let path = Path::new("test.js");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight JavaScript code");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("function"), "Should contain original code");
    }

    #[test]
    fn test_highlight_typescript() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "const x: number = 42;";
        let path = Path::new("test.ts");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight TypeScript code");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("const"), "Should contain original code");
    }

    #[test]
    fn test_highlight_go() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "package main\n\nfunc main() {}";
        let path = Path::new("main.go");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight Go code");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("package"), "Should contain original code");
    }

    #[test]
    fn test_highlight_c() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "#include <stdio.h>\nint main() { return 0; }";
        let path = Path::new("main.c");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight C code");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("include"), "Should contain original code");
    }

    #[test]
    fn test_highlight_cpp() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "#include <iostream>\nint main() { std::cout << \"hi\"; }";
        let path = Path::new("main.cpp");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight C++ code");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("iostream"), "Should contain original code");
    }

    #[test]
    fn test_highlight_unsupported_fallback() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "This is plain text with no syntax";
        let path = Path::new("README");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should fallback to plain text");
        let html = lines.join("");

        // Assert
        assert!(
            html.contains("This is plain text"),
            "Should contain original text"
        );
    }

    #[test]
    fn test_highlight_html_escaping() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = r#"let s = "<>&\"';"#;
        let path = Path::new("test.rs");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should escape HTML");
        let html = lines.join("");

        // Assert: Syntect handles HTML escaping automatically
        assert!(!html.contains("<>&"), "HTML entities should be escaped");
    }

    #[test]
    fn test_highlight_empty_code() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "";
        let path = Path::new("test.rs");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should handle empty code");

        // Assert: Empty code returns no lines
        assert!(
            lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()),
            "Empty code should produce no/empty lines"
        );
    }

    #[test]
    fn test_highlight_multiline() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        let path = Path::new("test.rs");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight multiline code");
        let html = lines.join("");

        // Assert
        assert!(html.contains("fn"), "Should contain code");
        assert!(html.contains("let"), "Should contain code");
    }

    #[test]
    fn test_highlight_function_convenience() {
        // Arrange
        let code = "fn test() {}";
        let path = Path::new("test.rs");

        // Act
        let lines = highlight(code, path).expect("Convenience function should work");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("fn"), "Should contain original code");
    }

    #[test]
    fn test_highlighter_default() {
        // Arrange & Act
        let highlighter = Highlighter::default();

        // Assert
        assert!(
            highlighter
                .syntax_set
                .find_syntax_by_extension("rs")
                .is_some(),
            "Default should work like new()"
        );
    }

    #[test]
    fn test_highlight_json() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = r#"{"key": "value", "number": 42}"#;
        let path = Path::new("config.json");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight JSON");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("key"), "Should contain original code");
    }

    #[test]
    fn test_highlight_yaml() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "name: test\nversion: 1.0";
        let path = Path::new("config.yml");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight YAML");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("name"), "Should contain original code");
    }

    #[test]
    fn test_highlight_shell() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "#!/bin/bash\necho 'hello'";
        let path = Path::new("script.sh");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight shell script");
        let html = lines.join("");

        // Assert
        assert!(html.contains("style="), "Should contain inline styles");
        assert!(html.contains("echo"), "Should contain original code");
    }

    #[test]
    fn test_highlight_markdown() {
        // Arrange
        let highlighter = Highlighter::new();
        let code = "# Title\n\nSome **bold** text";
        let path = Path::new("README.md");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should highlight Markdown");
        let html = lines.join("");

        // Assert
        assert!(html.contains("Title"), "Should contain original code");
    }
}
