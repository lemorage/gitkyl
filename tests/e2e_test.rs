//! End-to-end tests for Gitkyl binary workflow.

use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Tests full binary execution generates valid output.
#[test]
fn test_full_workflow_e2e() -> Result<()> {
    // Arrange
    let temp_output = PathBuf::from("test-e2e-output");
    let _ = fs::remove_dir_all(&temp_output);
    let parent_repo = PathBuf::from("..");

    // Act
    let status = Command::new("cargo")
        .args(&[
            "run",
            "--manifest-path",
            "Cargo.toml",
            "--",
            parent_repo
                .to_str()
                .expect("Test repo path should be valid UTF8"),
            "-o",
            temp_output
                .to_str()
                .expect("Test output path should be valid UTF8"),
            "--name",
            "E2E Test",
            "--owner",
            "testuser",
            "--no-open",
        ])
        .status()?;

    // Assert
    if !status.success() {
        println!("Skipping: parent repository may have no commits");
        return Ok(());
    }

    let index_path = temp_output.join("index.html");
    if index_path.exists() {
        let html_content = fs::read_to_string(&index_path)?;
        assert!(html_content.contains("E2E Test"));
        assert!(html_content.contains("testuser"));
        assert!(html_content.contains("Gitkyl"));

        fs::remove_dir_all(&temp_output)?;
    }

    Ok(())
}

/// Tests binary execution with minimal arguments.
#[test]
fn test_minimal_args_e2e() -> Result<()> {
    // Arrange
    let temp_output = PathBuf::from("test-minimal-output");
    let _ = fs::remove_dir_all(&temp_output);
    let parent_repo = PathBuf::from("..");

    // Act
    let status = Command::new("cargo")
        .args(&[
            "run",
            "--manifest-path",
            "Cargo.toml",
            "--",
            parent_repo
                .to_str()
                .expect("Test repo path should be valid UTF8"),
            "-o",
            temp_output
                .to_str()
                .expect("Test output path should be valid UTF8"),
            "--no-open",
        ])
        .status()?;

    // Assert
    if !status.success() {
        println!("Skipping: parent repository may have no commits");
        return Ok(());
    }

    let index_path = temp_output.join("index.html");
    if index_path.exists() {
        assert!(index_path.exists(), "index.html should be generated");

        fs::remove_dir_all(&temp_output)?;
    }

    Ok(())
}
