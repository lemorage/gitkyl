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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_icon_info_readme() {
        // Arrange & Act & Assert: Test README file icon detection
        let (icon, modifier) = icon_classes("README.md");
        assert_eq!(icon, "ph ph-info", "README should use info icon");
        assert_eq!(
            modifier,
            Some("icon-readme"),
            "README should have icon-readme modifier"
        );

        let (icon, modifier) = icon_classes("readme.txt");
        assert_eq!(icon, "ph ph-info", "lowercase readme should use info icon");
        assert_eq!(modifier, Some("icon-readme"), "Should have modifier");

        let (icon, modifier) = icon_classes("README");
        assert_eq!(
            icon, "ph ph-info",
            "README without extension should use info icon"
        );
        assert_eq!(modifier, Some("icon-readme"), "Should have modifier");
    }

    #[test]
    fn test_get_file_icon_info_rust_files() {
        // Arrange & Act & Assert
        let (icon, modifier) = icon_classes("main.rs");
        assert_eq!(icon, "ph ph-file-rs", "Rust files should use rs icon");
        assert_eq!(modifier, Some("icon-rust"), "Should have rust modifier");
    }

    #[test]
    fn test_get_file_icon_info_config_files() {
        // Arrange & Act & Assert
        let (icon, modifier) = icon_classes("Cargo.toml");
        assert_eq!(icon, "ph ph-gear", "TOML should use gear icon");
        assert_eq!(modifier, Some("icon-config"), "Should have config modifier");

        let (icon, _modifier) = icon_classes("config.yaml");
        assert_eq!(icon, "ph ph-gear", "YAML should use gear icon");
    }

    #[test]
    fn test_get_file_icon_info_directories() {
        // Arrange & Act & Assert
        let (icon, modifier) = icon_classes("src/");
        assert_eq!(
            icon, "ph-fill ph-folder",
            "Directories should use folder icon"
        );
        assert_eq!(modifier, Some("icon-folder"), "Should have folder modifier");
    }

    #[test]
    fn test_get_file_icon_info_generic() {
        // Arrange & Act & Assert
        let (icon, modifier) = icon_classes("unknown.xyz");
        assert_eq!(icon, "ph ph-file", "Unknown files should use generic icon");
        assert_eq!(modifier, None, "Should have no modifier");
    }

    #[test]
    fn test_readme_vs_other_markdown() {
        // Arrange & Act: Test that README is prioritized over extension
        let (readme_icon, readme_mod) = icon_classes("README.md");
        let (doc_icon, doc_mod) = icon_classes("CONTRIBUTING.md");

        // Assert: README should use info icon, other .md should use generic
        assert_eq!(readme_icon, "ph ph-info", "README should use info icon");
        assert_eq!(
            readme_mod,
            Some("icon-readme"),
            "README has readme modifier"
        );

        assert_eq!(doc_icon, "ph ph-file", "Other markdown uses generic icon");
        assert_eq!(doc_mod, None, "Other markdown has no modifier");
    }
}
