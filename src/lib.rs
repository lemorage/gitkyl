//! Static site generator for Git repositories.

mod config;
mod generators;
mod git;
mod highlight;

pub use config::Config;
pub use generators::{generate_blob_page, generate_commits_page};
pub use git::{
    CommitInfo, FileEntry, RepoInfo, analyze_repository, list_commits, list_files, read_blob,
};
pub use highlight::{Language, highlight};
