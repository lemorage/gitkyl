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

/// Extracts breadcrumb path components from file path
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
/// Vector of path component strings for breadcrumb display
pub fn extract_breadcrumb_components(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Builds breadcrumb link data from path components
///
/// Maps each component to a `(name, link)` pair where all components
/// except the last get a link to their tree page, and the last gets
/// `None` (current location, no link).
///
/// # Arguments
///
/// * `components`: Path segments from `extract_breadcrumb_components`
/// * `depth`: Directory depth for relative path calculation
/// * `ref_name`: Git reference being viewed
pub fn build_breadcrumb_data<'a>(
    components: &[&'a str],
    depth: usize,
    ref_name: &str,
) -> Vec<(&'a str, Option<String>)> {
    components
        .iter()
        .enumerate()
        .map(|(idx, &component)| {
            if idx == components.len() - 1 {
                (component, None)
            } else {
                let partial_path = components[..=idx].join("/");
                let link = format!(
                    "{}tree/{}/{}.html",
                    "../".repeat(depth),
                    ref_name,
                    partial_path
                );
                (component, Some(link))
            }
        })
        .collect()
}
