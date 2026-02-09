//! Blob page generation for file viewing.

use anyhow::{Context, Result};
use gitkyl::{Config, FileEntry};
use rayon::prelude::*;
use std::fs;
use std::path::Path;

/// Result of processing a single blob entry.
enum BlobKind {
    /// Markdown file
    Markdown,
    /// Code file
    Code,
    /// Entry skipped: not a blob, invalid path, or unprocessable.
    Skipped,
}

/// Processes a single blob entry.
fn process_blob_entry(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    entry: &FileEntry,
) -> Result<BlobKind> {
    let Some(path) = entry.path() else {
        return Ok(BlobKind::Skipped);
    };

    if path.to_str().is_none() {
        eprintln!(
            "Warning: Skipping file with invalid UTF-8 path: {}",
            path.display()
        );
        return Ok(BlobKind::Skipped);
    }

    if gitkyl::is_markdown(path) {
        process_markdown_blob(config, repo_info, branch, entry, path)?;
        return Ok(BlobKind::Markdown);
    }

    process_code_blob(config, repo_info, branch, entry, path)
}

/// Processes a markdown file: rendered view, source view, and blame.
fn process_markdown_blob(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    entry: &FileEntry,
    path: &Path,
) -> Result<()> {
    // Read blob once by oid, no redundant tree traversal
    let bytes = gitkyl::read_blob_by_oid(&config.repo, entry.oid())
        .with_context(|| format!("Failed to read blob {}", path.display()))?;

    let rendered =
        gitkyl::pages::blob::generate_markdown_from_content(&bytes, path, branch, repo_info.name())
            .with_context(|| format!("Failed to render markdown {}", path.display()))?;

    let blob_path = config
        .output
        .join("blob")
        .join(branch)
        .join(format!("{}.html", path.display()));

    if let Some(parent) = blob_path.parent() {
        fs::create_dir_all(parent).context("Failed to create blob directory")?;
    }

    fs::write(&blob_path, rendered.into_string())
        .with_context(|| format!("Failed to write blob page {}", blob_path.display()))?;

    // Reuse already-read bytes for source view
    let source = gitkyl::pages::blob::generate_markdown_source_from_content(
        &bytes,
        path,
        branch,
        repo_info.name(),
        &config.theme,
    )
    .with_context(|| format!("Failed to highlight markdown source {}", path.display()))?;

    let source_path = config
        .output
        .join("blob")
        .join(branch)
        .join(format!("{}.source.html", path.display()));

    fs::write(&source_path, source.into_string())
        .with_context(|| format!("Failed to write source page {}", source_path.display()))?;

    write_blame_page(config, repo_info, branch, path);

    Ok(())
}

/// Processes a code file: syntax highlighted view, raw image copy, and blame.
fn process_code_blob(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    entry: &FileEntry,
    path: &Path,
) -> Result<BlobKind> {
    let bytes = gitkyl::read_blob_by_oid(&config.repo, entry.oid())
        .with_context(|| format!("Failed to read blob {}", path.display()))?;

    let html = gitkyl::pages::blob::generate_from_content(
        &bytes,
        path,
        branch,
        repo_info.name(),
        &config.theme,
    )
    .with_context(|| format!("Failed to generate blob page for {}", path.display()))?;

    let blob_path = config
        .output
        .join("blob")
        .join(branch)
        .join(format!("{}.html", path.display()));

    if let Some(parent) = blob_path.parent() {
        fs::create_dir_all(parent).context("Failed to create blob directory")?;
    }

    fs::write(&blob_path, html.into_string())
        .with_context(|| format!("Failed to write blob page {}", blob_path.display()))?;

    // Reuse already-read bytes for image copy and blame
    match gitkyl::detect_file_type(&bytes, path) {
        gitkyl::FileType::Image(_) => {
            let raw_path = config.output.join("blob").join(branch).join(path);

            if let Some(parent) = raw_path.parent() {
                fs::create_dir_all(parent).context("Failed to create raw image directory")?;
            }

            fs::write(&raw_path, &bytes)
                .with_context(|| format!("Failed to write raw image {}", raw_path.display()))?;
        }
        gitkyl::FileType::Text => {
            write_blame_page(config, repo_info, branch, path);
        }
        _ => {}
    }

    Ok(BlobKind::Code)
}

/// Writes blame page for a file if blame generation succeeds.
fn write_blame_page(config: &Config, repo_info: &gitkyl::RepoInfo, branch: &str, path: &Path) {
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
pub fn generate_blob_pages_for_branch(
    config: &Config,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    files: &[FileEntry],
) -> Result<(usize, usize)> {
    let results: Vec<BlobKind> = files
        .par_iter()
        .map(|entry| process_blob_entry(config, repo_info, branch, entry))
        .collect::<Result<Vec<_>>>()?;

    let (blob_count, markdown_count) =
        results
            .iter()
            .fold((0, 0), |(blobs, mds), kind| match kind {
                BlobKind::Markdown => (blobs + 1, mds + 1),
                BlobKind::Code => (blobs + 1, mds),
                BlobKind::Skipped => (blobs, mds),
            });

    Ok((blob_count, markdown_count))
}
