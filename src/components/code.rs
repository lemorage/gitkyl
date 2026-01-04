//! Code display components with syntax highlighting and line numbers

use maud::{Markup, PreEscaped, html};

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
    html! {
        script {
            (PreEscaped(r#"
document.querySelector('.copy-btn')?.addEventListener('click', async function() {
    const rows = document.querySelectorAll('#blob-code .line-content');
    const code = Array.from(rows).map(r => r.textContent).join('\n');
    if (code) {
        try {
            await navigator.clipboard.writeText(code);
            this.classList.add('copied');
            const icon = this.querySelector('i');
            icon.className = 'ph ph-check';
            setTimeout(() => {
                this.classList.remove('copied');
                icon.className = 'ph ph-copy';
            }, 2000);
        } catch (e) { console.error('Copy failed:', e); }
    }
});
"#))
        }
    }
}
