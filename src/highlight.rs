//! Syntax highlighting with syntect.
//!
//! Uses TextMate grammars and Sublime Text themes for high quality
//! syntax highlighting across 75+ languages. Provides inline styled
//! HTML output using professionally designed color schemes.
//!
//! Default theme: Catppuccin-Latte (modern warm light theme).

use anyhow::{Context, Result, bail};
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
    /// Creates highlighter with default theme (Catppuccin-Latte).
    ///
    /// Falls back to base16-ocean.light if Catppuccin themes unavailable.
    ///
    /// # Errors
    ///
    /// Returns error if no themes can be loaded.
    pub fn new() -> Result<Self> {
        Self::with_theme("Catppuccin-Latte").or_else(|_| Self::with_theme("base16-ocean.light"))
    }

    /// Creates highlighter with specified theme.
    ///
    /// # Theme Resolution Order
    ///
    /// 1. Themes directory (Catppuccin-Latte, Catppuccin-Mocha)
    /// 2. Built-in syntect themes (InspiredGitHub, base16-ocean.*, Solarized)
    /// 3. External .tmTheme files (if path ends with .tmTheme)
    ///
    /// # Recommended Themes
    ///
    /// - `Catppuccin-Latte` - Modern warm light theme
    /// - `Catppuccin-Mocha` - Modern dark theme
    ///
    /// # Errors
    ///
    /// Returns error if theme is not found or cannot be loaded.
    pub fn with_theme(theme_name: &str) -> Result<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines();

        // Themes directory (priority)
        let theme_path = Path::new("themes").join(format!("{}.tmTheme", theme_name));
        if theme_path.exists() {
            let theme = ThemeSet::get_theme(&theme_path)
                .with_context(|| format!("Failed to load theme: {}", theme_name))?;
            return Ok(Self { syntax_set, theme });
        }

        // Built-in syntect themes (fallback)
        let theme_set = ThemeSet::load_defaults();
        if let Some(theme) = theme_set.themes.get(theme_name) {
            return Ok(Self {
                syntax_set,
                theme: theme.clone(),
            });
        }

        // External .tmTheme file
        if theme_name.ends_with(".tmTheme") {
            let theme = ThemeSet::get_theme(theme_name)
                .with_context(|| format!("Failed to load theme: {}", theme_name))?;
            return Ok(Self { syntax_set, theme });
        }

        bail!(
            "Theme '{}' not found. Available:\n\
            Recommended: Catppuccin-Latte, Catppuccin-Mocha\n\
            Built-in: InspiredGitHub, base16-ocean.light, base16-ocean.dark, \
            Solarized (light), Solarized (dark)\n\
            Or provide path to .tmTheme file.",
            theme_name
        )
    }

    /// Highlights source code with syntax highlighting.
    ///
    /// Detects language from file extension and applies highlighting
    /// line by line. Falls back to plain text for unsupported languages.
    ///
    /// # Arguments
    ///
    /// * `code`: Source code to highlight
    /// * `path`: File path for language detection
    ///
    /// # Returns
    ///
    /// Vector of HTML strings (one per line) with inline styles.
    ///
    /// # Errors
    ///
    /// Returns error if syntax highlighting fails.
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

/// Highlights source code with syntax highlighting.
///
/// Convenience function that creates a highlighter and highlights the code.
/// For repeated highlighting, create a Highlighter instance to reuse
/// loaded syntaxes and themes.
///
/// # Errors
///
/// Returns error if highlighting fails.
pub fn highlight(code: &str, path: &Path) -> Result<Vec<String>> {
    let highlighter = Highlighter::new()?;
    highlighter.highlight(code, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_new() {
        // Act
        let result = Highlighter::new();

        // Assert
        assert!(result.is_ok(), "Should create with default theme");
    }

    #[test]
    fn test_with_theme_variations() {
        // Built-in themes
        assert!(Highlighter::with_theme("InspiredGitHub").is_ok());
        assert!(Highlighter::with_theme("base16-ocean.light").is_ok());

        // Catppuccin themes (from themes/ dir)
        assert!(Highlighter::with_theme("Catppuccin-Latte").is_ok());
        assert!(Highlighter::with_theme("Catppuccin-Mocha").is_ok());

        // Nonexistent theme
        assert!(Highlighter::with_theme("NonexistentTheme").is_err());
    }

    #[test]
    fn test_highlight_rust() {
        // Arrange
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
    }

    #[test]
    fn test_highlight_python() {
        // Arrange
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
    }

    #[test]
    fn test_highlight_javascript() {
        // Arrange
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
        let code = r#"let s = "<>&\"';"#;
        let path = Path::new("test.rs");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should escape HTML");
        let html = lines.join("");

        // Assert
        assert!(!html.contains("<>&"), "HTML entities should be escaped");
    }

    #[test]
    fn test_highlight_empty_code() {
        // Arrange
        let highlighter = Highlighter::new().expect("Should create highlighter");
        let code = "";
        let path = Path::new("test.rs");

        // Act
        let lines = highlighter
            .highlight(code, path)
            .expect("Should handle empty code");

        // Assert
        assert!(
            lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()),
            "Empty code should produce no/empty lines"
        );
    }

    #[test]
    fn test_highlight_multiline() {
        // Arrange
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
    fn test_highlight_json() {
        // Arrange
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
        let highlighter = Highlighter::new().expect("Should create highlighter");
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
