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

/// Git tag with metadata.
#[derive(Debug, Clone)]
pub struct TagInfo {
    /// Tag name (e.g., "v1.0.0")
    pub name: String,
    /// Target commit OID
    pub target_oid: String,
    /// Short commit hash (7 characters)
    pub short_oid: String,
    /// Tag message (annotated tags only)
    pub message: Option<String>,
    /// Tagger name and email (annotated tags only)
    pub tagger: Option<String>,
    /// Tag creation date (Unix timestamp)
    pub date: Option<i64>,
}

impl TagInfo {
    /// Creates a new TagInfo instance.
    ///
    /// # Arguments
    ///
    /// * `name`: Tag name
    /// * `target_oid`: Full commit hash that this tag points to
    /// * `message`: Optional tag message (annotated tags)
    /// * `tagger`: Optional tagger identity (annotated tags)
    /// * `date`: Optional tag creation timestamp (annotated tags)
    ///
    /// # Returns
    ///
    /// A new TagInfo instance with derived short OID.
    pub fn new(
        name: String,
        target_oid: String,
        message: Option<String>,
        tagger: Option<String>,
        date: Option<i64>,
    ) -> Self {
        let short_oid = if target_oid.len() >= 7 {
            target_oid[..7].to_string()
        } else {
            target_oid.clone()
        };

        Self {
            name,
            target_oid,
            short_oid,
            message,
            tagger,
            date,
        }
    }
}

/// Paginated commits result with navigation metadata.
#[derive(Debug, Clone)]
pub struct PaginatedCommits {
    /// Commits for current page
    pub commits: Vec<CommitInfo>,
    /// Current page number (1-indexed)
    pub page: usize,
    /// Commits per page
    pub per_page: usize,
    /// Whether more commits exist after this page
    pub has_more: bool,
}

