//! Code display components with syntax highlighting and line numbers

use maud::{Markup, PreEscaped, html};

use super::scripts::copy_file_content_script;

/// Renders syntax highlighted code as a table with line numbers
pub fn code_table(highlighted_lines: &[String]) -> Markup {
    html! {
        div class="blob-code-wrapper" {
            table class="blob-code" {
                tbody id="blob-code" {
                    @for (idx, line) in highlighted_lines.iter().enumerate() {
                        @let line_num = idx + 1;
                        tr id=(format!("L{}", line_num)) class="code-line" {
                            td class="line-number" data-line=(line_num) {
                                a href=(format!("#L{}", line_num)) { (line_num) }
                            }
                            td class="line-content" {
                                (PreEscaped(line))
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Renders copy button script for clipboard functionality
pub fn copy_button_script() -> Markup {
    copy_file_content_script()
}
