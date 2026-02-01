//! JavaScript utility scripts for interactive features

use maud::{Markup, PreEscaped, html};

/// Generates clipboard copy script for a button with visual feedback
///
/// Creates a click handler that copies content to clipboard and swaps
/// the icon from copy to check for 2 seconds as visual confirmation.
///
/// # Arguments
///
/// * `button_selector`: CSS selector for the copy button
/// * `content_expression`: JavaScript expression that evaluates to text to copy
///
/// # Returns
///
/// Script tag with configured click handler
///
/// # Examples
///
/// ```no_run
/// // Copy commit hash from adjacent code element
/// clipboard_script(
///     ".copy-hash-btn",
///     "this.parentElement.querySelector('code').textContent"
/// )
///
/// // Copy file content from table rows
/// clipboard_script(
///     ".copy-btn",
///     "Array.from(document.querySelectorAll('#blob-code .line-content')).map(r => r.textContent).join('\\n')"
/// )
/// ```
pub fn clipboard_script(button_selector: &str, content_expression: &str) -> Markup {
    let script = format!(
        r#"
document.querySelector('{button_selector}')?.addEventListener('click', async function() {{
    const content = {content_expression};
    if (!content) {{
        console.error('No content to copy');
        return;
    }}
    try {{
        await navigator.clipboard.writeText(content);
        const icon = this.querySelector('i');
        if (icon) {{
            icon.className = 'ph ph-check';
            setTimeout(() => {{
                icon.className = 'ph ph-copy';
            }}, 2000);
        }}
    }} catch (e) {{
        console.error('Copy failed:', e);
    }}
}});
"#,
        button_selector = button_selector,
        content_expression = content_expression
    );

    html! {
        script {
            (PreEscaped(script))
        }
    }
}

/// Generates script for copying commit hash from detail page
///
/// Looks for hash in adjacent code element within the button's parent.
///
/// # Returns
///
/// Script tag configured for commit hash copying
pub fn copy_commit_hash_script() -> Markup {
    clipboard_script(
        ".copy-hash-btn",
        "this.parentElement.querySelector('code').textContent",
    )
}

/// Generates script for copying entire file content from blob page
///
/// Extracts text from all line content cells in the code table
/// and joins with newlines to preserve file structure.
///
/// # Returns
///
/// Script tag configured for file content copying
pub fn copy_file_content_script() -> Markup {
    clipboard_script(
        ".copy-btn",
        "Array.from(document.querySelectorAll('#blob-code .line-content')).map(r => r.textContent).join('\\n')",
    )
}
