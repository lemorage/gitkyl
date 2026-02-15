//! Per-branch page generation orchestration.

use crate::renderer::{self, BlobCache};
use anyhow::{Context, Result};
use gitkyl::Config;
use std::collections::HashMap;

/// Generation statistics for a single branch.
#[derive(Debug, Default, Clone)]
pub struct BranchStats {
    pub tree_pages: usize,
    pub blob_pages: usize,
    pub commit_pages: usize,
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
/// * `cache`: Shared blob cache for deduplication across refs
///
/// # Returns
///
/// Statistics about generated pages
///
/// # Errors
///
/// Returns error if any critical generation step fails
pub fn generate_all_pages_for_branch(
    config: &Config,
    cache: &BlobCache,
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
            HashMap::new()
        });

    let tree_pages =
        renderer::generate_tree_pages_for_branch(config, repo_info, branch, &tree, &commit_map)?;

    let blob_pages =
        renderer::generate_blob_pages_for_branch(config, cache, repo_info, branch, &files)?;

    renderer::generate_commits_page_for_branch(config, repo_info, branch)?;

    let commit_pages = renderer::generate_commit_detail_pages(config, repo_info, branch)
        .unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to generate commit detail pages for branch {}: {:#}",
                branch, e
            );
            0
        });

    Ok(BranchStats {
        tree_pages,
        blob_pages,
        commit_pages,
    })
}
