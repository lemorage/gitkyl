mod renderer;

use anyhow::{Context, Result};
use gitkyl::Config;
use gitkyl::pages::index::{IndexPageData, find_and_render_readme, generate as index_page};
use std::fs;

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
        renderer::generate_all_pages_for_branch(&config, &repo_info, repo_info.default_branch())?;

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

        match renderer::generate_all_pages_for_branch(&config, &repo_info, branch) {
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
        match renderer::generate_all_pages_for_branch(&config, &repo_info, &tag.name) {
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

    let tags_count = renderer::generate_tags_pages(&config, &repo_info).unwrap_or_else(|e| {
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
