//! Shared test utilities for integration tests.
//!
//! Provides helper functions for creating temporary git repositories and
//! performing common git operations used across multiple test files.

use anyhow::Result;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Creates temporary git repository with test configuration.
///
/// Sets up a clean git repository with user name and email configured.
/// Uses anyhow::Result for proper error propagation in integration tests.
///
/// # Returns
///
/// Temporary directory containing initialized git repository
///
/// # Errors
///
/// Returns error if git commands fail or directory creation fails
pub fn create_test_repo() -> Result<TempDir> {
    let dir = TempDir::new()?;
    let path = dir.path();

    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()?;

    Ok(dir)
}

/// Commits staged changes and returns commit hash.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `message`: Commit message
///
/// # Returns
///
/// Full commit hash as string
///
/// # Errors
///
/// Returns error if commit fails or hash cannot be retrieved
pub fn git_commit(repo_path: &Path, message: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Stages files in repository.
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `files`: File paths to stage
///
/// # Errors
///
/// Returns error if git add fails
///
/// # Examples
///
/// ```no_run
/// # use anyhow::Result;
/// # use std::path::Path;
/// # fn example(repo_path: &Path) -> Result<()> {
/// // Stage all changes
/// common::git_add(repo_path, &["."])?;
///
/// // Stage specific files
/// common::git_add(repo_path, &["file1.txt", "file2.rs"])?;
/// # Ok(())
/// # }
/// ```
pub fn git_add(repo_path: &Path, files: &[&str]) -> Result<()> {
    let mut args = vec!["add"];
    args.extend_from_slice(files);

    let output = Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Writes file to repository, creating parent directories as needed.
///
/// # Errors
///
/// Returns error if directory creation or file write fails
pub fn write_file(repo_path: &Path, path: &str, content: &str) -> Result<()> {
    let file_path = repo_path.join(path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file_path, content)?;
    Ok(())
}
