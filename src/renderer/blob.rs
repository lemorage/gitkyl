//! Blob page generation for file viewing.

use anyhow::{Context, Result};
use gitkyl::{Config, FileEntry};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Output type for cache key differentiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputType {
    /// Rendered HTML (code highlighting or markdown render)
    Rendered,
    /// Markdown source with syntax highlighting
    Source,
}

impl OutputType {
    fn as_suffix(&self) -> &'static str {
        match self {
            Self::Rendered => "",
            Self::Source => ":source",
        }
    }
}

/// Cache for generated blob HTML, keyed by "{oid}:{type}".
#[derive(Debug, Clone, Default)]
pub struct BlobCache {
    inner: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl BlobCache {
    /// Creates a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds cache key from OID and output type.
    fn key(oid: &gix::ObjectId, output_type: OutputType) -> String {
        format!("{}{}", oid, output_type.as_suffix())
    }

    /// Returns cached HTML if present.
    pub fn get(&self, oid: &gix::ObjectId, output_type: OutputType) -> Option<Vec<u8>> {
        let key = Self::key(oid, output_type);
        self.inner.read().ok()?.get(&key).cloned()
    }

    /// Stores HTML in cache and returns it for chaining.
    pub fn insert(&self, oid: &gix::ObjectId, output_type: OutputType, html: Vec<u8>) -> Vec<u8> {
        let key = Self::key(oid, output_type);
        if let Ok(mut cache) = self.inner.write() {
            cache.insert(key, html.clone());
        }
        html
    }

    /// Returns number of cached entries.
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.read().map(|c| c.len()).unwrap_or(0)
    }
}

/// Returns the blob output directory for a branch.
#[inline]
fn blob_dir(config: &Config, branch: &str) -> PathBuf {
    config.output.join("blob").join(branch)
}

/// Creates all unique parent directories for a batch of files.
fn ensure_directories(base: &Path, files: &[FileEntry]) -> Result<()> {
    let dirs: HashSet<PathBuf> = files
        .iter()
        .filter_map(|f| f.path())
        .filter_map(|p| p.parent())
        .map(|p| base.join(p))
        .collect();

    for dir in dirs {
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory {}", dir.display()))?;
    }

    Ok(())
}

/// Processes a single blob entry, returning true if processed.
fn process_blob_entry(
    config: &Config,
    cache: &BlobCache,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    entry: &FileEntry,
) -> Result<bool> {
    let Some(path) = entry.path() else {
        return Ok(false);
    };

    if path.to_str().is_none() {
        eprintln!(
            "Warning: Skipping file with invalid UTF-8 path: {}",
            path.display()
        );
        return Ok(false);
    }

    if gitkyl::is_markdown(path) {
        process_markdown_blob(config, cache, repo_info, branch, entry, path)?;
        return Ok(true);
    }

    process_code_blob(config, cache, repo_info, branch, entry, path)
}

/// Processes a markdown file: rendered view, source view, and blame.
fn process_markdown_blob(
    config: &Config,
    cache: &BlobCache,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    entry: &FileEntry,
    path: &Path,
) -> Result<()> {
    let oid = entry.oid();
    let base = blob_dir(config, branch);

    // Rendered markdown HTML
    let out_path = base.join(format!("{}.html", path.display()));
    let rendered_html = if let Some(cached) = cache.get(oid, OutputType::Rendered) {
        cached
    } else {
        let bytes = gitkyl::read_blob_by_oid(&config.repo, oid)
            .with_context(|| format!("Failed to read blob {}", path.display()))?;
        let rendered = gitkyl::pages::blob::generate_markdown_from_content(
            &bytes,
            path,
            branch,
            repo_info.name(),
            config.ui_mode().markdown_class(),
        )
        .with_context(|| format!("Failed to render markdown {}", path.display()))?;
        cache.insert(
            oid,
            OutputType::Rendered,
            rendered.into_string().into_bytes(),
        )
    };
    fs::write(&out_path, &rendered_html)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;

    // Source view HTML
    let source_path = base.join(format!("{}.source.html", path.display()));
    let source_html = if let Some(cached) = cache.get(oid, OutputType::Source) {
        cached
    } else {
        let bytes = gitkyl::read_blob_by_oid(&config.repo, oid)
            .with_context(|| format!("Failed to read blob {}", path.display()))?;
        let source = gitkyl::pages::blob::generate_markdown_source_from_content(
            &bytes,
            path,
            branch,
            repo_info.name(),
            &config.theme,
        )
        .with_context(|| format!("Failed to highlight markdown source {}", path.display()))?;
        cache.insert(oid, OutputType::Source, source.into_string().into_bytes())
    };
    fs::write(&source_path, &source_html)
        .with_context(|| format!("Failed to write {}", source_path.display()))?;

    // Blame is not cached (history-dependent)
    write_blame_page(config, repo_info, branch, path);

    Ok(())
}

