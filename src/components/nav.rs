//! Navigation breadcrumb component

use maud::{Markup, html};

/// Renders breadcrumb navigation
///
/// Displays hierarchical path navigation with repository name as root link
/// and path components as breadcrumb trail. Used in blob and tree pages
/// to show current location and enable quick navigation.
///
/// # Arguments
///
/// * `repo_name`: Repository name for root breadcrumb link
/// * `index_path`: Relative path back to index.html
/// * `components`: Path components with optional link targets (None for current)
/// * `ref_name`: Git reference (branch/tag) being viewed
///
/// # Returns
///
/// Breadcrumb navigation markup with links and separators
pub fn breadcrumb(
    repo_name: &str,
    index_path: &str,
    components: &[(&str, Option<String>)],
    ref_name: &str,
) -> Markup {
    html! {
        header {
            div class="breadcrumb" {
                a href=(index_path) class="breadcrumb-link" { (repo_name) }
                @for (component, href) in components {
                    span class="breadcrumb-separator" { "/" }
                    @if let Some(link) = href {
                        a href=(link) class="breadcrumb-link" { (*component) }
                    } @else {
                        span class="breadcrumb-current" { (*component) }
                    }
                }
            }
            div class="ref-info" {
                span class="ref-label" { "ref: " }
                span class="ref-name" { (ref_name) }
            }
        }
    }
}

/// Extracts path components from file path
///
/// Splits path string on forward slashes and filters empty components.
/// Used to build breadcrumb navigation from file paths.
///
/// # Arguments
///
/// * `path`: File path string
///
/// # Returns
///
/// Vector of path component strings
pub fn path_components(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}
