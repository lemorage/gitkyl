//! Link resolution for repository internal references.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Resolves relative links in markdown to static blob/tree pages.
///
/// Transforms repository internal links (./file.rs, ../docs/) into
/// URLs pointing to generated static pages (/blob/branch/file.rs.html).
pub struct LinkResolver {
    branch: String,
    current_path: PathBuf,
}

impl LinkResolver {
    /// Creates link resolver for specified branch and current file.
    ///
    /// # Arguments
    ///
    /// * `branch`: Git branch name for link resolution
    /// * `current_path`: Path to current markdown file being rendered
    pub fn new(branch: impl Into<String>, current_path: impl AsRef<Path>) -> Self {
        Self {
            branch: branch.into(),
            current_path: current_path.as_ref().to_path_buf(),
        }
    }

    /// Resolves link to absolute URL for static site.
    ///
    /// Handles different link types:
    /// - Absolute URLs (http://, https://) remain unchanged
    /// - Anchor links (#section) remain unchanged
    /// - Relative paths (./file.rs) resolve to /blob/branch/path.html
    /// - Parent paths (../file.rs) resolve relative to current file
    /// - Directory links (./dir/) resolve to /tree/branch/dir.html
    ///
    /// # Arguments
    ///
    /// * `link`: Link href from markdown
    /// * `is_image`: Whether link is for image (skips .html extension)
    ///
    /// # Returns
    ///
    /// Resolved absolute URL for static site
    ///
    /// # Errors
    ///
    /// Returns error if path resolution fails or contains invalid characters
    pub fn resolve(&self, link: &str, is_image: bool) -> Result<String> {
        // Absolute URLs unchanged
        if link.starts_with("http://") || link.starts_with("https://") {
            return Ok(link.to_string());
        }

        // Anchor links unchanged
        if link.starts_with('#') {
            return Ok(link.to_string());
        }

        // Relative paths: resolve to current file directory
        let current_dir = self.current_path.parent().unwrap_or_else(|| Path::new(""));

        // Join and normalize path
        let target_path = current_dir.join(link);
        let normalized = self
            .normalize_path(&target_path)
            .context("Failed to normalize path")?;

        let path_str = normalized.to_str().context("Path contains invalid UTF8")?;

        // Check if directory (ends with /)
        if link.ends_with('/') {
            return Ok(format!("/tree/{}/{}.html", self.branch, path_str));
        }

        // Regular file: blob page
        if is_image {
            // Images: raw file path without .html
            Ok(format!("/blob/{}/{}", self.branch, path_str))
        } else {
            // Links: HTML page
            Ok(format!("/blob/{}/{}.html", self.branch, path_str))
        }
    }

