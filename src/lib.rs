//! Static site generator for Git repositories.

mod assets;
pub mod components;
mod config;
mod git;
mod highlight;
mod markdown;
pub mod pages;
mod tree;
mod util;

pub use assets::write_css_assets;
pub use components::icons::is_readme;
pub use config::Config;
pub use git::{
    CommitInfo, FileEntry, RepoInfo, TreeItem, analyze_repository, get_file_last_commit,
    get_last_commits_batch, list_commits, list_files, read_blob,
};
pub use highlight::{Highlighter, highlight};
pub use markdown::{LinkResolver, MarkdownRenderer};
pub use tree::FileTree;
