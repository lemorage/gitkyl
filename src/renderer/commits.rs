//! Commit log and detail page generation.

use anyhow::{Context, Result};
use gitkyl::Config;
use std::fs;

/// Default limit for commits displayed on commit log page.
pub const DEFAULT_COMMIT_LIMIT: usize = 35;

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
pub fn generate_commits_page_for_branch(
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
pub fn generate_commit_detail_pages(
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