/// Processes a code file: syntax highlighted view, raw image copy, and blame.
fn process_code_blob(
    config: &Config,
    cache: &BlobCache,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    entry: &FileEntry,
    path: &Path,
) -> Result<bool> {
    let oid = entry.oid();
    let base = blob_dir(config, branch);
    let out_path = base.join(format!("{}.html", path.display()));

    // Check cache for rendered HTML
    let (html, bytes) = if let Some(cached) = cache.get(oid, OutputType::Rendered) {
        // Cache hit: still need bytes for image/blame checks
        let bytes = gitkyl::read_blob_by_oid(&config.repo, oid)
            .with_context(|| format!("Failed to read blob {}", path.display()))?;
        (cached, bytes)
    } else {
        // Cache miss: generate and cache
        let bytes = gitkyl::read_blob_by_oid(&config.repo, oid)
            .with_context(|| format!("Failed to read blob {}", path.display()))?;
        let generated = gitkyl::pages::blob::generate_from_content(
            &bytes,
            path,
            branch,
            repo_info.name(),
            &config.theme,
        )
        .with_context(|| format!("Failed to generate blob page for {}", path.display()))?;
        let html = cache.insert(
            oid,
            OutputType::Rendered,
            generated.into_string().into_bytes(),
        );
        (html, bytes)
    };

    fs::write(&out_path, &html)
        .with_context(|| format!("Failed to write {}", out_path.display()))?;

    match gitkyl::detect_file_type(&bytes, path) {
        gitkyl::FileType::Image(_) => {
            let img_path = base.join(path);
            fs::write(&img_path, &bytes)
                .with_context(|| format!("Failed to write {}", img_path.display()))?;
        }
        gitkyl::FileType::Text => write_blame_page(config, repo_info, branch, path),
        _ => {}
    }

    Ok(true)
}

/// Writes blame page for a file if blame generation succeeds.
fn write_blame_page(config: &Config, repo_info: &gitkyl::RepoInfo, branch: &str, path: &Path) {
    let Ok(blame) = gitkyl::pages::blob::generate_blame(
        &config.repo,
        branch,
        path,
        repo_info.name(),
        &config.theme,
    ) else {
        return;
    };

    let blame_path = blob_dir(config, branch).join(format!("{}.blame.html", path.display()));

    if let Err(e) = fs::write(&blame_path, blame.into_string()) {
        eprintln!("Warning: Failed to write {}: {}", blame_path.display(), e);
    }
}

/// Generates blob pages for all files in a branch.
///
/// Creates HTML pages for all files in the specified branch. Markdown files
/// get rendered views plus source views. Code files receive syntax highlighting.
/// Image files are copied raw for markdown references. Text files get blame views.
///
/// # Arguments
///
/// * `config`: Application configuration including output path and theme
/// * `cache`: Shared blob cache for deduplication
/// * `repo_info`: Repository metadata including name
/// * `branch`: Branch name to generate blob pages for
/// * `files`: File entries to process
///
/// # Returns
///
/// Count of blob pages generated
///
/// # Errors
///
/// Returns error if blob page generation or file writing fails
pub fn generate_blob_pages_for_branch(
    config: &Config,
    cache: &BlobCache,
    repo_info: &gitkyl::RepoInfo,
    branch: &str,
    files: &[FileEntry],
) -> Result<usize> {
    // Pre-create all directories
    ensure_directories(&blob_dir(config, branch), files)?;

    let count = files
        .par_iter()
        .map(|entry| process_blob_entry(config, cache, repo_info, branch, entry))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|&processed| processed)
        .count();

    Ok(count)
}
