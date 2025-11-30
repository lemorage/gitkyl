//! Page layout wrapper component

use maud::{DOCTYPE, Markup, html};

use super::footer::footer;

/// Wraps page content with standard HTML structure
///
/// Provides consistent DOCTYPE, html, head, and container structure across
/// all page types. The wrapper handles viewport configuration, charset, and
/// stylesheet loading while the caller provides page-specific body content.
///
/// # Arguments
///
/// * `title`: Page title text (without suffix)
/// * `stylesheets`: Array of CSS file paths to include
/// * `body`: Page-specific body markup
///
/// # Returns
///
/// Complete HTML document with wrapped content
pub fn page_wrapper(title: &str, stylesheets: &[&str], body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) " - Gitkyl" }
                script src="https://unpkg.com/@phosphor-icons/web" {}
                @for stylesheet in stylesheets {
                    link rel="stylesheet" href=(stylesheet);
                }
            }
            body {
                div class="container" {
                    (body)
                }
                (footer())
            }
        }
    }
}
