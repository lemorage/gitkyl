mod renderer;

use anyhow::{Context, Result};
use gitkyl::Config;
use gitkyl::pages::index::{IndexPageData, find_and_render_readme, generate as index_page};
use renderer::BranchStats;
use std::fs;

/// Accumulated generation statistics across all refs.
#[derive(Default)]
struct Totals {
    trees: usize,
    blobs: usize,
    commits: usize,
    refs: usize,
}

impl Totals {
    /// Prints branch stats and accumulates into totals.
    fn record(&mut self, name: &str, stats: &BranchStats) {
        println!(
            "→ {}: {} trees, {} blobs, {} commits",
            name, stats.tree_pages, stats.blob_pages, stats.commit_pages
        );
        self.trees += stats.tree_pages;
        self.blobs += stats.blob_pages;
        self.commits += stats.commit_pages;
        self.refs += 1;
    }
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
        renderer::generate_all_pages_for_branch(&config, &repo_info, repo_info.default_branch())?;

    let mut totals = Totals::default();
    totals.record(repo_info.default_branch(), &default_stats);

    for branch in repo_info.branches() {
        if branch == repo_info.default_branch() {
            continue;
        }
        match renderer::generate_all_pages_for_branch(&config, &repo_info, branch) {
            Ok(stats) => totals.record(branch, &stats),
            Err(e) => eprintln!("✗ {}: {:#}", branch, e),
        }
    }

    let branch_count = totals.refs;

    // Generate tree and blob pages for tags
    let tags = gitkyl::list_tags(&config.repo).unwrap_or_default();
    for tag in &tags {
        match renderer::generate_all_pages_for_branch(&config, &repo_info, &tag.name) {
            Ok(stats) => totals.record(&tag.name, &stats),
            Err(e) => eprintln!("✗ tag {}: {:#}", tag.name, e),
        }
    }

    let tags_count = renderer::generate_tags_pages(&config, &repo_info).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to generate tags pages: {:#}", e);
        0
    });

    println!(
        "✓ Generated {} trees, {} blobs, {} commits ({} branches, {} tags)",
        totals.trees, totals.blobs, totals.commits, branch_count, tags_count
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
