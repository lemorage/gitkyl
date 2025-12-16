//! CSS asset bundling

use anyhow::{Context, Result};
use std::{fs, path::Path};

const BASE: &str = include_str!("../assets/base.css");
const LAYOUT: &str = include_str!("../assets/components/layout.css");
const NAV: &str = include_str!("../assets/components/nav.css");
const FILE_LIST: &str = include_str!("../assets/components/file-list.css");

const INDEX_PAGE: &str = include_str!("../assets/page-index.css");
const TREE_PAGE: &str = include_str!("../assets/page-tree.css");
const BLOB_PAGE: &str = include_str!("../assets/page-blob.css");
const COMMITS_PAGE: &str = include_str!("../assets/page-commits.css");
const TAGS_PAGE: &str = include_str!("../assets/page-tags.css");
const MARKDOWN: &str = include_str!("../assets/markdown.css");

/// Writes all bundled CSS assets to output directory
pub fn write_css_assets(assets_dir: &Path) -> Result<()> {
    write_bundled(
        assets_dir,
        "index.css",
        &[BASE, LAYOUT, FILE_LIST, INDEX_PAGE],
    )?;
    write_bundled(
        assets_dir,
        "tree.css",
        &[BASE, LAYOUT, NAV, FILE_LIST, TREE_PAGE],
    )?;
    write_bundled(assets_dir, "blob.css", &[BASE, LAYOUT, NAV, BLOB_PAGE])?;
    write_bundled(
        assets_dir,
        "commits.css",
        &[BASE, LAYOUT, NAV, COMMITS_PAGE],
    )?;
    write_bundled(assets_dir, "tags.css", &[BASE, LAYOUT, NAV, TAGS_PAGE])?;
    write_bundled(assets_dir, "markdown.css", &[MARKDOWN])?;
    Ok(())
}

fn write_bundled(dir: &Path, name: &str, parts: &[&str]) -> Result<()> {
    let css = parts.join("\n");
    fs::write(dir.join(name), css)
        .with_context(|| format!("Failed to write CSS asset: {}", name))?;
    Ok(())
}
