//! Git repository operations.

use anyhow::{Context, Result};
use gix::bstr::{BString, ByteSlice};
use std::path::Path;

/// File entry in repository tree.
#[derive(Debug, Clone)]
pub struct FileEntry {
    path: BString,
    oid: gix::ObjectId,
}

impl FileEntry {
    /// File path relative to repository root.
    ///
    /// Returns None if path contains platform-incompatible characters.
    pub fn path(&self) -> Option<&Path> {
        self.path.to_path().ok()
    }

    /// Git object ID.
    pub fn oid(&self) -> &gix::ObjectId {
        &self.oid
    }

    /// Git object ID as hexadecimal string.
    pub fn oid_hex(&self) -> String {
        self.oid.to_hex().to_string()
    }
}

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

/// Resolves reference to commit object.
fn resolve_commit<'a>(
    repo: &'a gix::Repository,
    ref_name: Option<&str>,
) -> Result<gix::Commit<'a>> {
    match ref_name {
        Some(ref_str) => repo
            .find_reference(ref_str)
            .with_context(|| format!("Failed to find reference: {}", ref_str))?
            .into_fully_peeled_id()
            .with_context(|| format!("Failed to peel reference '{}'", ref_str))?
            .object()
            .context("Failed to resolve object")?
            .try_into_commit()
            .map_err(|_| anyhow::anyhow!("Reference '{}' does not point to a commit", ref_str)),
        None => repo.head_commit().context("Failed to read HEAD commit"),
    }
}

/// Reads blob content from repository at given reference and path.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Reference name (branch/tag/commit), defaults to HEAD if None
/// * `file_path`: Path to file within repository tree
///
/// # Returns
///
/// Blob content as bytes
///
/// # Errors
///
/// Returns error if:
/// - Repository cannot be opened
/// - Reference cannot be resolved
/// - File does not exist in tree
/// - Blob cannot be read
pub fn read_blob(
    repo_path: impl AsRef<Path>,
    ref_name: Option<&str>,
    file_path: impl AsRef<Path>,
) -> Result<Vec<u8>> {
    let repo = gix::open(repo_path.as_ref()).with_context(|| {
        format!(
            "Failed to open repository at {}",
            repo_path.as_ref().display()
        )
    })?;

    let commit = resolve_commit(&repo, ref_name)?;

    let mut tree = commit.tree().context("Failed to read commit tree")?;

    let entry = tree
        .peel_to_entry_by_path(file_path.as_ref())
        .context("Failed to traverse tree to path")?
        .ok_or_else(|| {
            anyhow::anyhow!("File not found in tree: {}", file_path.as_ref().display())
        })?;

    let object = entry.object().context("Failed to read tree entry object")?;

    let blob = object
        .try_into_blob()
        .map_err(|_| anyhow::anyhow!("Path is not a blob: {}", file_path.as_ref().display()))?;

    Ok(blob.data.to_vec())
}

/// Lists all files in repository at given reference.
///
/// Traverses the tree at the specified reference using breadth-first order,
/// returning all blob entries (regular files). Excludes directories, symlinks,
/// and submodules.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Reference name (branch/tag/commit), defaults to HEAD if None
///
/// # Returns
///
/// Vector of FileEntry containing path and object ID for each blob
///
/// # Errors
///
/// Returns error if:
/// - Repository cannot be opened
/// - Reference cannot be resolved
/// - Tree cannot be traversed
///
/// # Examples
///
/// ```no_run
/// use gitkyl::list_files;
/// use std::path::Path;
///
/// let files = list_files(Path::new("."), None)?;
/// for entry in files {
///     if let Some(path) = entry.path() {
///         println!("{}: {}", path.display(), entry.oid_hex());
///     }
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn list_files(repo_path: impl AsRef<Path>, ref_name: Option<&str>) -> Result<Vec<FileEntry>> {
    let repo = gix::open(repo_path.as_ref()).with_context(|| {
        format!(
            "Failed to open repository at {}",
            repo_path.as_ref().display()
        )
    })?;

    let commit = resolve_commit(&repo, ref_name)?;

    let tree = commit.tree().context("Failed to read commit tree")?;

    // Traverse tree in breadth-first order for consistent output
    let files = tree
        .traverse()
        .breadthfirst
        .files()
        .context("Failed to traverse tree")?
        .into_iter()
        .map(|entry| FileEntry {
            path: entry.filepath,
            oid: entry.oid,
        })
        .collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

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
    fn test_list_files_default_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act
        let files = list_files(&repo_path, None).expect("Should list files from HEAD");

        // Assert
        assert!(!files.is_empty(), "Repository should contain files");
        assert!(
            files.iter().any(|entry| {
                entry
                    .path()
                    .and_then(|p| p.to_str())
                    .map_or(false, |s| s.contains("Cargo.toml"))
            }),
            "Should find Cargo.toml in repository"
        );
        assert!(
            files.iter().all(|entry| !entry.oid_hex().is_empty()),
            "All entries should have valid OIDs"
        );
    }

    #[test]
    fn test_list_files_specific_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act
        let files =
            list_files(&repo_path, Some("HEAD")).expect("Should list files from HEAD reference");

        // Assert
        assert!(!files.is_empty(), "Branch should contain files");
        assert!(
            files
                .iter()
                .all(|entry| entry.path().map_or(false, |p| p.is_relative())),
            "All paths should be relative to repository root"
        );
    }

    #[test]
    fn test_list_files_invalid_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let invalid_ref = "refs/heads/this_branch_definitely_does_not_exist_12345";

        // Act
        let result = list_files(&repo_path, Some(invalid_ref));

        // Assert
        assert!(
            result.is_err(),
            "Should return error for nonexistent reference"
        );
    }

    #[test]
    fn test_read_blob_default_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let file_path = Path::new("Cargo.toml");

        // Act
        let content =
            read_blob(&repo_path, None, file_path).expect("Should read Cargo.toml from HEAD");

        // Assert
        let content_str = String::from_utf8_lossy(&content);
        assert!(
            content_str.contains("[package]"),
            "Should contain package section"
        );
        assert!(
            content_str.contains("gitkyl"),
            "Should contain package name"
        );
    }

    #[test]
    fn test_read_blob_specific_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let file_path = Path::new("Cargo.toml");

        // Act
        let content = read_blob(&repo_path, Some("HEAD"), file_path)
            .expect("Should read file from HEAD reference");

        // Assert
        assert!(!content.is_empty(), "File content should not be empty");
    }

    #[test]
    fn test_read_blob_nonexistent_file() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let file_path = Path::new("this_file_does_not_exist_12345.txt");

        // Act
        let result = read_blob(&repo_path, None, file_path);

        // Assert
        assert!(result.is_err(), "Should return error for nonexistent file");
    }

    #[test]
    fn test_read_blob_directory_path() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir_path = Path::new("src");

        // Act
        let result = read_blob(&repo_path, None, dir_path);

        // Assert
        assert!(
            result.is_err(),
            "Should return error when path is directory"
        );
    }
}