impl PaginatedCommits {
    /// Creates a new PaginatedCommits instance.
    ///
    /// # Arguments
    ///
    /// * `commits`: Commits for this page
    /// * `page`: Current page number (1-indexed)
    /// * `per_page`: Commits per page setting
    /// * `has_more`: Whether additional pages exist
    pub fn new(commits: Vec<CommitInfo>, page: usize, per_page: usize, has_more: bool) -> Self {
        Self {
            commits,
            page,
            per_page,
            has_more,
        }
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

/// Lists commits with pagination support.
///
/// Fetches commits for a specific page. Internally fetches one extra commit
/// to determine if more pages exist (has_more flag).
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Reference to start from (branch/tag, None for HEAD)
/// * `page`: Page number (1-indexed, must be >= 1)
/// * `per_page`: Commits per page (must be >= 1)
///
/// # Returns
///
/// PaginatedCommits with commits for requested page and navigation metadata.
///
/// # Errors
///
/// Returns error if:
/// - Page or per_page is zero
/// - Repository cannot be opened
/// - Reference cannot be resolved
/// - Commit history cannot be traversed
///
/// # Examples
///
/// ```no_run
/// use gitkyl::list_commits_paginated;
/// use std::path::Path;
///
/// let page1 = list_commits_paginated(Path::new("."), None, 1, 35)?;
/// println!("Page {}: {} commits", page1.page, page1.commits.len());
/// if page1.has_more {
///     let page2 = list_commits_paginated(Path::new("."), None, 2, 35)?;
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn list_commits_paginated(
    repo_path: impl AsRef<Path>,
    ref_name: Option<&str>,
    page: usize,
    per_page: usize,
) -> Result<PaginatedCommits> {
    // Validate pagination parameters
    if page == 0 {
        anyhow::bail!("Page number must be >= 1, got 0");
    }
    if per_page == 0 {
        anyhow::bail!("Per page count must be >= 1, got 0");
    }

    let repo = gix::open(repo_path.as_ref()).with_context(|| {
        format!(
            "Failed to open repository at {}",
            repo_path.as_ref().display()
        )
    })?;

    let commit = resolve_commit(&repo, ref_name)?;

    let skip = (page - 1) * per_page;
    // Fetch one extra to detect if more pages exist
    let fetch_count = per_page + 1;

    let mut commits = Vec::new();
    let walker = commit
        .ancestors()
        .all()
        .context("Failed to create commit ancestor iterator")?;

    for result in walker.skip(skip).take(fetch_count) {
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

    // Detect if more pages exist
    let has_more = commits.len() > per_page;

    // Take only per_page commits (not the extra one)
    commits.truncate(per_page);

    Ok(PaginatedCommits::new(commits, page, per_page, has_more))
}

/// Lists all tags in the repository with metadata.
///
/// Retrieves all tags from the repository, extracting both lightweight and
/// annotated tag information. For annotated tags, includes message, tagger,
/// and creation date. Tags are sorted by date (newest first) for annotated
/// tags, then by name for lightweight tags.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
///
/// # Returns
///
/// Vector of TagInfo structs with metadata for each tag
///
/// # Errors
///
/// Returns error if:
/// - Repository cannot be opened
/// - References cannot be read
/// - Tag objects cannot be resolved
///
/// # Examples
///
/// ```no_run
/// use gitkyl::list_tags;
/// use std::path::Path;
///
/// let tags = list_tags(Path::new("."))?;
/// for tag in tags {
///     println!("{}: {}", tag.name, tag.short_oid);
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn list_tags(repo_path: impl AsRef<Path>) -> Result<Vec<TagInfo>> {
    let repo = gix::open(repo_path.as_ref()).with_context(|| {
        format!(
            "Failed to open repository at {}",
            repo_path.as_ref().display()
        )
    })?;

    let references_platform = repo.references().context("Failed to read references")?;

    let references = references_platform
        .all()
        .context("Failed to get all references")?;

    let mut tags = Vec::new();

    for reference_result in references {
        let reference = match reference_result {
            Ok(r) => r,
            Err(_) => continue,
        };
        let ref_name = reference.name();

        if !ref_name.as_bstr().starts_with(b"refs/tags/") {
            continue;
        }

        let tag_name = ref_name
            .shorten()
            .to_str()
            .context("Tag name contains invalid UTF8")?
            .to_string();

        let peeled_id = reference
            .into_fully_peeled_id()
            .context("Failed to peel reference")?;

        let target_commit = peeled_id
            .object()
            .context("Failed to resolve peeled object")?
            .try_into_commit()
            .map_err(|_| anyhow::anyhow!("Tag does not point to a commit"))?;

        let target_oid = target_commit.id.to_hex().to_string();

        // Try to get annotated tag information
        let tag_ref_name = format!("refs/tags/{}", tag_name);
        let tag_ref = repo
            .find_reference(tag_ref_name.as_str())
            .context("Failed to find tag reference")?;

        let tag_object_id = tag_ref.id();
        let tag_object = repo
            .find_object(tag_object_id)
            .context("Failed to find tag object")?;

        // Check if this is an annotated tag
        let (message, tagger, date) = if let Ok(tag_obj) = tag_object.try_into_tag() {
            // Annotated tag: decode the tag data
            let decoded = tag_obj.decode().context("Failed to decode tag object")?;

            let tag_message = if decoded.message.is_empty() {
                None
            } else {
                Some(decoded.message.to_str_lossy().to_string())
            };

            let tagger_info = decoded
                .tagger
                .as_ref()
                .map(|t| format!("{} <{}>", t.name.to_str_lossy(), t.email.to_str_lossy()));

            let tag_date = decoded.tagger.as_ref().map(|t| t.time.seconds);

            (tag_message, tagger_info, tag_date)
        } else {
            // Lightweight tag: use commit date for sorting
            let commit_date = target_commit
                .committer()
                .ok()
                .map(|c| c.time.seconds);
            (None, None, commit_date)
        };

        tags.push(TagInfo::new(tag_name, target_oid, message, tagger, date));
    }

    // Sort by date for annotated tags, then by name
    tags.sort_by(|a, b| match (a.date, b.date) {
        (Some(date_a), Some(date_b)) => date_b.cmp(&date_a),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.name.cmp(&b.name),
    });

    Ok(tags)
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

    fn git_tag(repo_path: &Path, tag_name: &str) {
        std::process::Command::new("git")
            .args(["tag", tag_name])
            .current_dir(repo_path)
            .output()
            .expect("Failed to create git tag");
    }

    fn git_tag_annotated(repo_path: &Path, tag_name: &str, message: &str) {
        std::process::Command::new("git")
            .args(["tag", "-a", tag_name, "-m", message])
            .current_dir(repo_path)
            .output()
            .expect("Failed to create annotated git tag");
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

    #[test]
    fn test_list_commits_paginated_first_page() {
        // Arrange: create repo with 10 commits
        let td = temp_repo();
        for i in 1..=10 {
            write_file(
                td.path(),
                &format!("file{}.txt", i),
                &format!("content{}", i),
            );
            git_add(td.path());
            git_commit(td.path(), &format!("Commit {}", i));
        }

        // Act
        let result = list_commits_paginated(td.path(), None, 1, 5).expect("Should list first page");

        // Assert
        assert_eq!(result.page, 1);
        assert_eq!(result.per_page, 5);
        assert_eq!(result.commits.len(), 5, "First page should have 5 commits");
        assert!(result.has_more, "Should indicate more pages exist");
        assert!(
            result.commits[0].message.contains("Commit 10"),
            "Newest commit should be first"
        );
    }

    #[test]
    fn test_list_commits_paginated_middle_page() {
        // Arrange: create repo with 15 commits
        let td = temp_repo();
        for i in 1..=15 {
            write_file(
                td.path(),
                &format!("file{}.txt", i),
                &format!("content{}", i),
            );
            git_add(td.path());
            git_commit(td.path(), &format!("Commit {}", i));
        }

        // Act: page 2 of 3 with 5 per page
        let result =
            list_commits_paginated(td.path(), None, 2, 5).expect("Should list middle page");

        // Assert
        assert_eq!(result.page, 2);
        assert_eq!(result.commits.len(), 5);
        assert!(result.has_more, "Should have page 3");
    }

    #[test]
    fn test_list_commits_paginated_last_page() {
        // Arrange: create repo with 12 commits
        let td = temp_repo();
        for i in 1..=12 {
            write_file(
                td.path(),
                &format!("file{}.txt", i),
                &format!("content{}", i),
            );
            git_add(td.path());
            git_commit(td.path(), &format!("Commit {}", i));
        }

        // Act: page 3, last page with only 2 commits
        let result = list_commits_paginated(td.path(), None, 3, 5).expect("Should list last page");

        // Assert
        assert_eq!(result.page, 3);
        assert_eq!(result.commits.len(), 2, "Last page has only 2 commits");
        assert!(!result.has_more, "Should indicate no more pages");
    }

    #[test]
    fn test_list_commits_paginated_exact_boundary() {
        // Arrange: exactly 10 commits, 5 per page = 2 pages exactly
        let td = temp_repo();
        for i in 1..=10 {
            write_file(
                td.path(),
                &format!("file{}.txt", i),
                &format!("content{}", i),
            );
            git_add(td.path());
            git_commit(td.path(), &format!("Commit {}", i));
        }

        // Act
        let page2 = list_commits_paginated(td.path(), None, 2, 5).expect("Should list page 2");

        // Assert
        assert_eq!(page2.commits.len(), 5);
        assert!(!page2.has_more, "Exactly at boundary, no more pages");
    }

    #[test]
    fn test_list_commits_paginated_invalid_page_zero() {
        // Arrange
        let td = temp_repo();
        write_file(td.path(), "test.txt", "content");
        git_add(td.path());
        git_commit(td.path(), "Initial commit");

        // Act
        let result = list_commits_paginated(td.path(), None, 0, 35);

        // Assert
        assert!(result.is_err(), "Page 0 should be invalid");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Page number must be >= 1"),
            "Error should mention page constraint"
        );
    }

    #[test]
    fn test_list_commits_paginated_beyond_last_page() {
        // Arrange: only 3 commits
        let td = temp_repo();
        for i in 1..=3 {
            write_file(
                td.path(),
                &format!("file{}.txt", i),
                &format!("content{}", i),
            );
            git_add(td.path());
            git_commit(td.path(), &format!("Commit {}", i));
        }

        // Act: request page 2 with 5 per page, but only 3 commits exist
        let result =
            list_commits_paginated(td.path(), None, 2, 5).expect("Should handle beyond last page");

        // Assert
        assert_eq!(result.commits.len(), 0, "Beyond last page returns empty");
        assert!(!result.has_more);
    }

    #[test]
    fn test_list_tags_empty_repo() {
        // Arrange: create repo with commit but no tags
        let td = temp_repo();
        write_file(td.path(), "test.txt", "content");
        git_add(td.path());
        git_commit(td.path(), "Initial commit");

        // Act
        let tags = list_tags(td.path()).expect("Should list tags");

        // Assert
        assert!(tags.is_empty(), "New repo should have no tags");
    }

    #[test]
    fn test_list_tags_lightweight() {
        // Arrange: create repo with lightweight tag
        let td = temp_repo();
        write_file(td.path(), "test.txt", "content");
        git_add(td.path());
        git_commit(td.path(), "Initial commit");
        git_tag(td.path(), "v1.0.0");

        // Act
        let tags = list_tags(td.path()).expect("Should list tags");

        // Assert
        assert_eq!(tags.len(), 1, "Should have one tag");
        let tag = &tags[0];
        assert_eq!(tag.name, "v1.0.0", "Tag name should match");
        assert!(!tag.target_oid.is_empty(), "Should have target OID");
        assert_eq!(tag.short_oid.len(), 7, "Short OID should be 7 chars");
        assert!(tag.message.is_none(), "Lightweight tag has no message");
        assert!(tag.tagger.is_none(), "Lightweight tag has no tagger");
        assert!(tag.date.is_none(), "Lightweight tag has no date");
    }

    #[test]
    fn test_list_tags_annotated() {
        // Arrange: create repo with annotated tag
        let td = temp_repo();
        write_file(td.path(), "test.txt", "content");
        git_add(td.path());
        git_commit(td.path(), "Initial commit");
        git_tag_annotated(td.path(), "v2.0.0", "Release version 2.0.0");

        // Act
        let tags = list_tags(td.path()).expect("Should list tags");

        // Assert
        assert_eq!(tags.len(), 1, "Should have one tag");
        let tag = &tags[0];
        assert_eq!(tag.name, "v2.0.0", "Tag name should match");
        assert!(!tag.target_oid.is_empty(), "Should have target OID");
        assert_eq!(tag.short_oid.len(), 7, "Short OID should be 7 chars");
        assert!(tag.message.is_some(), "Annotated tag should have message");
        assert_eq!(
            tag.message.as_ref().unwrap().trim(),
            "Release version 2.0.0",
            "Message should match"
        );
        assert!(tag.tagger.is_some(), "Annotated tag should have tagger");
        assert!(
            tag.tagger
                .as_ref()
                .unwrap()
                .contains("Test User <test@example.com>"),
            "Tagger should match test user"
        );
        assert!(tag.date.is_some(), "Annotated tag should have date");
        assert!(tag.date.unwrap() > 0, "Date should be positive timestamp");
    }

    #[test]
    fn test_list_tags_sorted_by_date() {
        // Arrange: create repo with multiple tags at different times
        let td = temp_repo();
        write_file(td.path(), "test1.txt", "content1");
        git_add(td.path());
        git_commit(td.path(), "First commit");
        git_tag_annotated(td.path(), "v1.0.0", "First release");

        std::thread::sleep(std::time::Duration::from_secs(1));

        write_file(td.path(), "test2.txt", "content2");
        git_add(td.path());
        git_commit(td.path(), "Second commit");
        git_tag_annotated(td.path(), "v2.0.0", "Second release");

        std::thread::sleep(std::time::Duration::from_secs(1));

        write_file(td.path(), "test3.txt", "content3");
        git_add(td.path());
        git_commit(td.path(), "Third commit");
        git_tag_annotated(td.path(), "v3.0.0", "Third release");

        // Act
        let tags = list_tags(td.path()).expect("Should list tags");

        // Assert
        assert_eq!(tags.len(), 3, "Should have three tags");
        assert_eq!(tags[0].name, "v3.0.0", "Newest tag should be first");
        assert_eq!(tags[1].name, "v2.0.0", "Second tag should be second");
        assert_eq!(tags[2].name, "v1.0.0", "Oldest tag should be last");

        for i in 0..tags.len() - 1 {
            let current_date = tags[i].date.expect("Should have date");
            let next_date = tags[i + 1].date.expect("Should have date");
            assert!(
                current_date >= next_date,
                "Tags should be sorted by date (newest first)"
            );
        }
    }
}
