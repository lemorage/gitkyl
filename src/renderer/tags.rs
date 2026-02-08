//! Tag listing and detail page generation.

use anyhow::{Context, Result};
use gitkyl::Config;
use std::fs;

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
pub fn generate_tags_pages(config: &Config, repo_info: &gitkyl::RepoInfo) -> Result<usize> {
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
