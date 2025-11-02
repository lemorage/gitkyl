//! Syntax highlighting with tree-sitter.

use anyhow::Result;
use std::path::Path;

/// Supported languages for syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
}

impl Language {
    /// Detects language from file extension.
    ///
    /// Returns None if the extension is not recognized or supported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitkyl::Language;
    /// use std::path::Path;
    ///
    /// let lang = Language::from_extension(Path::new("main.rs"));
    /// assert_eq!(lang, Some(Language::Rust));
    /// ```
    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        match ext {
            "rs" => Some(Language::Rust),
            _ => None,
        }
    }

    /// Returns tree-sitter language parser for this language.
    fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        }
    }

    /// Returns the language name as string.
    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
        }
    }
}

/// HTML class names for syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightClass {
    Comment,
    String,
    Number,
    Keyword,
    Type,
    Function,
    Variable,
    Property,
    Operator,
    Punctuation,
}

impl HighlightClass {
    /// CSS class name for this highlight type.
    pub fn css_class(&self) -> &'static str {
        match self {
            HighlightClass::Comment => "hl-comment",
            HighlightClass::String => "hl-string",
            HighlightClass::Number => "hl-number",
            HighlightClass::Keyword => "hl-keyword",
            HighlightClass::Type => "hl-type",
            HighlightClass::Function => "hl-function",
            HighlightClass::Variable => "hl-variable",
            HighlightClass::Property => "hl-property",
            HighlightClass::Operator => "hl-operator",
            HighlightClass::Punctuation => "hl-punctuation",
        }
    }

    /// Maps tree-sitter node kind to highlight class.
    fn from_node_kind(kind: &str) -> Option<Self> {
        match kind {
            "line_comment" | "block_comment" => Some(HighlightClass::Comment),
            "string_literal" | "raw_string_literal" | "char_literal" => {
                Some(HighlightClass::String)
            }
            "integer_literal" | "float_literal" => Some(HighlightClass::Number),
            "let" | "mut" | "fn" | "pub" | "use" | "mod" | "impl" | "trait" | "struct" | "enum"
            | "type" | "const" | "static" | "match" | "if" | "else" | "while" | "for" | "loop"
            | "break" | "continue" | "return" | "async" | "await" | "unsafe" | "move" | "ref"
            | "self" | "super" | "crate" | "where" | "as" | "in" => Some(HighlightClass::Keyword),
            "type_identifier" | "primitive_type" => Some(HighlightClass::Type),
            "function_item" | "call_expression" => Some(HighlightClass::Function),
            "identifier" => Some(HighlightClass::Variable),
            "field_identifier" | "field_expression" => Some(HighlightClass::Property),
            "+" | "-" | "*" | "/" | "%" | "=" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&"
            | "||" | "!" | "&" | "|" | "^" | "<<" | ">>" | "+=" | "-=" | "*=" | "/=" | "%="
            | "&=" | "|=" | "^=" | "<<=" | ">>=" => Some(HighlightClass::Operator),
            ";" | "," | "." | "::" | ":" | "->" | "=>" => Some(HighlightClass::Punctuation),
            _ => None,
        }
    }
}

/// Highlighted segment in source code.
#[derive(Debug, Clone)]
struct Segment {
    start: usize,
    end: usize,
    class: HighlightClass,
}

/// Highlights source code with syntax highlighting.
///
/// Attempts to detect the language from the file path extension.
/// If detection fails or the language is unsupported, returns plain HTML
/// without syntax highlighting.
///
/// # Arguments
///
/// * `code`: Source code to highlight
/// * `path`: File path used for language detection
///
/// # Returns
///
/// HTML string with syntax highlighting spans. Each highlighted segment
/// is wrapped in a span with the appropriate CSS class.
///
/// # Errors
///
/// Returns error if tree-sitter parsing fails unexpectedly.
///
/// # Examples
///
/// ```no_run
/// use gitkyl::highlight;
/// use std::path::Path;
///
/// let code = "fn main() { println!(\"Hello\"); }";
/// let html = highlight(code, Path::new("main.rs"))?;
/// assert!(html.contains("hl-keyword"));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn highlight(code: &str, path: &Path) -> Result<String> {
    let Some(language) = Language::from_extension(path) else {
        return Ok(escape_html(code));
    };

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language.tree_sitter_language())
        .map_err(|e| anyhow::anyhow!("Failed to set tree-sitter language: {}", e))?;

    let tree = parser
        .parse(code, None)
        .ok_or_else(|| anyhow::anyhow!("Tree-sitter parsing failed"))?;

    let root = tree.root_node();

    let mut segments = Vec::new();
    collect_segments(root, &mut segments);

    segments.sort_by_key(|s| s.start);

    let mut result = String::with_capacity(code.len() * 2);
    let mut pos = 0;

    for segment in segments {
        if segment.start > pos {
            result.push_str(&escape_html(&code[pos..segment.start]));
        }

        let text = &code[segment.start..segment.end];
        result.push_str(&format!(
            "<span class=\"{}\">{}</span>",
            segment.class.css_class(),
            escape_html(text)
        ));

        pos = segment.end;
    }

    if pos < code.len() {
        result.push_str(&escape_html(&code[pos..]));
    }

    Ok(result)
}

