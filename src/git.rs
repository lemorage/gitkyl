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

/// Commit metadata.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    oid: String,
    short_oid: String,
    author: String,
    author_email: String,
    committer: String,
    date: i64,
    message: String,
    message_full: String,
}

impl CommitInfo {
    /// Full commit hash.
    pub fn oid(&self) -> &str {
        &self.oid
    }

    /// Short commit hash (7 characters).
    pub fn short_oid(&self) -> &str {
        &self.short_oid
    }

    /// Author name.
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Author email.
    pub fn author_email(&self) -> &str {
        &self.author_email
    }

    /// Committer name.
    pub fn committer(&self) -> &str {
        &self.committer
    }

    /// Commit timestamp (Unix seconds).
    pub fn date(&self) -> i64 {
        self.date
    }

    /// First line of commit message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Full commit message.
    pub fn message_full(&self) -> &str {
        &self.message_full
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

/// Lists commits for a given reference in reverse chronological order.
///
/// Traverses commit history from the specified reference, extracting metadata
/// for each commit including hash, author, date, and message. Returns commits
/// in reverse chronological order (newest first).
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Reference name (branch/tag/commit), defaults to HEAD if None
/// * `limit`: Optional limit on number of commits to retrieve
///
/// # Returns
///
/// Vector of CommitInfo structs with metadata for each commit
///
/// # Errors
///
/// Returns error if:
/// - Repository cannot be opened
/// - Reference cannot be resolved
/// - Commit traversal fails
///
/// # Examples
///
/// ```no_run
/// use gitkyl::list_commits;
/// use std::path::Path;
///
/// let commits = list_commits(Path::new("."), None, Some(10))?;
/// for commit in commits {
///     println!("{}: {}", commit.short_oid(), commit.message());
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn list_commits(
    repo_path: impl AsRef<Path>,
    ref_name: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<CommitInfo>> {
    let repo = gix::open(repo_path.as_ref()).with_context(|| {
        format!(
            "Failed to open repository at {}",
            repo_path.as_ref().display()
        )
    })?;

    let commit = resolve_commit(&repo, ref_name)?;

    let mut commits = Vec::new();
    let walker = commit
        .ancestors()
        .all()
        .context("Failed to create commit ancestor iterator")?;

    for (idx, result) in walker.enumerate() {
        if let Some(max) = limit
            && idx >= max
        {
            break;
        }

        let commit_info = result.context("Failed to traverse commit ancestor")?;
        let commit_obj = commit_info
            .object()
            .context("Failed to read commit object")?;

        let author = commit_obj.author().context("Failed to read author")?;
        let committer = commit_obj.committer().context("Failed to read committer")?;
        let message_bytes = commit_obj
            .message_raw()
            .context("Failed to read commit message")?;
        let message_full = message_bytes.to_str_lossy().to_string();
        let first_line = message_full.lines().next().unwrap_or("").to_string();

        commits.push(CommitInfo {
            oid: commit_obj.id.to_hex().to_string(),
            short_oid: commit_obj.id.to_hex_with_len(7).to_string(),
            author: author.name.to_str_lossy().to_string(),
            author_email: author.email.to_str_lossy().to_string(),
            committer: committer.name.to_str_lossy().to_string(),
            date: author.time.seconds,
            message: first_line,
            message_full,
        });
    }

    Ok(commits)
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

    #[test]
    fn test_list_commits_default_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act
        let commits =
            list_commits(&repo_path, None, Some(10)).expect("Should list commits from HEAD");

        // Assert
        assert!(!commits.is_empty(), "Repository should have commits");
        assert!(commits.len() <= 10, "Should respect limit of 10 commits");
        assert_eq!(
            commits[0].oid().len(),
            40,
            "Full hash should be 40 characters"
        );
        assert_eq!(
            commits[0].short_oid().len(),
            7,
            "Short hash should be 7 characters"
        );
    }

    #[test]
    fn test_list_commits_with_branch() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let branch = "master";

        // Act
        let commits = list_commits(&repo_path, Some(branch), Some(5))
            .expect("Should list commits from master branch");

        // Assert
        assert!(!commits.is_empty(), "Branch should have commits");
        assert!(commits.len() <= 5, "Should respect limit of 5 commits");
        assert!(commits[0].date() > 0, "Should have valid timestamp");
        assert!(!commits[0].author().is_empty(), "Should have author name");
        assert!(
            !commits[0].author_email().is_empty(),
            "Should have author email"
        );
    }

    #[test]
    fn test_list_commits_invalid_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let invalid_ref = "refs/heads/nonexistent_branch_12345";

        // Act
        let result = list_commits(&repo_path, Some(invalid_ref), None);

        // Assert
        assert!(result.is_err(), "Should return error for invalid reference");
    }

    #[test]
    fn test_commit_info_message_parsing() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act
        let commits =
            list_commits(&repo_path, None, Some(1)).expect("Should retrieve at least one commit");

