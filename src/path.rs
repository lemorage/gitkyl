//! Path utilities for HTML generation

/// Calculates relative path depth for HTML pages.
///
/// Determines how many `../` prefixes are needed to reach repository root
/// from generated HTML pages. Accounts for slashes in branch names
/// (e.g., "fix/bug") and nested file paths.
///
/// # Arguments
///
/// * `branch`: Branch or reference name (may contain slashes)
/// * `path`: File or directory path (empty string for root level pages)
///
/// # Returns
///
/// Number of directory levels needed to traverse back to root
pub fn calculate_depth(branch: &str, path: &str) -> usize {
    let branch_depth = branch.matches('/').count() + 1;
    let path_depth = if path.is_empty() {
        0
    } else {
        path.matches('/').count()
    };
    branch_depth + path_depth + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_level_branch_root() {
        assert_eq!(calculate_depth("dev", ""), 2);
        assert_eq!(calculate_depth("master", ""), 2);
        assert_eq!(calculate_depth("main", ""), 2);
    }

    #[test]
    fn test_multi_level_branch_root() {
        assert_eq!(calculate_depth("fix/bug", ""), 3);
        assert_eq!(calculate_depth("feature/new-ui", ""), 3);
        assert_eq!(calculate_depth("fix/delay", ""), 3);
        assert_eq!(calculate_depth("a/b/c", ""), 4);
    }

    #[test]
    fn test_single_level_branch_with_path() {
        assert_eq!(calculate_depth("dev", "src"), 2);
        assert_eq!(calculate_depth("dev", "README.md"), 2);
        assert_eq!(calculate_depth("dev", "src/main.rs"), 3);
        assert_eq!(calculate_depth("dev", "src/pages/index.rs"), 4);
    }

    #[test]
    fn test_multi_level_branch_with_path() {
        assert_eq!(calculate_depth("fix/bug", "src"), 3);
        assert_eq!(calculate_depth("fix/bug", "src/main.rs"), 4);
        assert_eq!(calculate_depth("feature/ui", "assets/styles.css"), 4);
    }
}
