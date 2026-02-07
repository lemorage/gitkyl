//! Blob page generation for file viewing.

use anyhow::{Context, Result};
use gitkyl::{Config, FileEntry};
use std::fs;
use std::path::Path;

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
