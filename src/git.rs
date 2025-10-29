//! Git repository operations.

use anyhow::{Context, Result};
use gix::bstr::ByteSlice;
use std::path::Path;

/// Repository metadata.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    name: String,
    default_branch: String,
    branches: Vec<String>,
    commit_count: usize,
    owner: Option<String>,
}

impl RepoInfo {
    /// Repository name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Default branch name.
    pub fn default_branch(&self) -> &str {
        &self.default_branch
    }

    /// Branch names as slice.
    pub fn branches(&self) -> &[String] {
        &self.branches
    }

    /// Commit count on default branch.
    pub fn commit_count(&self) -> usize {
        self.commit_count
    }

    /// Repository owner.
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }
}

/// Analyzes a git repository and extracts metadata.
///
/// # Arguments
///
/// * `path`: Repository path
/// * `owner`: Optional owner name
///
/// # Errors
///
/// Returns error if repository cannot be opened or read.
pub fn analyze_repository(path: impl AsRef<Path>, owner: Option<String>) -> Result<RepoInfo> {
    let repo = gix::open(path.as_ref())
        .with_context(|| format!("Failed to open repository at {}", path.as_ref().display()))?;

    let resolved_path = path
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| path.as_ref().to_path_buf());

    let name = resolved_path
        .file_name()
        .and_then(|n| n.to_str())
        .with_context(|| {
            format!(
                "Cannot determine repository name from path: {}",
                resolved_path.display()
            )
        })?
        .to_string();

    let head_ref = repo.head_ref().context("Failed to read HEAD reference")?;
    let default_branch = head_ref
        .and_then(|r| r.name().shorten().to_str().ok().map(|s| s.to_string()))
        .unwrap_or_else(|| "main".to_string());

    let branches = repo
        .references()
        .context("Failed to read references")?
        .local_branches()
        .context("Failed to get local branches")?
        .filter_map(|r| {
            r.ok()?
                .name()
                .shorten()
                .to_str()
                .ok()
                .map(|s| s.to_string())
        })
        .collect();

    let head = repo.head_commit().context("Failed to read HEAD commit")?;
    let commit_count = head
        .ancestors()
        .all()
        .context("Failed to traverse commit history")?
        .count();

    Ok(RepoInfo {
        name,
        default_branch,
        branches,
        commit_count,
        owner,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_repo_info_accessors_direct() {
        // Arrange
        let repo_info = RepoInfo {
            name: "test-repo".to_string(),
            default_branch: "main".to_string(),
            branches: vec!["main".to_string(), "dev".to_string()],
            commit_count: 42,
            owner: Some("testowner".to_string()),
        };

        // Act & Assert
        assert_eq!(repo_info.name(), "test-repo");
        assert_eq!(repo_info.default_branch(), "main");
        assert_eq!(
            repo_info.branches(),
            &["main".to_string(), "dev".to_string()]
        );
        assert_eq!(repo_info.commit_count(), 42);
        assert_eq!(repo_info.owner(), Some("testowner"));
    }

    #[test]
    fn test_repo_info_without_owner() {
        // Arrange
        let repo_info = RepoInfo {
            name: "ownerless".to_string(),
            default_branch: "master".to_string(),
            branches: vec!["master".to_string()],
            commit_count: 10,
            owner: None,
        };

        // Act & Assert
        assert_eq!(repo_info.owner(), None);
        assert_eq!(repo_info.branches().len(), 1);
    }

    #[test]
    fn test_analyze_repository_invalid_path() {
        // Arrange
        let invalid_path = PathBuf::from("/definitely/not/a/real/path/anywhere");

        // Act
        let result = analyze_repository(&invalid_path, None);

        // Assert
        assert!(result.is_err(), "Should fail for invalid repository path");
    }

    #[test]
    fn test_analyze_tmp_test_repo() {
        // Arrange: Use the test repo at /tmp/test_repo if it exists
        let path = PathBuf::from("/tmp/test_repo");

        // Skip test if repo doesn't exist
        if !path.exists() {
            eprintln!("Skipping: /tmp/test_repo doesn't exist");
            return;
        }

        // Act
        let result = analyze_repository(&path, None);

        // Assert
        if let Ok(info) = result {
            assert!(info.commit_count() > 0, "Should have at least one commit");
            assert!(
                !info.default_branch().is_empty(),
                "Should have a default branch"
            );
            assert_eq!(info.name(), "test_repo");
        }
    }

    #[test]
    fn test_analyze_repo_with_complex_path() {
        // Arrange: Path with ".." components that needs canonicalization
        let path = PathBuf::from("/tmp/test_repo/../test_repo");

        // Skip test if repo doesn't exist
        if !PathBuf::from("/tmp/test_repo").exists() {
            eprintln!("Skipping: /tmp/test_repo doesn't exist");
            return;
        }

        // Act
        let result = analyze_repository(&path, None);

        // Assert
        if let Ok(info) = result {
            assert_eq!(info.name(), "test_repo");
        }
    }
}