        // Assert
        assert!(!commits.is_empty(), "Should have at least one commit");
        let commit = &commits[0];
        assert!(
            !commit.message().contains('\n'),
            "First line should not contain newlines"
        );
        assert!(
            commit.message_full().len() >= commit.message().len(),
            "Full message should contain first line"
        );
    }

    #[test]
    fn test_commit_info_accessors() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act
        let commits = list_commits(&repo_path, None, Some(1)).expect("Should retrieve commit");

        // Assert
        let commit = &commits[0];
        assert!(!commit.oid().is_empty(), "Should have OID");
        assert!(!commit.short_oid().is_empty(), "Should have short OID");
        assert!(!commit.author().is_empty(), "Should have author");
        assert!(!commit.author_email().is_empty(), "Should have email");
        assert!(!commit.committer().is_empty(), "Should have committer");
        assert!(commit.date() > 0, "Should have positive timestamp");
        assert!(!commit.message().is_empty(), "Should have message");
        assert!(
            !commit.message_full().is_empty(),
            "Should have full message"
        );
    }

    #[test]
    fn test_list_commits_no_limit() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act
        let commits_limited =
            list_commits(&repo_path, None, Some(3)).expect("Should list limited commits");
        let commits_unlimited =
            list_commits(&repo_path, None, None).expect("Should list all commits without limit");

        // Assert
        assert!(
            !commits_unlimited.is_empty(),
            "Repository should have commits"
        );
        assert!(
            commits_unlimited.len() >= commits_limited.len(),
            "Unlimited query should return at least as many commits as limited query"
        );
    }

    #[test]
    fn test_list_commits_ordering() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act
        let commits = list_commits(&repo_path, None, Some(5)).expect("Should list commits");

        // Assert
        assert!(
            commits.len() >= 2,
            "Need at least 2 commits to test ordering"
        );
        for i in 0..commits.len() - 1 {
            assert!(
                commits[i].date() >= commits[i + 1].date(),
                "Commits should be in reverse chronological order (newest first)"
            );
        }
    }

    #[test]
    fn test_list_commits_limit_exceeds_total() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Get actual commit count first
        let all_commits = list_commits(&repo_path, None, None).expect("Should list all commits");
        let actual_count = all_commits.len();

        // Act: Request way more commits than exist
        let commits = list_commits(&repo_path, None, Some(actual_count * 10))
            .expect("Should list commits without error");

        // Assert
        assert_eq!(
            commits.len(),
            actual_count,
            "Should return all available commits when limit exceeds total"
        );
        assert!(
            commits.len() > 0,
            "Repository should have at least one commit"
        );
    }

    #[test]
    fn test_list_commits_zero_limit() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act: Request zero commits
        let commits =
            list_commits(&repo_path, None, Some(0)).expect("Should handle zero limit gracefully");

        // Assert
        assert_eq!(
            commits.len(),
            0,
            "Should return empty list when limit is zero"
        );
    }

    #[test]
    fn test_list_commits_no_limit_ordering() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act: Request all commits (no limit)
        let commits = list_commits(&repo_path, None, None).expect("Should list all commits");

        // Assert
        assert!(
            commits.len() > 0,
            "Repository should have at least one commit"
        );
        // Verify ordering: first commit should be most recent
        if commits.len() >= 2 {
            assert!(
                commits[0].date() >= commits[1].date(),
                "Commits should be in reverse chronological order"
            );
        }
    }

    #[test]
    fn test_list_files_invalid_repository_path() {
        // Arrange: Use a path that is definitely not a git repository
        let invalid_path = PathBuf::from("/tmp/definitely-not-a-git-repo-12345");

        // Act
        let result = list_files(&invalid_path, None);

        // Assert
        assert!(
            result.is_err(),
            "Should return error for invalid repository path"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Failed to open repository at"),
            "Error should mention failed repository opening"
        );
    }

    #[test]
    fn test_analyze_repository_invalid_path_name() {
        // Arrange: Use root path which has no valid file name
        let root_path = PathBuf::from("/");

        // Act
        let result = analyze_repository(&root_path, None);

        // Assert
        assert!(
            result.is_err(),
            "Should return error for path with no valid name"
        );
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Cannot determine repository name from path")
                || err_msg.contains("Failed to open repository"),
            "Error should mention repository name determination or opening failure"
        );
    }

    #[test]
    fn test_file_entry_oid_accessor() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Act: List files and access their OIDs
        let files = list_files(&repo_path, None).expect("Should list files");

        // Assert
        assert!(!files.is_empty(), "Should have at least one file");

        for file in &files {
            // Call the oid() method to ensure it's covered
            let oid = file.oid();
            assert_eq!(
                oid.as_bytes().len(),
                20,
                "Git OID should be 20 bytes (SHA-1)"
            );
        }
    }
}

#[cfg(test)]
impl CommitInfo {
    /// Creates a new CommitInfo instance for testing.
    ///
    /// This constructor is only available in test builds.
    /// Production code should use list_commits() to retrieve commit data.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_for_test(
        oid: String,
        short_oid: String,
        author: String,
        author_email: String,
        committer: String,
        date: i64,
        message: String,
        message_full: String,
    ) -> Self {
        Self {
            oid,
            short_oid,
            author,
            author_email,
            committer,
            date,
            message,
            message_full,
        }
    }
}
