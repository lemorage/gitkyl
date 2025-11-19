//! Static site generator for Git repositories.

mod config;
mod generators;
mod git;
mod highlight;
mod tree;

pub use config::Config;
pub use generators::{TreeItem, generate_blob_page, generate_commits_page, generate_tree_page};
pub use git::{
    CommitInfo, FileEntry, RepoInfo, analyze_repository, get_file_last_commit,
    get_last_commits_batch, list_commits, list_files, read_blob,
};
pub use highlight::{Highlighter, highlight};
pub use tree::FileTree;
