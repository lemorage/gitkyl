mod renderer;

use anyhow::{Context, Result};
use gitkyl::Config;
use gitkyl::pages::index::{IndexPageData, find_and_render_readme, generate as index_page};
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
    commit_pages: usize,
}

impl BranchStats {
    fn total_blobs(&self) -> usize {
        self.blob_pages + self.markdown_pages
    }
}

/// Writes blame page for a file if blame generation succeeds
fn write_blame_page(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    path: &std::path::Path,
) {
    if let Ok(blame) = gitkyl::pages::blob::generate_blame(
        &config.repo,
        branch,
        path,
        repo_info.name(),
        &config.theme,
    ) {
        let blame_path = config
            .output
            .join("blob")
            .join(branch)
            .join(format!("{}.blame.html", path.display()));

        if let Err(e) = fs::write(&blame_path, blame.into_string()) {
            eprintln!(
                "Warning: Failed to write blame page {}: {}",
                blame_path.display(),
                e
            );
        }
    }
}

/// Generates blob pages for all files in a branch.
///
/// Creates HTML pages for all files in the specified branch, with special
/// handling for markdown files. README files are rendered with full markdown
/// processing, while code files receive syntax highlighting. Image files
/// are copied as raw files alongside their HTML viewer pages for use in
/// markdown image references. Text files also get blame views.
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

            let result = if gitkyl::is_markdown(path) {
                markdown_count += 1;

                // Generate rendered markdown view
                let rendered = gitkyl::pages::blob::generate_markdown(
                    &config.repo,
                    branch,
                    path,
                    repo_info.name(),
                )?;

                let blob_path = config
                    .output
                    .join("blob")
                    .join(branch)
                    .join(format!("{}.html", path.display()));

                if let Some(parent) = blob_path.parent() {
                    fs::create_dir_all(parent).context("Failed to create blob directory")?;
                }

                fs::write(&blob_path, rendered.into_string()).with_context(|| {
                    format!("Failed to write blob page {}", blob_path.display())
                })?;

                // Generate source view for markdown files
                let source = gitkyl::pages::blob::generate_markdown_source(
                    &config.repo,
                    branch,
                    path,
                    repo_info.name(),
                    &config.theme,
                )?;

                let source_path = config
                    .output
                    .join("blob")
                    .join(branch)
                    .join(format!("{}.source.html", path.display()));

                fs::write(&source_path, source.into_string()).with_context(|| {
                    format!("Failed to write source page {}", source_path.display())
                })?;

                write_blame_page(config, repo_info, branch, path);

                blob_count += 1;
                continue;
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
                    } else if let Ok(bytes) = gitkyl::read_blob(&config.repo, Some(branch), path)
                        && gitkyl::detect_file_type(&bytes, path) == gitkyl::FileType::Text
                    {
                        write_blame_page(config, repo_info, branch, path);
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

/// Generates individual commit detail pages for all commits in a branch.
///
/// Creates a commit/ directory and generates HTML pages showing full diffs
/// and metadata for each commit.
///
/// # Arguments
///
/// * `config`: CLI configuration
/// * `repo_info`: Repository metadata
/// * `branch`: Branch name to generate commit pages for
///
/// # Returns
///
/// Number of commit pages generated
///
/// # Errors
///
/// Returns error if commit listing or page generation fails
fn generate_commit_detail_pages(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
) -> Result<usize> {
    let commit_dir = config.output.join("commit");
    fs::create_dir_all(&commit_dir).context("Failed to create commit directory")?;

    // List all commits for this branch
    let all_commits =
        gitkyl::list_commits(&config.repo, Some(branch), None).context("Failed to list commits")?;

    let total = all_commits.len();

    for commit in all_commits {
        let commit_oid = commit.oid();
        let html = gitkyl::pages::commit::generate_commit_page(
            &config.repo,
            commit_oid,
            repo_info.name(),
            Some(branch),
        )
        .with_context(|| format!("Failed to generate commit page for {}", commit_oid))?;

        let commit_path = commit_dir.join(format!("{}.html", commit_oid));
        fs::write(&commit_path, html)
            .with_context(|| format!("Failed to write commit page to {}", commit_path.display()))?;
    }

    Ok(total)
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

    let tree_pages =
        renderer::generate_tree_pages_for_branch(config, repo_info, branch, &tree, &commit_map)?;

    let (blob_pages, markdown_pages) =
        generate_blob_pages_for_branch(config, repo_info, branch, &files)?;

    generate_commits_page_for_branch(config, repo_info, branch)?;

    let commit_pages =
        generate_commit_detail_pages(config, repo_info, branch).unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to generate commit detail pages for branch {}: {:#}",
                branch, e
            );
            0
        });

    Ok(BranchStats {
        tree_pages,
        blob_pages,
        markdown_pages,
        commit_pages,
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

    renderer::setup_output_directories(&config.output)?;

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

    let tree_items = renderer::build_tree_items(
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
        "→ {}: {} trees, {} blobs ({} md), {} commits",
        repo_info.default_branch(),
        default_stats.tree_pages,
        default_stats.total_blobs(),
        default_stats.markdown_pages,
        default_stats.commit_pages
    );

    let mut total_trees = default_stats.tree_pages;
    let mut total_blobs = default_stats.total_blobs();
    let mut total_commits = default_stats.commit_pages;
    let mut branch_count = 1;

    for branch in repo_info.branches() {
        if branch == repo_info.default_branch() {
            continue;
        }

        match generate_all_pages_for_branch(&config, &repo_info, branch) {
            Ok(stats) => {
                println!(
                    "→ {}: {} trees, {} blobs ({} md), {} commits",
                    branch,
                    stats.tree_pages,
                    stats.total_blobs(),
                    stats.markdown_pages,
                    stats.commit_pages
                );
                total_trees += stats.tree_pages;
                total_blobs += stats.total_blobs();
                total_commits += stats.commit_pages;
                branch_count += 1;
            }
            Err(e) => {
                eprintln!("✗ {}: {:#}", branch, e);
            }
        }
    }

    // Generate tree and blob pages for tags to enable file browsing
    let tags = gitkyl::list_tags(&config.repo).unwrap_or_default();
    for tag in &tags {
        match generate_all_pages_for_branch(&config, &repo_info, &tag.name) {
            Ok(stats) => {
                println!(
                    "→ {}: {} trees, {} blobs ({} md), {} commits",
                    tag.name,
                    stats.tree_pages,
                    stats.total_blobs(),
                    stats.markdown_pages,
                    stats.commit_pages
                );
                total_trees += stats.tree_pages;
                total_blobs += stats.total_blobs();
                total_commits += stats.commit_pages;
            }
            Err(e) => {
                eprintln!("✗ tag {}: {:#}", tag.name, e);
            }
        }
    }

    let tags_count = generate_tags_pages(&config, &repo_info).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to generate tags pages: {:#}", e);
        0
    });

    println!(
        "✓ Generated {} trees, {} blobs, {} commits ({} branches, {} tags)",
        total_trees, total_blobs, total_commits, branch_count, tags_count
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
