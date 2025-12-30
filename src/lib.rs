//! Static site generator for Git repositories.

mod assets;
pub mod avatar;
pub mod components;
mod config;
mod filetype;
mod git;
mod highlight;
mod markdown;
pub mod pages;
mod tree;
mod util;

pub use assets::write_css_assets;
pub use avatar::render;
pub use components::icons::{is_markdown, is_readme};
pub use config::Config;
pub use filetype::{FileType, ImageFormat, detect_file_type};
pub use git::{
    CommitInfo, FileEntry, PaginatedCommits, RepoInfo, TagInfo, TreeItem, analyze_repository,
    get_last_commits_batch, list_commits, list_commits_paginated, list_files, list_tags, read_blob,
};
pub use highlight::{Highlighter, highlight};
pub use markdown::{LinkResolver, MarkdownRenderer};
pub use tree::FileTree;
