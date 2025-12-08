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
    /// Creates a new CommitInfo instance.
    ///
    /// Primarily for testing and manual construction. Production code should
    /// use list_commits() or get_last_commits_batch() to retrieve commit data.
    ///
    /// # Arguments
    ///
    /// * `oid`: Full commit hash
    /// * `message`: First line of commit message
    /// * `message_full`: Full commit message (includes body)
    /// * `author`: Author name
    /// * `date`: Commit timestamp (Unix seconds)
    ///
    /// # Returns
    ///
    /// A new CommitInfo instance with derived fields (short OID, committer).
    pub fn new(
        oid: String,
        message: String,
        message_full: String,
        author: String,
        date: i64,
    ) -> Self {
        let short_oid = if oid.len() >= 7 {
            oid[..7].to_string()
        } else {
            oid.clone()
        };

        Self {
            oid,
            short_oid,
            author: author.clone(),
            author_email: String::new(),
            committer: author,
            date,
            message,
            message_full,
        }
    }

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

/// Represents an item in a directory tree view.
///
/// Distinguishes between regular files (git blobs) and directories (git trees)
/// with proper semantic representation and commit metadata.
#[derive(Debug, Clone)]
pub enum TreeItem {
    /// Regular file with its last modifying commit
    File {
        entry: FileEntry,
        commit: CommitInfo,
    },
    /// Directory with its most recent commit
    Directory {
        name: String,
        full_path: String,
        commit: CommitInfo,
    },
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
///         println!("{}", path.display());
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
        .filter(|entry| entry.mode.is_blob())
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

/// Extracts CommitInfo from gix commit object.
fn extract_commit_info(commit: &gix::Commit) -> Result<CommitInfo> {
    let author = commit.author().context("Failed to read author")?;
    let committer = commit.committer().context("Failed to read committer")?;
    let message_bytes = commit
        .message_raw()
        .context("Failed to read commit message")?;
    let message_full = message_bytes.to_str_lossy().to_string();
    let first_line = message_full.lines().next().unwrap_or("").to_string();

    Ok(CommitInfo {
        oid: commit.id.to_hex().to_string(),
        short_oid: commit.id.to_hex_with_len(7).to_string(),
        author: author.name.to_str_lossy().to_string(),
        author_email: author.email.to_str_lossy().to_string(),
        committer: committer.name.to_str_lossy().to_string(),
        date: author.time.seconds,
        message: first_line,
        message_full,
    })
}

/// Batch lookup last commits for multiple files in single history walk.
///
/// Performs single history walk to find most recent commit that modified each
/// file. Compares file OIDs between commit and parent trees to detect modifications.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Reference name (branch/tag/commit), defaults to HEAD if None
/// * `file_paths`: Slice of file paths to lookup
///
/// # Returns
///
/// HashMap mapping file paths to their last CommitInfo
///
/// # Errors
///
/// Returns error if repository access or commit traversal fails
///
/// # Performance
///
/// Complexity: O(m × n) where m = commits walked, n = files tracked
///
/// Single repository open and history walk with early exit when all files found
///
/// # Examples
///
/// ```no_run
/// use gitkyl::get_last_commits_batch;
/// use std::path::Path;
///
/// let paths = &["src/lib.rs", "src/git.rs", "Cargo.toml"];
/// let commits = get_last_commits_batch(Path::new("."), None, paths)?;
/// for (path, commit) in commits {
///     println!("{}: {}", path, commit.short_oid());
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_last_commits_batch(
    repo_path: impl AsRef<Path>,
    ref_name: Option<&str>,
    file_paths: &[&str],
) -> Result<std::collections::HashMap<String, CommitInfo>> {
    use std::collections::{HashMap, HashSet};

    if file_paths.is_empty() {
        return Ok(HashMap::new());
    }

    let repo = gix::open(repo_path.as_ref()).with_context(|| {
        format!(
            "Failed to open repository at {}",
            repo_path.as_ref().display()
        )
    })?;

    let commit = resolve_commit(&repo, ref_name)?;

    let mut results: HashMap<String, CommitInfo> = HashMap::with_capacity(file_paths.len());
    let mut remaining: HashSet<String> = file_paths.iter().map(|s| s.to_string()).collect();

    let walker = commit
        .ancestors()
        .all()
        .context("Failed to create commit ancestor iterator")?;

    for result in walker {
        if remaining.is_empty() {
            break;
        }

        let commit_info = result.context("Failed to traverse commit ancestor")?;
        let commit_obj = commit_info
            .object()
            .context("Failed to read commit object")?;

        let parent_ids: Vec<_> = commit_obj.parent_ids().collect();

        if parent_ids.is_empty() {
            // Initial commit contains all files added in this commit
            let commit_data = extract_commit_info(&commit_obj)?;

            let remaining_snapshot: Vec<String> = remaining.iter().cloned().collect();
            for file_path in remaining_snapshot {
                let mut tree = commit_obj.tree().context("Failed to read commit tree")?;
                if let Ok(Some(_)) = tree.peel_to_entry_by_path(&file_path) {
                    results.insert(file_path.clone(), commit_data.clone());
                    remaining.remove(&file_path);
                }
            }
            break;
        }

        // Process each parent to handle merge commits
        let commit_data = extract_commit_info(&commit_obj)?;

        for parent_id in parent_ids {
            if remaining.is_empty() {
                break;
            }

            let parent = repo
                .find_object(parent_id)
                .context("Failed to find parent object")?
                .try_into_commit()
                .map_err(|_| anyhow::anyhow!("Parent object is not a commit"))?;

            // Check each remaining file for modifications via OID comparison
            let remaining_snapshot: Vec<String> = remaining.iter().cloned().collect();
            for file_path in remaining_snapshot {
                let mut current_tree = commit_obj.tree().context("Failed to read commit tree")?;
                let mut parent_tree = parent.tree().context("Failed to read parent tree")?;

                let current_entry = match current_tree.peel_to_entry_by_path(&file_path) {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let parent_entry = match parent_tree.peel_to_entry_by_path(&file_path) {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                // File modified if OID differs or file was added
                let was_modified = match (current_entry, parent_entry) {
                    (Some(_), None) => true,
                    (Some(curr), Some(par)) => curr.oid() != par.oid(),
                    _ => false,
                };

                if was_modified {
                    results.insert(file_path.clone(), commit_data.clone());
                    remaining.remove(&file_path);
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn temp_repo() -> tempfile::TempDir {
        let td = tempfile::TempDir::with_prefix("gitkyl-test-").unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(td.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(td.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(td.path())
            .output()
            .unwrap();
        td
    }

    fn write_file(repo_path: &Path, path: &str, content: &str) {
        let file_path = repo_path.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(file_path, content).unwrap();
    }

    fn git_add(repo_path: &Path) {
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    fn git_commit(repo_path: &Path, message: &str) -> String {
        let output = std::process::Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_path)
            .output()
            .unwrap();
        assert!(output.status.success());

        let rev_parse = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        String::from_utf8(rev_parse.stdout)
            .unwrap()
            .trim()
            .to_string()
    }

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

    #[test]
    fn test_get_last_commits_batch_empty_input() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let empty_paths: &[&str] = &[];

        // Act
        let results = get_last_commits_batch(&repo_path, None, empty_paths)
            .expect("Should handle empty input");

        // Assert
        assert!(
            results.is_empty(),
            "Empty input should return empty results"
        );
    }

    #[test]
    fn test_get_last_commits_batch_single_file() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let paths = &["Cargo.toml"];

        // Act
        let results = get_last_commits_batch(&repo_path, None, paths).expect("Should find commit");

        // Assert
        assert_eq!(results.len(), 1, "Should find one commit");
        assert!(
            results.contains_key("Cargo.toml"),
            "Should contain Cargo.toml"
        );

        let commit = &results["Cargo.toml"];
        assert!(!commit.oid().is_empty(), "Should have commit OID");
        assert_eq!(commit.short_oid().len(), 7, "Short OID should be 7 chars");
        assert!(!commit.author().is_empty(), "Should have author");
        assert!(commit.date() > 0, "Should have positive timestamp");
    }

    #[test]
    fn test_get_last_commits_batch_multiple_files() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let paths = &["Cargo.toml", "src/lib.rs", "src/git.rs"];

        // Act
        let results = get_last_commits_batch(&repo_path, None, paths).expect("Should find commits");

        // Assert
        assert!(results.len() >= 2, "Should find commits for multiple files");

        for path in paths {
            if let Some(commit) = results.get(*path) {
                assert!(!commit.oid().is_empty(), "Commit should have OID");
                assert!(!commit.author().is_empty(), "Commit should have author");
                assert!(commit.date() > 0, "Commit should have positive timestamp");
                assert!(!commit.message().is_empty(), "Commit should have message");
            }
        }
    }

    #[test]
    fn test_get_last_commits_batch_nonexistent_files() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let paths = &["nonexistent_file_12345.txt", "another_missing_file.rs"];

        // Act
        let results =
            get_last_commits_batch(&repo_path, None, paths).expect("Should handle gracefully");

        // Assert
        assert!(
            results.is_empty() || results.len() < paths.len(),
            "Should not find commits for nonexistent files"
        );
    }

    #[test]
    fn test_get_last_commits_batch_mixed_existing_nonexistent() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let paths = &["Cargo.toml", "nonexistent_12345.txt", "src/lib.rs"];

        // Act
        let results =
            get_last_commits_batch(&repo_path, None, paths).expect("Should handle mixed files");

        // Assert
        assert!(
            results.contains_key("Cargo.toml"),
            "Should find existing Cargo.toml"
        );
        assert!(
            results.contains_key("src/lib.rs"),
            "Should find existing src/lib.rs"
        );
        assert!(
            !results.contains_key("nonexistent_12345.txt"),
            "Should not find nonexistent file"
        );
    }

    #[test]
    fn test_get_last_commits_batch_specific_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let paths = &["Cargo.toml"];

        // Act
        let results = get_last_commits_batch(&repo_path, Some("HEAD"), paths)
            .expect("Should work with specific reference");

        // Assert
        assert!(
            !results.is_empty(),
            "Should find commit with HEAD reference"
        );
    }

    #[test]
    fn test_get_last_commits_batch_invalid_ref() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let invalid_ref = "refs/heads/nonexistent_branch_12345";
        let paths = &["Cargo.toml"];

        // Act
        let result = get_last_commits_batch(&repo_path, Some(invalid_ref), paths);

        // Assert
        assert!(result.is_err(), "Should return error for invalid reference");
    }

    #[test]
    fn test_get_last_commits_batch_invalid_repo() {
        // Arrange
        let invalid_path = PathBuf::from("/tmp/definitely-not-a-repo-12345");
        let paths = &["file.txt"];

        // Act
        let result = get_last_commits_batch(&invalid_path, None, paths);

        // Assert
        assert!(
            result.is_err(),
            "Should return error for invalid repository"
        );
    }

    #[test]
    fn test_get_last_commits_batch_finds_commits() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let file_path = "Cargo.toml";

        // Act
        let batch_results = get_last_commits_batch(&repo_path, None, &[file_path])
            .expect("Should get batch results");

        // Assert
        assert!(
            batch_results.contains_key(file_path),
            "Batch should find commit for the file"
        );
        let batch_commit = &batch_results[file_path];
        assert!(!batch_commit.oid().is_empty(), "Should have valid OID");
        assert!(!batch_commit.author().is_empty(), "Should have author");
        assert!(batch_commit.date() > 0, "Should have positive date");
    }

    #[test]
    fn test_get_last_commits_batch_commit_info_fields() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let paths = &["Cargo.toml"];

        // Act
        let results = get_last_commits_batch(&repo_path, None, paths).expect("Should find commit");

        // Assert
        let commit = &results["Cargo.toml"];
        assert_eq!(commit.oid().len(), 40, "Full OID should be 40 chars");
        assert_eq!(commit.short_oid().len(), 7, "Short OID should be 7 chars");
        assert!(
            !commit.author_email().is_empty(),
            "Should have author email"
        );
        assert!(!commit.committer().is_empty(), "Should have committer");
        assert!(
            !commit.message().contains('\n'),
            "Message should be single line"
        );
        assert!(
            commit.message_full().len() >= commit.message().len(),
            "Full message should contain at least first line"
        );
    }

