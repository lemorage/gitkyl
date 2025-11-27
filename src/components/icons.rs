//! File type icon rendering and detection

use maud::{Markup, html};
use std::path::Path;

/// Renders file icon based on path
///
/// Generates Phosphor icon HTML with appropriate CSS classes for file type
/// visual distinction. Icon selection is based on file extension and patterns.
///
/// # Arguments
///
/// * `path`: File path relative to repository root
///
/// # Returns
///
/// Icon markup with appropriate Phosphor icon class and color modifier
pub fn file_icon(path: &str) -> Markup {
    let (icon_class, icon_modifier) = icon_classes(path);

    html! {
        div class="icon-box" {
            @if let Some(modifier) = icon_modifier {
                i class=(format!("{} {}", icon_class, modifier)) {}
            } @else {
                i class=(icon_class) {}
            }
        }
    }
}

/// Returns Phosphor icon classes for file type
///
/// Matches file paths to appropriate icon classes based on extension
/// and filename patterns. Icon colors are controlled via CSS modifier classes.
///
/// # Arguments
///
/// * `path`: File path relative to repository root
///
/// # Returns
///
/// Phosphor icon class name and optional CSS modifier class for color styling
pub fn icon_classes(path: &str) -> (&'static str, Option<&'static str>) {
    if path.ends_with('/') {
        return ("ph-fill ph-folder", Some("icon-folder"));
    }

    let path_lower = path.to_lowercase();
    let file_name = Path::new(&path_lower)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if file_name.starts_with("readme") {
        return ("ph ph-info", Some("icon-readme"));
    }

    if let Some(ext) = Path::new(&path_lower).extension().and_then(|e| e.to_str()) {
        match ext {
            "rs" => ("ph ph-file-rs", Some("icon-rust")),
            "toml" | "yaml" | "yml" => ("ph ph-gear", Some("icon-config")),
            _ => ("ph ph-file", None),
        }
    } else {
        ("ph ph-file", None)
    }
}

/// Checks if file path is a README file
///
/// Detects README files case insensitively with or without extension.
/// Recognized patterns: README, README.md, README.MD, readme.md, Readme.md
///
/// # Arguments
///
/// * `path`: File path to check
///
/// # Returns
///
/// True if file is a README, false otherwise
pub fn is_readme(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase().starts_with("readme"))
        .unwrap_or(false)
}