/// Recursively collects highlighted segments from tree-sitter AST.
fn collect_segments(node: tree_sitter::Node, segments: &mut Vec<Segment>) {
    if let Some(class) = HighlightClass::from_node_kind(node.kind()) {
        segments.push(Segment {
            start: node.start_byte(),
            end: node.end_byte(),
            class,
        });
    }

    for child in node.children(&mut node.walk()) {
        collect_segments(child, segments);
    }
}

/// Escapes HTML special characters.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection_rust() {
        // Arrange
        let path = Path::new("main.rs");

        // Act
        let lang = Language::from_extension(path);

        // Assert
        assert_eq!(lang, Some(Language::Rust));
    }

    #[test]
    fn test_language_detection_unsupported() {
        // Arrange
        let path = Path::new("script.py");

        // Act
        let lang = Language::from_extension(path);

        // Assert
        assert_eq!(lang, None);
    }

    #[test]
    fn test_language_detection_no_extension() {
        // Arrange
        let path = Path::new("README");

        // Act
        let lang = Language::from_extension(path);

        // Assert
        assert_eq!(lang, None);
    }

    #[test]
    fn test_highlight_rust_keywords() {
        // Arrange
        let code = "fn main() {}";
        let path = Path::new("test.rs");

        // Act
        let html = highlight(code, path).expect("Highlighting should succeed");

        // Assert
        assert!(html.contains("hl-keyword"), "Should highlight 'fn' keyword");
    }

    #[test]
    fn test_highlight_rust_string() {
        // Arrange
        let code = r#"let s = "hello";"#;
        let path = Path::new("test.rs");

        // Act
        let html = highlight(code, path).expect("Highlighting should succeed");

        // Assert
        assert!(
            html.contains("hl-string"),
            "Should highlight string literal"
        );
        assert!(
            html.contains("hl-keyword"),
            "Should highlight 'let' keyword"
        );
    }

    #[test]
    fn test_highlight_rust_number() {
        // Arrange
        let code = "let x = 42;";
        let path = Path::new("test.rs");

        // Act
        let html = highlight(code, path).expect("Highlighting should succeed");

        // Assert
        assert!(
            html.contains("hl-number"),
            "Should highlight number literal"
        );
    }

    #[test]
    fn test_highlight_rust_comment() {
        // Arrange
        let code = "// This is a comment\nfn main() {}";
        let path = Path::new("test.rs");

        // Act
        let html = highlight(code, path).expect("Highlighting should succeed");

        // Assert
        assert!(html.contains("hl-comment"), "Should highlight line comment");
    }

    #[test]
    fn test_highlight_unsupported_language_fallback() {
        // Arrange
        let code = "print('hello')";
        let path = Path::new("script.py");

        // Act
        let html = highlight(code, path).expect("Should fallback to plain text");

        // Assert
        assert!(
            !html.contains("hl-"),
            "Should not contain highlight classes"
        );
        assert!(html.contains("print"), "Should contain original text");
    }

    #[test]
    fn test_highlight_html_escaping() {
        // Arrange
        let code = r#"let s = "<>&\"';"#;
        let path = Path::new("test.rs");

        // Act
        let html = highlight(code, path).expect("Highlighting should succeed");

        // Assert
        assert!(html.contains("&lt;"), "Should escape '<'");
        assert!(html.contains("&gt;"), "Should escape '>'");
        assert!(html.contains("&amp;"), "Should escape '&'");
        assert!(html.contains("&quot;"), "Should escape '\"'");
        assert!(html.contains("&#39;"), "Should escape '\''");
    }

    #[test]
    fn test_highlight_empty_code() {
        // Arrange
        let code = "";
        let path = Path::new("test.rs");

        // Act
        let html = highlight(code, path).expect("Should handle empty code");

        // Assert
        assert_eq!(html, "", "Should return empty string for empty input");
    }

    #[test]
    fn test_highlight_class_css_names() {
        // Arrange & Act & Assert
        assert_eq!(HighlightClass::Comment.css_class(), "hl-comment");
        assert_eq!(HighlightClass::String.css_class(), "hl-string");
        assert_eq!(HighlightClass::Number.css_class(), "hl-number");
        assert_eq!(HighlightClass::Keyword.css_class(), "hl-keyword");
        assert_eq!(HighlightClass::Type.css_class(), "hl-type");
        assert_eq!(HighlightClass::Function.css_class(), "hl-function");
    }

    #[test]
    fn test_language_name() {
        // Arrange & Act & Assert
        assert_eq!(Language::Rust.name(), "rust");
    }

    #[test]
    fn test_escape_html_all_characters() {
        // Arrange
        let input = r#"<>&"'"#;

        // Act
        let output = escape_html(input);

        // Assert
        assert_eq!(output, "&lt;&gt;&amp;&quot;&#39;");
    }
}
