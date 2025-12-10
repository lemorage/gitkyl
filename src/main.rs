use anyhow::{Context, Result};
use gitkyl::pages::index::{IndexPageData, find_and_render_readme, generate as index_page};
use gitkyl::{Config, TreeItem};
use std::fs;

/// Default limit for commits displayed on commit log page.
///
/// Limits display to 35 commits to balance page load time and commit
/// visibility. Repositories with extensive history should implement
/// pagination in future versions.
const DEFAULT_COMMIT_LIMIT: usize = 35;

fn validate_tree_path(path: &str) -> Result<()> {
    if path.contains("..") {
        anyhow::bail!("Path contains directory traversal: {}", path);
    }
    if path.starts_with('/') {
        anyhow::bail!("Path is absolute, must be relative: {}", path);
    }
    Ok(())
}

/// Creates output directory structure and writes CSS assets.
///
/// Sets up required directories (output root, assets, tree, blob, commits)
/// and writes all CSS bundles to assets directory.
///
/// # Arguments
///
/// * `output_dir`: Base output directory path
///
/// # Errors
///
/// Returns error if directory creation fails or CSS writing fails
fn setup_output_directories(output_dir: &std::path::Path) -> Result<()> {
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let assets_dir = output_dir.join("assets");
    fs::create_dir_all(&assets_dir).context("Failed to create assets directory")?;

    gitkyl::write_css_assets(&assets_dir).context("Failed to write CSS assets")?;

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
fn build_tree_items(
    file_entries: &[gitkyl::FileEntry],
    subdir_names: &[&str],
    dir_path: &str,
    file_commit_map: &std::collections::HashMap<String, gitkyl::CommitInfo>,
    dir_commit_map: &std::collections::HashMap<String, gitkyl::CommitInfo>,
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

fn main() -> Result<()> {
    let config = Config::parse();
    config.validate().context("Invalid configuration")?;

    let repo_info = gitkyl::analyze_repository(&config.repo, config.owner.clone())
        .context("Failed to analyze repository")?;

    setup_output_directories(&config.output)?;

    let latest_commit =
        gitkyl::list_commits(&config.repo, Some(repo_info.default_branch()), Some(1))
            .ok()
            .and_then(|commits| commits.into_iter().next());

    let files =
        gitkyl::list_files(&config.repo, Some(repo_info.default_branch())).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to list files: {:#}", e);
            vec![]
        });

    // Build tree structure once for O(depth) queries
    let tree = gitkyl::FileTree::from_files(files.clone());

    // Batch lookup commits for ALL files once (reused for root and tree pages)
    let all_file_paths: Vec<&str> = files.iter().filter_map(|f| f.path()?.to_str()).collect();

    let commit_map = gitkyl::get_last_commits_batch(
        &config.repo,
        Some(repo_info.default_branch()),
        &all_file_paths,
    )
    .unwrap_or_else(|e| {
        eprintln!("Warning: Failed to batch lookup commits: {:#}", e);
        std::collections::HashMap::new()
    });

    let top_level_files = tree.files_at("");
    let top_level_subdirs = tree.subdirs_at("");

    // Batch lookup for root level directories
    let root_dir_commit_map = if !top_level_subdirs.is_empty() {
        gitkyl::get_last_commits_batch(
            &config.repo,
            Some(repo_info.default_branch()),
            &top_level_subdirs,
        )
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to batch lookup directory commits: {:#}", e);
            std::collections::HashMap::new()
        })
    } else {
        std::collections::HashMap::new()
    };

    let tree_items = build_tree_items(
        top_level_files,
        &top_level_subdirs,
        "",
        &commit_map,
        &root_dir_commit_map,
    );

    // Detect and render README file at repository root
    let readme_html = find_and_render_readme(&config.repo, repo_info.default_branch(), &tree_items)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to render README: {:#}", e);
            None
        });

    let html = index_page(IndexPageData {
        name: &config
            .project_name()
            .context("Failed to determine project name")?,
        owner: repo_info.owner(),
        default_branch: repo_info.default_branch(),
        branches: repo_info.branches(),
        commit_count: repo_info.commit_count(),
        latest_commit: latest_commit.as_ref(),
        items: &tree_items,
        readme_html: readme_html.as_deref(),
        depth: 0,
    });

    let index_path = config.output.join("index.html");
    fs::write(&index_path, html.into_string())
        .with_context(|| format!("Failed to write index page to {}", index_path.display()))?;

    println!("Generated: {}", index_path.display());

    let commits = gitkyl::list_commits(
        &config.repo,
        Some(repo_info.default_branch()),
        Some(DEFAULT_COMMIT_LIMIT),
    )
    .context("Failed to list commits")?;

    let commits_html =
        gitkyl::pages::commits::generate(&commits, repo_info.default_branch(), repo_info.name());

    let commits_dir = config
        .output
        .join("commits")
        .join(repo_info.default_branch());

    fs::create_dir_all(&commits_dir).context("Failed to create commits directory")?;

    let commits_path = commits_dir.join("index.html");
    fs::write(&commits_path, commits_html.into_string())
        .with_context(|| format!("Failed to write commits page to {}", commits_path.display()))?;

    println!(
        "Generated: {} ({} commits)",
        commits_path.display(),
        commits.len()
    );

    println!("Generating file pages...");

    let mut generated_count = 0;
    let mut markdown_count = 0;
    for entry in &files {
        if let Some(path) = entry.path() {
            if path.to_str().is_none() {
                eprintln!(
                    "Warning: Skipping file with invalid UTF-8 path: {}",
                    path.display()
                );
                continue;
            }

            // Detect README files for markdown rendering
            let result = if gitkyl::is_readme(path) {
                markdown_count += 1;
                gitkyl::pages::blob::generate_markdown(
                    &config.repo,
                    repo_info.default_branch(),
                    path,
                    &config
                        .project_name()
                        .context("Failed to determine project name")?,
                )
            } else {
                gitkyl::pages::blob::generate(
                    &config.repo,
                    repo_info.default_branch(),
                    path,
                    &config
                        .project_name()
                        .context("Failed to determine project name")?,
                    &config.theme,
                )
            };

            match result {
                Ok(html) => {
                    let blob_path = config
                        .output
                        .join("blob")
                        .join(repo_info.default_branch())
                        .join(format!("{}.html", path.display()));

                    if let Some(parent) = blob_path.parent() {
                        fs::create_dir_all(parent).context("Failed to create blob directory")?;
                    }

                    fs::write(&blob_path, html.into_string()).with_context(|| {
                        format!("Failed to write blob page {}", blob_path.display())
                    })?;

                    generated_count += 1;
                }
                Err(e) => {
                    let err_msg = format!("{:?}", e);
                    if err_msg.contains("not a blob") || err_msg.contains("invalid UTF8") {
                        continue;
                    }
                    return Err(e).with_context(|| {
                        format!("Failed to generate blob page for {}", path.display())
                    });
                }
            }
        }
    }

    println!(
        "Generated {} file pages ({} markdown, {} code)",
        generated_count,
        markdown_count,
        generated_count - markdown_count
    );

    println!("Generating tree pages...");

    let directories = tree.all_dirs();
    let mut tree_count = 0;

    for dir_path in directories {
        validate_tree_path(&dir_path)
            .with_context(|| format!("Invalid tree path: {}", dir_path))?;

        let entries_at_this_level = tree.files_at(&dir_path);
        let subdirs_at_this_level = tree.subdirs_at(&dir_path);

        // Build full subdir paths for directory commit lookup
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
            gitkyl::get_last_commits_batch(
                &config.repo,
                Some(repo_info.default_branch()),
                &dir_path_refs,
            )
            .unwrap_or_else(|e| {
                eprintln!(
                    "Warning: Failed to batch lookup directory commits for {}: {:#}",
                    dir_path, e
                );
                std::collections::HashMap::new()
            })
        } else {
            std::collections::HashMap::new()
        };

        let tree_items_for_page = build_tree_items(
            entries_at_this_level,
            &subdirs_at_this_level,
            &dir_path,
            &commit_map,
            &level_dir_commit_map,
        );

        let html_result = if dir_path.is_empty() {
            let readme_html = gitkyl::pages::index::find_and_render_readme(
                &config.repo,
                repo_info.default_branch(),
                &tree_items_for_page,
            )
            .ok()
            .flatten();

            let depth = repo_info.default_branch().matches('/').count() + 2;
            Ok(gitkyl::pages::index::generate(IndexPageData {
                name: repo_info.name(),
                owner: repo_info.owner(),
                default_branch: repo_info.default_branch(),
                branches: repo_info.branches(),
                commit_count: commits.len(),
                latest_commit: latest_commit.as_ref(),
                items: &tree_items_for_page,
                readme_html: readme_html.as_deref(),
                depth,
            }))
        } else {
            gitkyl::pages::tree::generate(
                &config.repo,
                repo_info.default_branch(),
                &dir_path,
                repo_info.name(),
                &tree_items_for_page,
            )
        };

        match html_result {
            Ok(html) => {
                let tree_path = if dir_path.is_empty() {
                    config
                        .output
                        .join("tree")
                        .join(repo_info.default_branch())
                        .join("index.html")
                } else {
                    config
                        .output
                        .join("tree")
                        .join(repo_info.default_branch())
                        .join(format!("{}.html", dir_path))
                };

                if let Some(parent) = tree_path.parent() {
                    fs::create_dir_all(parent).context("Failed to create tree directory")?;
                }

                fs::write(&tree_path, html.into_string()).with_context(|| {
                    format!("Failed to write tree page {}", tree_path.display())
                })?;

                tree_count += 1;
            }
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Failed to generate tree page for {}", dir_path));
            }
        }
    }

    println!("Generated {} tree pages for default branch", tree_count);

    println!("Generating tree pages for all branches...");

    let mut total_tree_pages = tree_count;

    for branch in repo_info.branches() {
        if branch == repo_info.default_branch() {
            continue;
        }

        let branch_files = gitkyl::list_files(&config.repo, Some(branch)).unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to list files for branch {}: {:#}",
                branch, e
            );
            vec![]
        });

        if branch_files.is_empty() {
            continue;
        }

        let branch_tree = gitkyl::FileTree::from_files(branch_files.clone());

        let branch_file_paths: Vec<&str> = branch_files
            .iter()
            .filter_map(|f| f.path()?.to_str())
            .collect();

        let branch_commit_map =
            gitkyl::get_last_commits_batch(&config.repo, Some(branch), &branch_file_paths)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Warning: Failed to batch lookup commits for branch {}: {:#}",
                        branch, e
                    );
                    std::collections::HashMap::new()
                });

        let branch_commits =
            gitkyl::list_commits(&config.repo, Some(branch), None).unwrap_or_else(|e| {
                eprintln!(
                    "Warning: Failed to list commits for branch {}: {:#}",
                    branch, e
                );
                vec![]
            });

        let branch_latest_commit = branch_commits.first();

        let branch_directories = branch_tree.all_dirs();

        for dir_path in &branch_directories {
            validate_tree_path(dir_path)
                .with_context(|| format!("Invalid tree path: {}", dir_path))?;

            let entries_at_level = branch_tree.files_at(dir_path);
            let subdirs_at_level = branch_tree.subdirs_at(dir_path);

            let full_subdir_paths: Vec<String> = subdirs_at_level
                .iter()
                .map(|subdir| {
                    if dir_path.is_empty() {
                        subdir.to_string()
                    } else {
                        format!("{}/{}", dir_path, subdir)
                    }
                })
                .collect();

            let dir_paths_refs: Vec<&str> = full_subdir_paths.iter().map(|s| s.as_str()).collect();

            let level_dir_commit_map = if !dir_paths_refs.is_empty() {
                gitkyl::get_last_commits_batch(&config.repo, Some(branch), &dir_paths_refs)
                    .unwrap_or_else(|e| {
                        eprintln!(
                            "Warning: Failed to lookup directory commits for branch {} dir {}: {:#}",
                            branch, dir_path, e
                        );
                        std::collections::HashMap::new()
                    })
            } else {
                std::collections::HashMap::new()
            };

            let tree_items = build_tree_items(
                entries_at_level,
                &subdirs_at_level,
                dir_path,
                &branch_commit_map,
                &level_dir_commit_map,
            );

            let html_result = if dir_path.is_empty() {
                let readme_html =
                    gitkyl::pages::index::find_and_render_readme(&config.repo, branch, &tree_items)
                        .ok()
                        .flatten();

                let depth = branch.matches('/').count() + 2;
                Ok(gitkyl::pages::index::generate(IndexPageData {
                    name: repo_info.name(),
                    owner: repo_info.owner(),
                    default_branch: branch,
                    branches: repo_info.branches(),
                    commit_count: branch_commits.len(),
                    latest_commit: branch_latest_commit,
                    items: &tree_items,
                    readme_html: readme_html.as_deref(),
                    depth,
                }))
            } else {
                gitkyl::pages::tree::generate(
                    &config.repo,
                    branch,
                    dir_path,
                    repo_info.name(),
                    &tree_items,
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

                    total_tree_pages += 1;
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to generate tree page for branch {} dir {}: {:#}",
                        branch, dir_path, e
                    );
                }
            }
        }

        println!(
            "Generated tree pages for branch: {} ({} directories)",
            branch,
            branch_directories.len()
        );

        println!("Generating blob pages for branch: {}", branch);
        let mut branch_blob_count = 0;
        let mut branch_markdown_count = 0;

        for entry in &branch_files {
            if let Some(path) = entry.path() {
                if path.to_str().is_none() {
                    eprintln!(
                        "Warning: Skipping file with invalid UTF-8 path: {}",
                        path.display()
                    );
                    continue;
                }

                let result = if gitkyl::is_readme(path) {
                    branch_markdown_count += 1;
                    gitkyl::pages::blob::generate_markdown(
                        &config.repo,
                        branch,
                        path,
                        &config
                            .project_name()
                            .context("Failed to determine project name")?,
                    )
                } else {
                    gitkyl::pages::blob::generate(
                        &config.repo,
                        branch,
                        path,
                        &config
                            .project_name()
                            .context("Failed to determine project name")?,
                        &config.theme,
                    )
                };

                match result {
                    Ok(html) => {
                        let blob_path = config
                            .output
                            .join("blob")
                            .join(branch)
                            .join(format!("{}.html", path.display()));

                        if let Some(parent) = blob_path.parent() {
                            fs::create_dir_all(parent)
                                .context("Failed to create blob directory")?;
                        }

                        fs::write(&blob_path, html.into_string()).with_context(|| {
                            format!("Failed to write blob page {}", blob_path.display())
                        })?;

                        branch_blob_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to generate blob for branch {} file {}: {:#}",
                            branch,
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        println!(
            "Generated {} blob pages for branch: {} ({} markdown)",
            branch_blob_count, branch, branch_markdown_count
        );

        let branch_commits_html =
            gitkyl::pages::commits::generate(&branch_commits, branch, repo_info.name());

        let branch_commits_dir = config.output.join("commits").join(branch);

        fs::create_dir_all(&branch_commits_dir).context("Failed to create commits directory")?;

        let branch_commits_path = branch_commits_dir.join("index.html");
        fs::write(&branch_commits_path, branch_commits_html.into_string()).with_context(|| {
            format!(
                "Failed to write commits page to {}",
                branch_commits_path.display()
            )
        })?;

        println!(
            "Generated commits page for branch: {} ({} commits)",
            branch,
            branch_commits.len()
        );
    }

    println!(
        "Generated {} total tree pages across all branches",
        total_tree_pages
    );

    if !config.no_open {
        let index_path = config.output.join("index.html");
        if index_path.exists()
            && let Err(e) = open::that(&index_path)
        {
            eprintln!("Warning: Failed to open index.html: {}", e);
        }
    }

    Ok(())
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
    fn test_setup_creates_directories() {
        use std::path::PathBuf;
        use tempfile::TempDir;

        // Arrange: create temporary output directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_path = temp_dir.path().join("output");

        // Act: call setup function
        let result = setup_output_directories(&output_path);

        // Assert: directories should be created
        assert!(
            result.is_ok(),
            "setup_output_directories failed: {:?}",
            result.err()
        );
        assert!(output_path.exists(), "Output directory not created");
        assert!(
            output_path.join("assets").exists(),
            "Assets directory not created"
        );
    }

    #[test]
    fn test_setup_writes_css_assets() {
        use tempfile::TempDir;

        // Arrange: create temporary output directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let output_path = temp_dir.path().join("output");

        // Act: call setup function
        let result = setup_output_directories(&output_path);

        // Assert: CSS files should be written
        assert!(
            result.is_ok(),
            "setup_output_directories failed: {:?}",
            result.err()
        );

        let assets_dir = output_path.join("assets");
        assert!(assets_dir.exists(), "Assets directory not created");

        // Check that CSS assets were written by verifying files exist
        let css_files = std::fs::read_dir(&assets_dir)
            .expect("Failed to read assets dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "css"))
            .count();

        assert!(css_files > 0, "No CSS files written to assets directory");
    }

    #[test]
    fn test_build_tree_items_empty() {
        use std::collections::HashMap;

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
        use std::collections::HashMap;

        // Arrange: directories only
        let file_entries = vec![];
        let subdir_names = vec!["src", "docs"];
        let dir_path = "";
        let file_commit_map = HashMap::new();

        let mut dir_commit_map = HashMap::new();
        dir_commit_map.insert(
            "src".to_string(),
            gitkyl::CommitInfo::new(
                "abc123".to_string(),
                "Initial commit".to_string(),
                "Initial commit\n\nFull message.".to_string(),
                "Test Author".to_string(),
                1704067200,
            ),
        );
        dir_commit_map.insert(
            "docs".to_string(),
            gitkyl::CommitInfo::new(
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
        use std::collections::HashMap;

        // Arrange: nested directory path
        let file_entries = vec![];
        let subdir_names = vec!["utils"];
        let dir_path = "src/lib";
        let file_commit_map = HashMap::new();

        let mut dir_commit_map = HashMap::new();
        dir_commit_map.insert(
            "src/lib/utils".to_string(),
            gitkyl::CommitInfo::new(
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
