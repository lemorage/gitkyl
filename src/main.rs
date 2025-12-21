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

/// Generation statistics for a single branch.
#[derive(Debug, Default, Clone)]
struct BranchStats {
    tree_pages: usize,
    blob_pages: usize,
    markdown_pages: usize,
}

impl BranchStats {
    fn total_blobs(&self) -> usize {
        self.blob_pages + self.markdown_pages
    }
}

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
fn generate_tree_pages_for_branch(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    tree: &gitkyl::FileTree,
    file_commit_map: &std::collections::HashMap<String, gitkyl::CommitInfo>,
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
                    std::collections::HashMap::new()
                })
        } else {
            std::collections::HashMap::new()
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

            Ok(gitkyl::pages::index::generate(IndexPageData {
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

/// Generates blob pages for all files in a branch.
///
/// Creates HTML pages for all files in the specified branch, with special
/// handling for markdown files. README files are rendered with full markdown
/// processing, while code files receive syntax highlighting. Image files
/// are copied as raw files alongside their HTML viewer pages for use in
/// markdown image references.
///
/// # Arguments
///
/// * `config`: Application configuration including output path and theme
/// * `repo_info`: Repository metadata including name
/// * `branch`: Branch name to generate blob pages for
/// * `files`: File entries to process
///
/// # Returns
///
/// Tuple of (code blob count, markdown file count)
///
/// # Errors
///
/// Returns error if blob page generation or file writing fails
fn generate_blob_pages_for_branch(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    files: &[gitkyl::FileEntry],
) -> Result<(usize, usize)> {
    let mut blob_count = 0;
    let mut markdown_count = 0;

    for entry in files {
        if let Some(path) = entry.path() {
            if path.to_str().is_none() {
                eprintln!(
                    "Warning: Skipping file with invalid UTF-8 path: {}",
                    path.display()
                );
                continue;
            }

            let result = if gitkyl::is_readme(path) {
                markdown_count += 1;
                gitkyl::pages::blob::generate_markdown(&config.repo, branch, path, repo_info.name())
            } else {
                gitkyl::pages::blob::generate(
                    &config.repo,
                    branch,
                    path,
                    repo_info.name(),
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
                        fs::create_dir_all(parent).context("Failed to create blob directory")?;
                    }

                    fs::write(&blob_path, html.into_string()).with_context(|| {
                        format!("Failed to write blob page {}", blob_path.display())
                    })?;

                    // Copy raw image files for markdown image references
                    if let Ok(bytes) = gitkyl::read_blob(&config.repo, Some(branch), path)
                        && let gitkyl::FileType::Image(_) = gitkyl::detect_file_type(&bytes, path)
                    {
                        let raw_path = config.output.join("blob").join(branch).join(path);

                        if let Some(parent) = raw_path.parent() {
                            fs::create_dir_all(parent)
                                .context("Failed to create raw image directory")?;
                        }

                        fs::write(&raw_path, &bytes).with_context(|| {
                            format!("Failed to write raw image {}", raw_path.display())
                        })?;
                    }

                    blob_count += 1;
                }
                Err(e) => {
                    let err_msg = format!("{:?}", e);
                    if err_msg.contains("not a blob") {
                        continue;
                    }
                    return Err(e).with_context(|| {
                        format!("Failed to generate blob page for {}", path.display())
                    });
                }
            }
        }
    }

    Ok((blob_count, markdown_count))
}

/// Generates commits log page for a branch with pagination.
///
/// # Arguments
///
/// * `config`: Application configuration containing output path
/// * `repo_info`: Repository metadata including name and commit count
/// * `branch`: Branch name to generate commits page for
///
/// # Errors
///
/// Returns error if commit listing or page writing fails
fn generate_commits_page_for_branch(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
) -> Result<()> {
    let commits_dir = config.output.join("commits").join(branch);
    fs::create_dir_all(&commits_dir).context("Failed to create commits directory")?;

    let total_commits = repo_info.commit_count();
    let mut page = 1;

    loop {
        let paginated =
            gitkyl::list_commits_paginated(&config.repo, Some(branch), page, DEFAULT_COMMIT_LIMIT)
                .context("Failed to list paginated commits")?;

        let commits_html =
            gitkyl::pages::commits::generate(&paginated, branch, repo_info.name(), total_commits);

        let page_path = commits_dir.join(format!("page-{}.html", page));
        fs::write(&page_path, commits_html.into_string())
            .with_context(|| format!("Failed to write commits page to {}", page_path.display()))?;

        if !paginated.has_more {
            break;
        }

        page += 1;
    }

    Ok(())
}

/// Generates all pages for a single branch.
///
/// Orchestrates generation of tree pages, blob pages, and commits page for
/// the specified branch. Returns statistics for reporting.
///
/// # Arguments
///
/// * `config`: CLI configuration
/// * `repo_info`: Repository metadata
/// * `branch`: Branch name to generate for
///
/// # Returns
///
/// Statistics about generated pages
///
/// # Errors
///
/// Returns error if any critical generation step fails
fn generate_all_pages_for_branch(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
) -> Result<BranchStats> {
    let files = gitkyl::list_files(&config.repo, Some(branch)).context("Failed to list files")?;

    let tree = gitkyl::FileTree::from_files(files.clone());

    let file_paths: Vec<&str> = files.iter().filter_map(|f| f.path()?.to_str()).collect();

    let commit_map = gitkyl::get_last_commits_batch(&config.repo, Some(branch), &file_paths)
        .unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to batch lookup commits for branch {}: {:#}",
                branch, e
            );
            std::collections::HashMap::new()
        });

    let tree_pages = generate_tree_pages_for_branch(config, repo_info, branch, &tree, &commit_map)?;

    let (blob_pages, markdown_pages) =
        generate_blob_pages_for_branch(config, repo_info, branch, &files)?;

    generate_commits_page_for_branch(config, repo_info, branch)?;

    Ok(BranchStats {
        tree_pages,
        blob_pages,
        markdown_pages,
    })
}

/// Generates tags listing and detail pages.
///
/// Creates a tags index page listing all repository tags, plus individual
/// detail pages for each tag showing commit information.
///
/// # Arguments
///
/// * `config`: Application configuration containing repository and output paths
/// * `repo_info`: Repository metadata including name
///
/// # Returns
///
/// Count of tags processed
///
/// # Errors
///
/// Returns error if tag listing or page generation fails
fn generate_tags_pages(config: &Config, repo_info: &gitkyl::RepoInfo) -> Result<usize> {
    let tags = gitkyl::list_tags(&config.repo).context("Failed to list tags")?;

    if tags.is_empty() {
        return Ok(0);
    }

    let tags_dir = config.output.join("tags");
    fs::create_dir_all(&tags_dir).context("Failed to create tags directory")?;

    let tags_index_html = gitkyl::pages::tags::generate_list(repo_info.name(), &tags);
    let index_path = tags_dir.join("index.html");
    fs::write(&index_path, tags_index_html.into_string())
        .with_context(|| format!("Failed to write tags index to {}", index_path.display()))?;

    for tag in &tags {
        let commits =
            gitkyl::list_commits(&config.repo, Some(&tag.name), Some(1)).unwrap_or_else(|e| {
                eprintln!(
                    "Warning: Failed to get commit for tag {}: {:#}",
                    tag.name, e
                );
                vec![]
            });

        if let Some(commit) = commits.first() {
            let tag_html = gitkyl::pages::tags::generate_detail(
                repo_info.name(),
                tag,
                commit.message(),
                commit.author(),
                commit.date(),
            );

            let tag_path = tags_dir.join(format!("{}.html", tag.name));
            fs::write(&tag_path, tag_html.into_string())
                .with_context(|| format!("Failed to write tag page to {}", tag_path.display()))?;
        }
    }

    Ok(tags.len())
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

    let tree = gitkyl::FileTree::from_files(files.clone());

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

    let readme_html =
        find_and_render_readme(&config.repo, repo_info.default_branch(), &tree_items, 0)
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to render README: {:#}", e);
                None
            });

    let tag_count = gitkyl::list_tags(&config.repo)
        .map(|tags| tags.len())
        .unwrap_or(0);

    let html = index_page(IndexPageData {
        name: &config
            .project_name()
            .context("Failed to determine project name")?,
        owner: repo_info.owner(),
        default_branch: repo_info.default_branch(),
        branches: repo_info.branches(),
        commit_count: repo_info.commit_count(),
        tag_count,
        latest_commit: latest_commit.as_ref(),
        items: &tree_items,
        readme_html: readme_html.as_deref(),
        depth: 0,
    });

    let index_path = config.output.join("index.html");
    fs::write(&index_path, html.into_string())
        .with_context(|| format!("Failed to write index page to {}", index_path.display()))?;

    let default_stats =
        generate_all_pages_for_branch(&config, &repo_info, repo_info.default_branch())?;

    println!(
        "→ {}: {} trees, {} blobs ({} md)",
        repo_info.default_branch(),
        default_stats.tree_pages,
        default_stats.total_blobs(),
        default_stats.markdown_pages
    );

    let mut total_trees = default_stats.tree_pages;
    let mut total_blobs = default_stats.total_blobs();
    let mut branch_count = 1;

    for branch in repo_info.branches() {
        if branch == repo_info.default_branch() {
            continue;
        }

        match generate_all_pages_for_branch(&config, &repo_info, branch) {
            Ok(stats) => {
                println!(
                    "→ {}: {} trees, {} blobs ({} md)",
                    branch,
                    stats.tree_pages,
                    stats.total_blobs(),
                    stats.markdown_pages
                );
                total_trees += stats.tree_pages;
                total_blobs += stats.total_blobs();
                branch_count += 1;
            }
            Err(e) => {
                eprintln!("✗ {}: {:#}", branch, e);
            }
        }
    }

    let tags_count = generate_tags_pages(&config, &repo_info).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to generate tags pages: {:#}", e);
        0
    });

    println!(
        "✓ Generated {} trees, {} blobs ({} branches, {} tags)",
        total_trees, total_blobs, branch_count, tags_count
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
