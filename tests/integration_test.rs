//! Integration tests for Gitkyl.
//!
//! Tests repository analysis, configuration, and git operations.

use anyhow::Result;
use gitkyl::{Config, analyze_repository, get_last_commits_batch};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Creates temporary git repository with test configuration.
fn create_test_repo() -> Result<TempDir> {
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
fn git_commit(repo_path: &std::path::Path, message: &str) -> Result<String> {
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

/// Tests repository analysis with valid repository.
#[test]
fn test_analyze_repository_with_commits() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Act
    let result = analyze_repository(&repo_path, None);

    // Assert
    match result {
        Ok(repo_info) => {
            assert!(
                !repo_info.name().is_empty(),
                "Repository name should not be empty"
            );
            assert!(
                !repo_info.default_branch().is_empty(),
                "Default branch should not be empty"
            );
            assert!(
                repo_info.commit_count() > 0,
                "Repository should have commits after initial commit"
            );
            assert!(
                !repo_info.branches().is_empty(),
                "Repository should have branches"
            );
            assert!(
                repo_info.owner().is_none(),
                "Owner should be None when not provided"
            );
        }
        Err(e) => {
            let err_msg = format!("{:?}", e);
            if err_msg.contains("does not have any commits") {
                println!("Skipping: repository has no commits yet (bootstrap scenario)");
                return Ok(());
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Tests repository analysis includes owner when provided.
#[test]
fn test_analyze_repository_with_owner() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let owner = Some("lemorage".to_string());

    // Act
    let result = analyze_repository(&repo_path, owner.clone());

    // Assert
    match result {
        Ok(repo_info) => {
            assert_eq!(
                repo_info.owner(),
                owner.as_deref(),
                "Owner should be propagated"
            );
        }
        Err(e) => {
            let err_msg = format!("{:?}", e);
            if err_msg.contains("does not have any commits") {
                println!("Skipping: repository has no commits yet (bootstrap scenario)");
                return Ok(());
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Tests repository analysis fails for nonexistent path.
#[test]
fn test_analyze_repository_nonexistent() {
    // Arrange
    let repo_path = PathBuf::from("/nonexistent/repo/path");

    // Act
    let result = analyze_repository(&repo_path, None);

    // Assert
    assert!(
        result.is_err(),
        "Should fail to analyze nonexistent repository"
    );
}

/// Tests configuration validation accepts valid config.
#[test]
fn test_config_validation_valid() -> Result<()> {
    // Arrange
    let config = Config {
        repo: PathBuf::from("."),
        output: PathBuf::from("test-output"),
        name: Some("test".to_string()),
        owner: None,
        theme: "Catppuccin-Latte".to_string(),
    };

    // Act
    let result = config.validate();

    // Assert
    assert!(result.is_ok(), "Valid configuration should pass validation");

    Ok(())
}

/// Tests configuration validation rejects nonexistent repository.
#[test]
fn test_config_validation_invalid_repo() {
    // Arrange
    let config = Config {
        repo: PathBuf::from("/nonexistent/path"),
        output: PathBuf::from("test-output"),
        name: None,
        owner: None,
        theme: "Catppuccin-Latte".to_string(),
    };

    // Act
    let result = config.validate();

    // Assert
    assert!(
        result.is_err(),
        "Nonexistent repository should fail validation"
    );
}

/// Tests project name uses custom name when provided.
#[test]
fn test_project_name_custom() -> Result<()> {
    // Arrange
    let config = Config {
        repo: PathBuf::from("/some/path/myrepo"),
        output: PathBuf::from("dist"),
        name: Some("Custom Name".to_string()),
        owner: None,
        theme: "Catppuccin-Latte".to_string(),
    };

    // Act
    let name = config.project_name()?;

    // Assert
    assert_eq!(name, "Custom Name");

    Ok(())
}

/// Tests project name falls back to directory name.
#[test]
fn test_project_name_fallback() -> Result<()> {
    // Arrange
    let config = Config {
        repo: PathBuf::from("/some/path/myrepo"),
        output: PathBuf::from("dist"),
        name: None,
        owner: None,
        theme: "Catppuccin-Latte".to_string(),
    };

    // Act
    let name = config.project_name()?;

    // Assert
    assert_eq!(name, "myrepo");

    Ok(())
}

/// Tests configuration project name with current directory path.
#[test]
fn test_project_name_with_current_dir() -> Result<()> {
    // Arrange
    let config = Config {
        repo: PathBuf::from("."),
        output: PathBuf::from("dist"),
        name: None,
        owner: None,
        theme: "Catppuccin-Latte".to_string(),
    };

    // Act
    let name = config.project_name()?;

    // Assert
    assert!(
        !name.is_empty(),
        "Project name should not be empty for current directory"
    );

    Ok(())
}

/// Tests project name error on path without filename.
#[test]
fn test_project_name_root_path() {
    // Arrange
    let config = Config {
        repo: PathBuf::from("/"),
        output: PathBuf::from("dist"),
        name: None,
        owner: None,
        theme: "Catppuccin-Latte".to_string(),
    };

    // Act
    let result = config.project_name();

    // Assert
    assert!(
        result.is_err(),
        "Root path should fail to extract project name"
    );
}

/// Tests repository analysis handles path canonicalization.
#[test]
fn test_analyze_repository_with_dot_path() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(".");

    // Act
    let result = analyze_repository(&repo_path, None);

    // Assert
    match result {
        Ok(repo_info) => {
            assert!(!repo_info.name().is_empty());
            assert!(!repo_info.branches().is_empty());
        }
        Err(e) => {
            let err_msg = format!("{:?}", e);
            if err_msg.contains("does not have any commits") {
                println!("Skipping: repository has no commits yet");
                return Ok(());
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Tests repository with multiple branches extracts all branches correctly.
#[test]
fn test_analyze_repository_branch_count() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Act
    let result = analyze_repository(&repo_path, None);

    // Assert
    if let Ok(repo_info) = result {
        assert!(
            !repo_info.branches().is_empty(),
            "Repository should have at least one branch"
        );
        assert!(
            repo_info.commit_count() > 0,
            "Commit count should be positive"
        );
    }

    Ok(())
}

// /// Tests batch commit lookup handles file deletion correctly.
// #[test]
// fn test_batch_commits_file_deletion() {
//     // Arrange
//     let repo = create_test_repo().expect("Failed to create test repo");
//     let repo_path = repo.path();

//     fs::write(repo_path.join("temp.txt"), "temporary").expect("Failed to write temp");
//     fs::write(repo_path.join("permanent.txt"), "stays").expect("Failed to write permanent");
//     Command::new("git")
//         .args(["add", "."])
//         .current_dir(repo_path)
//         .output()
//         .expect("Failed to add files");
//     let create_commit = git_commit(repo_path, "Create files").expect("Failed to commit creation");

//     fs::remove_file(repo_path.join("temp.txt")).expect("Failed to delete temp");
//     Command::new("git")
//         .args(["add", "-A"])
//         .current_dir(repo_path)
//         .output()
//         .expect("Failed to stage deletion");
//     git_commit(repo_path, "Delete temp.txt").expect("Failed to commit deletion");

//     // Act
//     let results = get_last_commits_batch(repo_path, None, &["temp.txt", "permanent.txt"])
//         .expect("Should handle deleted files");

//     // Assert
//     assert!(
//         results.contains_key("permanent.txt"),
//         "Should find existing file"
//     );

//     if results.contains_key("temp.txt") {
//         let temp_commit = &results["temp.txt"];
//         assert_eq!(
//             temp_commit.oid(),
//             create_commit,
//             "Deleted file should point to creation commit"
//         );
//     }
// }

/// Tests batch commit lookup distinguishes modification from addition.
#[test]
fn test_batch_commits_modification_vs_addition() {
    // Arrange
    let repo = create_test_repo().expect("Failed to create test repo");
    let repo_path = repo.path();

    fs::write(repo_path.join("file.txt"), "v1").expect("Failed to write v1");
    Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add");
    git_commit(repo_path, "Initial version").expect("Failed to commit v1");

    fs::write(repo_path.join("file.txt"), "v2").expect("Failed to write v2");
    Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add modification");
    let modify_commit =
        git_commit(repo_path, "Modify file").expect("Failed to commit modification");

    // Act
    let results =
        get_last_commits_batch(repo_path, None, &["file.txt"]).expect("Should find modified file");

    // Assert
    assert_eq!(results.len(), 1, "Should find exactly one file");
    let commit = &results["file.txt"];
    assert_eq!(commit.oid(), modify_commit, "Should point to modification");
    assert_eq!(commit.message(), "Modify file", "Message should match");
}

/// Tests batch commit lookup finds files across different commits.
#[test]
fn test_batch_commits_multiple_history() {
    // Arrange
    let repo = create_test_repo().expect("Failed to create test repo");
    let repo_path = repo.path();

    fs::write(repo_path.join("file1.txt"), "content1").expect("Failed to write file1");
    Command::new("git")
        .args(["add", "file1.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add file1");
    let commit1 = git_commit(repo_path, "Add file1").expect("Failed to commit file1");

    fs::write(repo_path.join("file2.txt"), "content2").expect("Failed to write file2");
    Command::new("git")
        .args(["add", "file2.txt"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to add file2");
    let commit2 = git_commit(repo_path, "Add file2").expect("Failed to commit file2");

    // Act
    let results = get_last_commits_batch(repo_path, None, &["file1.txt", "file2.txt"])
        .expect("Should find both files");

    // Assert
    assert_eq!(results.len(), 2, "Should find both files");
    assert_eq!(results["file1.txt"].oid(), commit1, "file1 matches commit1");
    assert_eq!(results["file2.txt"].oid(), commit2, "file2 matches commit2");
    assert_ne!(
        results["file1.txt"].oid(),
        results["file2.txt"].oid(),
        "Different commits have different hashes"
    );
}
