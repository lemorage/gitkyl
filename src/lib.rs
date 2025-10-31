//! Static site generator for Git repositories.

mod config;
mod git;

pub use config::Config;
pub use git::{FileEntry, RepoInfo, analyze_repository, list_files};
