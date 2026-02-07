//! Tree page generation for directory listings.

use anyhow::{Context, Result};
use gitkyl::pages::index::{IndexPageData, generate as index_page};
use gitkyl::{CommitInfo, Config, FileEntry, FileTree, TreeItem};
use std::collections::HashMap;
use std::fs;

/// Default limit for commits displayed on commit log page.
const DEFAULT_COMMIT_LIMIT: usize = 35;

/// Validates tree path to prevent directory traversal attacks.
fn validate_tree_path(path: &str) -> Result<()> {
    if path.contains("..") {
        anyhow::bail!("Path contains directory traversal: {}", path);
    }
    if path.starts_with('/') {
        anyhow::bail!("Path is absolute, must be relative: {}", path);
    }
    Ok(())
}

/// Builds tree items from file entries and subdirectories.
///
/// Combines directory and file entries into a unified list of tree items,
/// each annotated with last commit information from pre-fetched maps.
///
/// # Arguments
///
/// * `file_entries`: File entries at current level
/// * `subdir_names`: Subdirectory names at current level
/// * `dir_path`: Current directory path for constructing full paths
/// * `file_commit_map`: Mapping of file paths to last commits
/// * `dir_commit_map`: Mapping of directory paths to last commits
///
/// # Returns
///
/// Vector of tree items combining directories and files with commits
pub fn build_tree_items(
    file_entries: &[FileEntry],
    subdir_names: &[&str],
    dir_path: &str,
    file_commit_map: &HashMap<String, CommitInfo>,
    dir_commit_map: &HashMap<String, CommitInfo>,
) -> Vec<TreeItem> {
    let mut items = Vec::new();

    // Build directory items with pre-fetched commits
    for subdir in subdir_names {
        let full_path = if dir_path.is_empty() {
            subdir.to_string()
        } else {
            format!("{}/{}", dir_path, subdir)
        };

        if let Some(commit) = dir_commit_map.get(&full_path) {
            items.push(TreeItem::Directory {
                name: subdir.to_string(),
                full_path,
                commit: commit.clone(),
            });
        } else {
            eprintln!("Warning: No commit found for directory {}", full_path);
        }
    }

    // Build file items with pre-fetched commits
    for entry in file_entries {
        if let Some(path) = entry.path()
            && let Some(path_str) = path.to_str()
        {
            if let Some(commit) = file_commit_map.get(path_str) {
                items.push(TreeItem::File {
                    entry: entry.clone(),
                    commit: commit.clone(),
                });
            } else {
                eprintln!("Warning: No commit found for file {}", path_str);
            }
        }
    }

    items
}