    #[test]
    fn test_get_last_commits_batch_octopus_merge() {
        let td = temp_repo();
        let repo_path = td.path();

        write_file(repo_path, "base.txt", "base");
        git_add(repo_path);
        git_commit(repo_path, "Initial commit");

        let base_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        let base_commit = String::from_utf8(base_commit.stdout)
            .unwrap()
            .trim()
            .to_string();

        std::process::Command::new("git")
            .args(["checkout", "-b", "branch1"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        write_file(repo_path, "branch1.txt", "content1");
        git_add(repo_path);
        git_commit(repo_path, "Add branch1.txt");

        std::process::Command::new("git")
            .args(["checkout", "-b", "branch2", &base_commit])
            .current_dir(repo_path)
            .output()
            .unwrap();
        write_file(repo_path, "branch2.txt", "content2");
        git_add(repo_path);
        git_commit(repo_path, "Add branch2.txt");

        std::process::Command::new("git")
            .args(["checkout", "-b", "branch3", &base_commit])
            .current_dir(repo_path)
            .output()
            .unwrap();
        write_file(repo_path, "branch3.txt", "content3");
        git_add(repo_path);
        git_commit(repo_path, "Add branch3.txt");

        std::process::Command::new("git")
            .args(["checkout", "master"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args([
                "merge",
                "--no-ff",
                "branch1",
                "branch2",
                "branch3",
                "-m",
                "Octopus merge",
            ])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let paths = vec!["branch1.txt", "branch2.txt", "branch3.txt"];
        let result = get_last_commits_batch(repo_path, None, &paths).unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.contains_key("branch1.txt"));
        assert!(result.contains_key("branch2.txt"));
        assert!(result.contains_key("branch3.txt"));

        assert_eq!(result["branch1.txt"].message(), "Octopus merge");
        assert_eq!(result["branch2.txt"].message(), "Octopus merge");
        assert_eq!(result["branch3.txt"].message(), "Octopus merge");
    }

    #[test]
    fn test_get_last_commits_batch_empty_merge_commit() {
        let td = temp_repo();
        let repo_path = td.path();

        write_file(repo_path, "file.txt", "original");
        git_add(repo_path);
        git_commit(repo_path, "Initial commit");

        let base_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        let base_commit = String::from_utf8(base_commit.stdout)
            .unwrap()
            .trim()
            .to_string();

        std::process::Command::new("git")
            .args(["checkout", "-b", "branch1"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        write_file(repo_path, "file.txt", "modified");
        git_add(repo_path);
        git_commit(repo_path, "Modify in branch1");

        std::process::Command::new("git")
            .args(["checkout", "-b", "branch2", &base_commit])
            .current_dir(repo_path)
            .output()
            .unwrap();
        write_file(repo_path, "file.txt", "modified");
        git_add(repo_path);
        git_commit(repo_path, "Modify in branch2");

        std::process::Command::new("git")
            .args(["checkout", "master"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args([
                "merge",
                "--no-ff",
                "branch1",
                "branch2",
                "-m",
                "Empty merge (no new changes)",
            ])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let paths = vec!["file.txt"];
        let result = get_last_commits_batch(repo_path, None, &paths).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.contains_key("file.txt"));
    }

    #[test]
    fn test_get_last_commits_batch_rename_in_merge() {
        let td = temp_repo();
        let repo_path = td.path();

        write_file(repo_path, "original.txt", "content");
        git_add(repo_path);
        git_commit(repo_path, "Initial commit");

        let base_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        let base_commit = String::from_utf8(base_commit.stdout)
            .unwrap()
            .trim()
            .to_string();

        std::process::Command::new("git")
            .args(["mv", "original.txt", "renamed.txt"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        git_add(repo_path);
        git_commit(repo_path, "Rename original to renamed");

        std::process::Command::new("git")
            .args(["checkout", "-b", "branch2", &base_commit])
            .current_dir(repo_path)
            .output()
            .unwrap();
        write_file(repo_path, "original.txt", "modified content");
        git_add(repo_path);
        git_commit(repo_path, "Modify original");

        std::process::Command::new("git")
            .args(["checkout", "master"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "merge",
                "--no-ff",
                "branch2",
                "-m",
                "Merge rename with modification",
            ])
            .current_dir(repo_path)
            .env("GIT_MERGE_AUTOEDIT", "no")
            .output()
            .ok();

        std::fs::remove_file(repo_path.join("original.txt")).ok();
        write_file(repo_path, "renamed.txt", "modified content");
        git_add(repo_path);
        std::process::Command::new("git")
            .args(["commit", "--no-edit"])
            .current_dir(repo_path)
            .output()
            .ok();

        let paths = vec!["renamed.txt"];
        let result = get_last_commits_batch(repo_path, None, &paths).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.contains_key("renamed.txt"));
    }

    #[test]
    fn test_get_last_commits_batch_mode_change_only() {
        let td = temp_repo();
        let repo_path = td.path();

        write_file(repo_path, "script.sh", "#!/bin/bash\necho hello");
        git_add(repo_path);
        git_commit(repo_path, "Initial commit");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let script_path = repo_path.join("script.sh");
            let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms).unwrap();

            git_add(repo_path);
            git_commit(repo_path, "Make script executable");

            let paths = vec!["script.sh"];
            let result = get_last_commits_batch(repo_path, None, &paths).unwrap();

            assert_eq!(result.len(), 1);
            assert!(result.contains_key("script.sh"));
            assert_eq!(result["script.sh"].message(), "Initial commit");
        }

        #[cfg(not(unix))]
        {
            let paths = vec!["script.sh"];
            let result = get_last_commits_batch(repo_path, None, &paths).unwrap();

            assert_eq!(result.len(), 1);
            assert!(result.contains_key("script.sh"));
            assert_eq!(result["script.sh"].message(), "Initial commit");
        }
    }
}
