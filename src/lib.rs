//! Static site generator for Git repositories.

mod config;
mod git;
mod highlight;

pub use config::Config;
pub use git::{FileEntry, RepoInfo, analyze_repository, list_files, read_blob};
pub use highlight::{Language, highlight};