/// Generates tree pages for all directories in a branch.
///
/// Creates index pages for the repository root and tree pages for all
/// subdirectories within the specified branch. Each page displays directory
/// listings with file metadata and last commit information.
///
/// # Arguments
///
/// * `config`: Application configuration containing output paths
/// * `repo_info`: Repository metadata including name and branches
/// * `branch`: Branch name to generate tree pages for
/// * `tree`: File tree structure for the branch
/// * `file_commit_map`: Pre-fetched mapping of file paths to last commits
///
/// # Returns
///
/// Count of tree pages generated
///
/// # Errors
///
/// Returns error if page generation or file writing fails
pub fn generate_tree_pages_for_branch(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    tree: &FileTree,
    file_commit_map: &HashMap<String, CommitInfo>,
) -> Result<usize> {
    let directories = tree.all_dirs();
    let mut count = 0;

    let commits = gitkyl::list_commits(&config.repo, Some(branch), Some(DEFAULT_COMMIT_LIMIT))
        .unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to list commits for branch {}: {:#}",
                branch, e
            );
            vec![]
        });

    let latest_commit = commits.first();

    for dir_path in directories {
        validate_tree_path(&dir_path)
            .with_context(|| format!("Invalid tree path: {}", dir_path))?;

        let entries_at_this_level = tree.files_at(&dir_path);
        let subdirs_at_this_level = tree.subdirs_at(&dir_path);

        let full_dir_paths: Vec<String> = subdirs_at_this_level
            .iter()
            .map(|subdir| {
                if dir_path.is_empty() {
                    subdir.to_string()
                } else {
                    format!("{}/{}", dir_path, subdir)
                }
            })
            .collect();

        let dir_path_refs: Vec<&str> = full_dir_paths.iter().map(|s| s.as_str()).collect();

        let level_dir_commit_map = if !dir_path_refs.is_empty() {
            gitkyl::get_last_commits_batch(&config.repo, Some(branch), &dir_path_refs)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Warning: Failed to batch lookup directory commits for {}: {:#}",
                        dir_path, e
                    );
                    HashMap::new()
                })
        } else {
            HashMap::new()
        };

        let tree_items_for_page = build_tree_items(
            entries_at_this_level,
            &subdirs_at_this_level,
            &dir_path,
            file_commit_map,
            &level_dir_commit_map,
        );

        let html_result = if dir_path.is_empty() {
            let depth = branch.matches('/').count() + 2;
            let readme_html = gitkyl::pages::index::find_and_render_readme(
                &config.repo,
                branch,
                &tree_items_for_page,
                depth,
            )
            .ok()
            .flatten();

            Ok(index_page(IndexPageData {
                name: repo_info.name(),
                owner: repo_info.owner(),
                default_branch: branch,
                branches: repo_info.branches(),
                commit_count: commits.len(),
                tag_count: 0,
                latest_commit,
                items: &tree_items_for_page,
                readme_html: readme_html.as_deref(),
                depth,
            }))
        } else {
            gitkyl::pages::tree::generate(
                &config.repo,
                branch,
                &dir_path,
                repo_info.name(),
                &tree_items_for_page,
            )
        };

        match html_result {
            Ok(html) => {
                let tree_path = if dir_path.is_empty() {
                    config.output.join("tree").join(branch).join("index.html")
                } else {
                    config
                        .output
                        .join("tree")
                        .join(branch)
                        .join(format!("{}.html", dir_path))
                };

                if let Some(parent) = tree_path.parent() {
                    fs::create_dir_all(parent).context("Failed to create tree directory")?;
                }

                fs::write(&tree_path, html.into_string()).with_context(|| {
                    format!("Failed to write tree page {}", tree_path.display())
                })?;

                count += 1;
            }
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Failed to generate tree page for {}", dir_path));
            }
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tree_path_valid() {
        // Arrange: Test valid paths
        let valid_paths = vec!["src", "src/main.rs", "docs/README.md", "a/b/c/d"];

        // Act & Assert: All should pass validation
        for path in valid_paths {
            let result = validate_tree_path(path);
            assert!(
                result.is_ok(),
                "Path '{}' should be valid but got error: {:?}",
                path,
                result.err()
            );
        }
    }

    #[test]
    fn test_validate_tree_path_traversal() {
        // Arrange: Test path traversal attempts
        let invalid_paths = vec![
            "../etc/passwd",
            "src/../../../etc/passwd",
            "foo/bar/../../../baz",
            "/absolute/path",
            "/etc/passwd",
        ];

        // Act & Assert: All should fail validation
        for path in invalid_paths {
            let result = validate_tree_path(path);
            assert!(
                result.is_err(),
                "Path '{}' should be invalid but passed validation",
                path
            );
        }
    }

    #[test]
    fn test_validate_tree_path_absolute() {
        // Arrange: Test absolute path rejection
        let absolute_paths = vec!["/", "/usr", "/home/user"];

        // Act & Assert
        for path in absolute_paths {
            let result = validate_tree_path(path);
            assert!(
                result.is_err(),
                "Absolute path '{}' should be rejected",
                path
            );
            assert!(
                result.unwrap_err().to_string().contains("absolute"),
                "Error should mention absolute path"
            );
        }
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_validate_tree_path_url_encoded_traversal() {
        assert!(validate_tree_path("%2e%2e/etc/passwd").is_err());
        assert!(validate_tree_path("src/%2e%2e/root").is_err());
        assert!(validate_tree_path("..%2Fetc").is_err());
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_validate_tree_path_unicode_separators() {
        assert!(validate_tree_path("\u{FF0F}etc\u{FF0F}passwd").is_err());
        assert!(validate_tree_path("src\u{FF0F}..").is_err());
        assert!(validate_tree_path("\u{2044}root").is_err());
    }

    #[test]
    fn test_validate_tree_path_unicode_bidi() {
        assert!(validate_tree_path("\u{202E}../etc/passwd").is_err());
        assert!(validate_tree_path("src\u{202E}/../root").is_err());
        assert!(validate_tree_path("\u{202D}..").is_err());
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_validate_tree_path_null_byte_injection() {
        assert!(validate_tree_path("etc\0/passwd").is_err());
        assert!(validate_tree_path("valid\0..").is_err());
        assert!(validate_tree_path("\0../root").is_err());
    }

    #[test]
    fn test_validate_tree_path_windows_separators() {
        assert!(validate_tree_path("..\\windows").is_err());
        assert!(validate_tree_path("src\\..\\..\\etc").is_err());
        assert!(validate_tree_path("path\\to\\..\\..\\sensitive").is_err());
        assert!(validate_tree_path("..\\..\\system32").is_err());
    }

    #[test]
    fn test_validate_tree_path_canonicalization_bypass() {
        assert!(validate_tree_path("/./../../etc/passwd").is_err());
        assert!(validate_tree_path("src/../../../etc").is_err());
        assert!(validate_tree_path("./../../etc/passwd").is_err());
        assert!(validate_tree_path("foo/./../../../bar").is_err());
        assert!(validate_tree_path("src/./../../sensitive").is_err());
    }

    #[test]
    fn test_build_tree_items_empty() {
        // Arrange: empty inputs
        let file_entries = vec![];
        let subdir_names = vec![];
        let dir_path = "";
        let file_commit_map = HashMap::new();
        let dir_commit_map = HashMap::new();

        // Act: build tree items
        let items = build_tree_items(
            &file_entries,
            &subdir_names,
            dir_path,
            &file_commit_map,
            &dir_commit_map,
        );

        // Assert: should return empty vector
        assert_eq!(items.len(), 0, "Expected empty tree items");
    }

    #[test]
    fn test_build_tree_items_with_dirs() {
        // Arrange: directories only
        let file_entries = vec![];
        let subdir_names = vec!["src", "docs"];
        let dir_path = "";
        let file_commit_map = HashMap::new();

        let mut dir_commit_map = HashMap::new();
        dir_commit_map.insert(
            "src".to_string(),
            CommitInfo::new(
                "abc123".to_string(),
                "Initial commit".to_string(),
                "Initial commit\n\nFull message.".to_string(),
                "Test Author".to_string(),
                1704067200,
            ),
        );
        dir_commit_map.insert(
            "docs".to_string(),
            CommitInfo::new(
                "def456".to_string(),
                "Add docs".to_string(),
                "Add docs\n\nFull message.".to_string(),
                "Test Author".to_string(),
                1704153600,
            ),
        );

        // Act: build tree items
        let items = build_tree_items(
            &file_entries,
            &subdir_names,
            dir_path,
            &file_commit_map,
            &dir_commit_map,
        );

        // Assert: should have two directory items
        assert_eq!(items.len(), 2, "Expected 2 tree items");

        match &items[0] {
            TreeItem::Directory {
                name,
                full_path,
                commit,
            } => {
                assert_eq!(name, "src");
                assert_eq!(full_path, "src");
                assert_eq!(commit.short_oid(), "abc123");
            }
            _ => panic!("Expected directory item"),
        }

        match &items[1] {
            TreeItem::Directory {
                name,
                full_path,
                commit,
            } => {
                assert_eq!(name, "docs");
                assert_eq!(full_path, "docs");
                assert_eq!(commit.short_oid(), "def456");
            }
            _ => panic!("Expected directory item"),
        }
    }

    #[test]
    fn test_build_tree_items_nested_path() {
        // Arrange: nested directory path
        let file_entries = vec![];
        let subdir_names = vec!["utils"];
        let dir_path = "src/lib";
        let file_commit_map = HashMap::new();

        let mut dir_commit_map = HashMap::new();
        dir_commit_map.insert(
            "src/lib/utils".to_string(),
            CommitInfo::new(
                "nested123".to_string(),
                "Add utils".to_string(),
                "Add utils\n\nFull message.".to_string(),
                "Test Author".to_string(),
                1704240000,
            ),
        );

        // Act: build tree items
        let items = build_tree_items(
            &file_entries,
            &subdir_names,
            dir_path,
            &file_commit_map,
            &dir_commit_map,
        );

        // Assert: full path should be constructed correctly
        assert_eq!(items.len(), 1, "Expected 1 tree item");

        match &items[0] {
            TreeItem::Directory {
                name,
                full_path,
                commit,
            } => {
                assert_eq!(name, "utils");
                assert_eq!(full_path, "src/lib/utils");
                assert_eq!(commit.short_oid(), "nested1");
            }
            _ => panic!("Expected directory item"),
        }
    }
}