    /// Normalizes path by resolving .. and . components.
    ///
    /// Security: Prevents directory traversal outside repository root.
    ///
    /// # Arguments
    ///
    /// * `path`: Path to normalize
    ///
    /// # Returns
    ///
    /// Normalized path relative to repository root
    ///
    /// # Errors
    ///
    /// Returns error if path attempts to escape repository root
    fn normalize_path(&self, path: &Path) -> Result<PathBuf> {
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::Normal(c) => {
                    components.push(c);
                }
                std::path::Component::ParentDir => {
                    if components.is_empty() {
                        bail!("Path escapes repository root: {}", path.display());
                    }
                    components.pop();
                }
                std::path::Component::CurDir => {
                    // Skip current directory markers
                }
                _ => {
                    // Skip prefix, root directory markers
                }
            }
        }

        Ok(components.iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative_link() {
        // Arrange
        let resolver = LinkResolver::new("main", "docs/guide.md");

        // Act
        let result = resolver.resolve("./api.md", false).expect("Should resolve");

        // Assert
        assert_eq!(result, "/blob/main/docs/api.md.html");
    }

    #[test]
    fn test_resolve_parent_directory_link() {
        // Arrange
        let resolver = LinkResolver::new("main", "docs/guide.md");

        // Act
        let result = resolver
            .resolve("../README.md", false)
            .expect("Should resolve");

        // Assert
        assert_eq!(result, "/blob/main/README.md.html");
    }

    #[test]
    fn test_resolve_absolute_url_unchanged() {
        // Arrange
        let resolver = LinkResolver::new("main", "README.md");

        // Act
        let result = resolver
            .resolve("https://example.com", false)
            .expect("Should pass through");

        // Assert
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn test_resolve_anchor_link_unchanged() {
        // Arrange
        let resolver = LinkResolver::new("main", "README.md");

        // Act
        let result = resolver
            .resolve("#section", false)
            .expect("Should pass through");

        // Assert
        assert_eq!(result, "#section");
    }

    #[test]
    fn test_resolve_image_path() {
        // Arrange
        let resolver = LinkResolver::new("main", "docs/guide.md");

        // Act
        let result = resolver
            .resolve("./assets/logo.png", true)
            .expect("Should resolve image");

        // Assert
        assert_eq!(
            result, "/blob/main/docs/assets/logo.png",
            "Images should not have .html extension"
        );
    }

    #[test]
    fn test_reject_traversal_attack() {
        // Arrange
        let resolver = LinkResolver::new("main", "README.md");

        // Act
        let result = resolver.resolve("../../../etc/passwd", false);

        // Assert
        assert!(
            result.is_err(),
            "Should reject path escaping repository root"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("escapes")
                || err_msg.contains("repository")
                || err_msg.contains("normalize"),
            "Error should indicate path problem: {}",
            err_msg
        );
    }

    #[test]
    fn test_resolve_root_readme_link() {
        // Arrange
        let resolver = LinkResolver::new("main", "README.md");

        // Act
        let result = resolver
            .resolve("./src/main.rs", false)
            .expect("Should resolve from root");

        // Assert
        assert_eq!(result, "/blob/main/src/main.rs.html");
    }

    #[test]
    fn test_resolve_nested_readme_link() {
        // Arrange
        let resolver = LinkResolver::new("develop", "docs/api/README.md");

        // Act
        let result = resolver
            .resolve("../../src/lib.rs", false)
            .expect("Should resolve nested path");

        // Assert
        assert_eq!(result, "/blob/develop/src/lib.rs.html");
    }

    #[test]
    fn test_resolve_link_to_directory() {
        // Arrange
        let resolver = LinkResolver::new("main", "README.md");

        // Act
        let result = resolver
            .resolve("./src/", false)
            .expect("Should resolve directory");

        // Assert
        assert_eq!(
            result, "/tree/main/src.html",
            "Directory links should use /tree/ prefix"
        );
    }

    #[test]
    fn test_resolve_http_url_unchanged() {
        // Arrange
        let resolver = LinkResolver::new("main", "README.md");

        // Act
        let result = resolver
            .resolve("http://example.com/page", false)
            .expect("Should pass through HTTP");

        // Assert
        assert_eq!(result, "http://example.com/page");
    }

    #[test]
    fn test_resolve_sibling_file() {
        // Arrange
        let resolver = LinkResolver::new("main", "docs/guide.md");

        // Act
        let result = resolver
            .resolve("./tutorial.md", false)
            .expect("Should resolve sibling");

        // Assert
        assert_eq!(result, "/blob/main/docs/tutorial.md.html");
    }

    #[test]
    fn test_resolve_current_dir_marker() {
        // Arrange
        let resolver = LinkResolver::new("main", "src/lib.rs");

        // Act
        let result = resolver
            .resolve("./config.rs", false)
            .expect("Should handle current directory marker");

        // Assert
        assert_eq!(result, "/blob/main/src/config.rs.html");
    }

    #[test]
    fn test_resolve_multiple_parent_dirs() {
        // Arrange
        let resolver = LinkResolver::new("main", "src/module/submodule/file.rs");

        // Act
        let result = resolver
            .resolve("../../../README.md", false)
            .expect("Should resolve multiple parent dirs");

        // Assert
        assert_eq!(result, "/blob/main/README.md.html");
    }
}
