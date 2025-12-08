//! In-memory directory tree for efficient file hierarchy queries.
//!
//! Provides O(depth) lookups for files and subdirectories at any level,
//! replacing O(n) linear scans through flat file lists.

use crate::FileEntry;
use std::collections::HashMap;

/// In-memory representation of repository directory tree.
///
/// Provides efficient queries for files and subdirectories at any level.
/// Replaces O(n) linear scans with O(depth) tree traversal.
///
/// # Performance
///
/// - Construction: O(n × depth) where n = total files
/// - Query files at level: O(depth)
/// - Query subdirs at level: O(depth + k log k) where k = subdirs
/// - List all directories: O(total_dirs)
///
/// # Examples
///
/// ```no_run
/// use gitkyl::{list_files, FileTree};
/// use std::path::Path;
///
/// let files = list_files(Path::new("."), None)?;
/// let tree = FileTree::from_files(files);
///
/// // O(1) lookup instead of O(n) scan
/// let root_files = tree.files_at("");
/// let src_subdirs = tree.subdirs_at("src");
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct FileTree {
    root: DirNode,
}

#[derive(Debug, Clone)]
struct DirNode {
    files: Vec<FileEntry>,
    subdirs: HashMap<String, DirNode>,
}

impl FileTree {
    /// Builds tree from flat file list in single pass.
    ///
    /// Constructs directory hierarchy by splitting file paths and inserting
    /// files into appropriate tree nodes. Creates intermediate directories
    /// as needed.
    ///
    /// # Arguments
    ///
    /// * `files`: Vector of file entries from repository
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitkyl::{list_files, FileTree};
    /// use std::path::Path;
    ///
    /// let files = list_files(Path::new("."), None)?;
    /// let tree = FileTree::from_files(files);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn from_files(files: Vec<FileEntry>) -> Self {
        let mut root = DirNode {
            files: Vec::new(),
            subdirs: HashMap::new(),
        };

        for file in files {
            if let Some(path) = file.path()
                && let Some(path_str) = path.to_str()
            {
                let components: Vec<&str> = path_str.split('/').collect();
                let mut current = &mut root;

                // Navigate or create directory structure
                for (i, &component) in components.iter().enumerate() {
                    if i == components.len() - 1 {
                        // Leaf node: file itself
                        current.files.push(file.clone());
                    } else {
                        // Intermediate directory
                        current =
                            current
                                .subdirs
                                .entry(component.to_string())
                                .or_insert_with(|| DirNode {
                                    files: Vec::new(),
                                    subdirs: HashMap::new(),
                                });
                    }
                }
            }
        }

