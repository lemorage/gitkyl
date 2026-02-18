//! Static site generator for Git repositories.

mod assets;
pub mod avatar;
pub mod components;
mod config;
mod filetype;
mod git;
mod highlight;
pub mod icons;
mod markdown;
pub mod pages;
mod tree;
mod util;

pub use assets::write_css_assets;
pub use avatar::render;
pub use config::{Config, UiMode};
pub use filetype::{FileType, ImageFormat, detect_file_type, is_binary_content};
pub use git::{
    BlameLine, BlameResult, ChangeType, ChangedFile, CommitDiff, CommitInfo, DiffHunk, DiffLine,
    DiffLineType, FileEntry, FileStats, PaginatedCommits, RepoInfo, TagInfo, TreeItem,
    analyze_repository, blame_file, get_commit_by_oid, get_commit_diff, get_last_commits_batch,
    list_commits, list_commits_paginated, list_files, list_tags, read_blob, read_blob_by_oid,
};
pub use highlight::{Highlighter, highlight};
pub use icons::{is_markdown, is_readme};
pub use markdown::{LinkResolver, MarkdownRenderer};
pub use tree::FileTree;
