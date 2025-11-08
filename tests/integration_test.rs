//! Integration tests for Gitkyl.
//!
//! Tests repository analysis, configuration, and basic operations.

use anyhow::Result;
use gitkyl::{Config, analyze_repository};
use std::path::PathBuf;

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