        Self { root }
    }

    /// Returns files directly at the given directory level.
    ///
    /// Returns only files that are immediate children of the directory,
    /// not files in subdirectories.
    ///
    /// # Arguments
    ///
    /// * `dir_path`: Directory path (empty string for root)
    ///
    /// # Returns
    ///
    /// Vector of file entries at the specified level
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use gitkyl::{list_files, FileTree};
    /// # use std::path::Path;
    /// # let files = list_files(Path::new("."), None)?;
    /// let tree = FileTree::from_files(files);
    ///
    /// let root_files = tree.files_at("");
    /// let src_files = tree.files_at("src");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn files_at(&self, dir_path: &str) -> &[FileEntry] {
        self.get_node(dir_path)
            .map(|node| node.files.as_slice())
            .unwrap_or(&[])
    }

    /// Returns immediate subdirectories at the given level.
    ///
    /// Returns directory names only (not full paths), sorted alphabetically.
    ///
    /// # Arguments
    ///
    /// * `dir_path`: Directory path (empty string for root)
    ///
    /// # Returns
    ///
    /// Sorted vector of subdirectory names
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use gitkyl::{list_files, FileTree};
    /// # use std::path::Path;
    /// # let files = list_files(Path::new("."), None)?;
    /// let tree = FileTree::from_files(files);
    ///
    /// let root_subdirs = tree.subdirs_at("");
    /// assert!(root_subdirs.contains(&"src"));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn subdirs_at(&self, dir_path: &str) -> Vec<&str> {
        self.get_node(dir_path)
            .map(|node| {
                let mut subdirs: Vec<&str> = node.subdirs.keys().map(|s| s.as_str()).collect();
                subdirs.sort();
                subdirs
            })
            .unwrap_or_default()
    }

    /// Returns all directory paths in the tree.
    ///
    /// Returns owned strings because full paths must be constructed by
    /// traversing the tree structure. For zero-copy queries at a specific
    /// level, use `subdirs_at()` which returns borrowed directory names.
    ///
    /// Performs depth-first traversal to collect all directory paths,
    /// sorted alphabetically.
    ///
    /// # Returns
    ///
    /// Sorted vector of all directory paths (owned strings)
    ///
    /// # Performance
    ///
    /// Allocates O(total_dirs) strings. For read-heavy workloads where
    /// `all_dirs()` is called repeatedly, consider caching the result.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use gitkyl::{list_files, FileTree};
    /// # use std::path::Path;
    /// # let files = list_files(Path::new("."), None)?;
    /// let tree = FileTree::from_files(files);
    ///
    /// let all_dirs = tree.all_dirs();
    /// assert!(all_dirs.contains(&"src".to_string()));
    /// assert!(all_dirs.contains(&"src/generators".to_string()));
    ///
    /// // For zero-copy queries at a specific level, use subdirs_at
    /// let src_subdirs = tree.subdirs_at("src");  // Returns Vec<&str>
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn all_dirs(&self) -> Vec<String> {
        let mut dirs = Vec::new();
        Self::collect_dirs(&self.root, String::new(), &mut dirs);
        dirs.sort();
        dirs
    }

    /// Returns all files under a directory (including subdirectories).
    ///
    /// Recursively collects files from the directory and all its subdirectories.
    ///
    /// # Arguments
    ///
    /// * `dir_path`: Directory path (empty string for root)
    ///
    /// # Returns
    ///
    /// Vector of all file entries in the subtree
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use gitkyl::{list_files, FileTree};
    /// # use std::path::Path;
    /// # let files = list_files(Path::new("."), None)?;
    /// let tree = FileTree::from_files(files);
    ///
    /// let all_src_files = tree.all_files_under("src");
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    #[allow(dead_code)] // Reserved for future recursive operations (search, stats)
    pub fn all_files_under(&self, dir_path: &str) -> Vec<&FileEntry> {
        let mut files = Vec::new();
        if let Some(node) = self.get_node(dir_path) {
            Self::collect_files_borrowed(node, &mut files);
        }
        files
    }

    fn get_node(&self, dir_path: &str) -> Option<&DirNode> {
        if dir_path.is_empty() {
            return Some(&self.root);
        }

        let components: Vec<&str> = dir_path.split('/').collect();
        let mut current = &self.root;

        for component in components {
            current = current.subdirs.get(component)?;
        }

        Some(current)
    }

    fn collect_dirs(node: &DirNode, path: String, dirs: &mut Vec<String>) {
        dirs.push(path.clone());

        for (name, subdir) in &node.subdirs {
            let subpath = if path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", path, name)
            };
            Self::collect_dirs(subdir, subpath, dirs);
        }
    }

    fn collect_files_borrowed<'a>(node: &'a DirNode, files: &mut Vec<&'a FileEntry>) {
        files.extend(node.files.iter());
        for subdir in node.subdirs.values() {
            Self::collect_files_borrowed(subdir, files);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_filetree_from_empty_files() {
        // Arrange
        let files = vec![];

        // Act
        let tree = FileTree::from_files(files);

        // Assert
        assert!(tree.files_at("").is_empty(), "Root should have no files");
        assert!(
            tree.subdirs_at("").is_empty(),
            "Root should have no subdirs"
        );
        assert_eq!(
            tree.all_dirs(),
            vec![""],
            "Empty tree should still have root directory"
        );
    }

    #[test]
    fn test_filetree_files_at_root() {
        // Arrange: Use real repository
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);
        let root_files = tree.files_at("");

        // Assert
        let root_file_names: Vec<String> = root_files
            .iter()
            .filter_map(|f| f.path()?.file_name()?.to_str().map(String::from))
            .collect();

        assert!(
            root_file_names.contains(&"Cargo.toml".to_string()),
            "Should find Cargo.toml at root"
        );
    }

    #[test]
    fn test_filetree_files_at_subdir() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);
        let src_files = tree.files_at("src");

        // Assert
        assert!(!src_files.is_empty(), "src directory should have files");

        let src_file_names: Vec<String> = src_files
            .iter()
            .filter_map(|f| f.path()?.file_name()?.to_str().map(String::from))
            .collect();

        assert!(
            src_file_names.contains(&"main.rs".to_string())
                || src_file_names.contains(&"lib.rs".to_string()),
            "Should find Rust files in src"
        );
    }

    #[test]
    fn test_filetree_subdirs_at_root() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);
        let root_subdirs = tree.subdirs_at("");

        // Assert
        assert!(root_subdirs.contains(&"src"), "Root should have src subdir");
        assert!(
            root_subdirs.contains(&"tests") || root_subdirs.contains(&"assets"),
            "Root should have tests or assets subdir"
        );
    }

    #[test]
    fn test_filetree_subdirs_sorted() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);
        let subdirs = tree.subdirs_at("");

        // Assert: Check if sorted
        let mut sorted_subdirs = subdirs.clone();
        sorted_subdirs.sort();
        assert_eq!(
            subdirs, sorted_subdirs,
            "Subdirectories should be sorted alphabetically"
        );
    }

    #[test]
    fn test_filetree_all_dirs() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);
        let all_dirs = tree.all_dirs();

        // Assert
        assert!(!all_dirs.is_empty(), "Should have at least one directory");
        assert!(
            all_dirs.contains(&"src".to_string()) || all_dirs.contains(&"tests".to_string()),
            "Should include src or tests directory"
        );

        // Check sorted
        let mut sorted_dirs = all_dirs.clone();
        sorted_dirs.sort();
        assert_eq!(all_dirs, sorted_dirs, "All directories should be sorted");
    }

    #[test]
    fn test_filetree_all_files_under() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files.clone());
        let all_src_files = tree.all_files_under("src");

        // Assert
        assert!(!all_src_files.is_empty(), "src should contain files");

        // Verify all files are under src/
        for file in &all_src_files {
            if let Some(path) = file.path()
                && let Some(path_str) = path.to_str()
            {
                assert!(
                    path_str.starts_with("src/"),
                    "File should be under src/: {}",
                    path_str
                );
            }
        }
    }

    #[test]
    fn test_filetree_all_files_under_root() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files.clone());
        let all_files = tree.all_files_under("");

        // Assert
        assert_eq!(
            all_files.len(),
            files.len(),
            "Root should contain all files"
        );
    }

    #[test]
    fn test_filetree_nonexistent_directory() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);
        let files = tree.files_at("nonexistent/path");
        let subdirs = tree.subdirs_at("nonexistent/path");

        // Assert
        assert!(files.is_empty(), "Nonexistent dir should have no files");
        assert!(subdirs.is_empty(), "Nonexistent dir should have no subdirs");
    }

    #[test]
    fn test_filetree_nested_path() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);

        // Find a nested directory
        let all_dirs = tree.all_dirs();
        if let Some(nested_dir) = all_dirs.iter().find(|d| d.contains('/')) {
            let _files = tree.files_at(nested_dir);
            let _subdirs = tree.subdirs_at(nested_dir);
            // Assert: Should be able to query nested paths without panicking
        }
    }

    #[test]
    fn test_filetree_files_not_in_subdirs() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = crate::list_files(&repo_path, None).expect("Failed to list files");

        // Act
        let tree = FileTree::from_files(files);
        let root_files = tree.files_at("");

        // Assert: Root files should NOT include files from subdirectories
        for file in root_files {
            if let Some(path) = file.path()
                && let Some(path_str) = path.to_str()
            {
                assert!(
                    !path_str.contains('/'),
                    "Root files should not contain subdirectory paths: {}",
                    path_str
                );
            }
        }
    }
}
