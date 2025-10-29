//! Static site generator for Git repositories.

mod config;
mod git;

pub use config::Config;
pub use git::{RepoInfo, analyze_repository};
