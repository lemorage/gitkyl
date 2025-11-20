//! Workflow integration tests for Gitkyl.
//!
//! Tests complete pipelines from listing files through HTML generation.

use anyhow::{Context, Result};
use gitkyl::{generate_blob_page, highlight, list_files, read_blob};
use std::path::{Path, PathBuf};

/// Tests complete workflow from listing files to reading blob content.
///
/// This tests the actual user workflow: list all files in repository,
/// then read content for a known file. Verifies the integration between
/// list_files and read_blob functions.
#[test]
fn test_workflow_list_files_to_read_blob() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ref_name = None;

    // Act: list files from repository
    let files = match list_files(&repo_path, ref_name) {
        Ok(f) => f,
        Err(e) => {
            if format!("{:?}", e).contains("does not have any commits") {
                println!("Skipping: repository has no commits yet");
                return Ok(());
            }
            return Err(e);
        }
    };

    // Assert: repository contains expected files
    assert!(!files.is_empty(), "Repository should contain files");

    // Act: read blob for first Rust file found
    let rust_file = files
        .iter()
        .find(|entry| {
            entry
                .path()
                .and_then(|p| p.extension())
                .and_then(|ext| ext.to_str())
                == Some("rs")
        })
        .expect("Repository should contain at least one Rust file");

    let file_path = rust_file.path().expect("File should have valid path");
    let blob_content = read_blob(&repo_path, ref_name, file_path)?;

    // Assert: blob content is valid UTF8 Rust source
    let content_str =
        String::from_utf8(blob_content).expect("Rust source file should be valid UTF8");
    assert!(
        !content_str.is_empty(),
        "Rust file content should not be empty"
    );

    Ok(())
}

/// Tests complete workflow from reading blob to syntax highlighting.
///
/// Verifies that blob content from git repository can be successfully
/// highlighted using tree-sitter, covering the read_blob to highlight pipeline.
#[test]
fn test_workflow_read_blob_to_highlight() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let file_path = Path::new("src/lib.rs");
    let ref_name = None;

    // Act: read blob content
    let blob_content = match read_blob(&repo_path, ref_name, file_path) {
        Ok(content) => content,
        Err(e) => {
            if format!("{:?}", e).contains("does not have any commits") {
                println!("Skipping: repository has no commits yet");
                return Ok(());
            }
            return Err(e);
        }
    };

    let content_str =
        String::from_utf8(blob_content).context("Blob content should be valid UTF8")?;

    // Act: apply syntax highlighting
    let highlighted_lines = highlight(&content_str, file_path)?;
    let highlighted = highlighted_lines.join("");

    // Assert: highlighting produces HTML with inline styles
    assert!(
        highlighted.contains("style="),
        "Highlighted output should contain inline styles"
    );
    assert!(
        highlighted.len() >= content_str.len(),
        "Highlighted output should be at least as long as input (due to HTML tags)"
    );

    Ok(())
}

/// Tests complete workflow from listing to blob page generation.
///
/// Exercises the full pipeline: list_files → read_blob → highlight → generate_blob_page.
/// This is the actual end-to-end workflow a user would execute.
#[test]
fn test_workflow_full_pipeline_rust_file() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ref_name = "HEAD";

    // Act: list all files
    let files = match list_files(&repo_path, Some(ref_name)) {
        Ok(f) => f,
        Err(e) => {
            if format!("{:?}", e).contains("does not have any commits") {
                println!("Skipping: repository has no commits yet");
                return Ok(());
            }
            return Err(e);
        }
    };

    // Act: find first Rust file
    let rust_file = files
        .iter()
        .find(|entry| {
            entry
                .path()
                .and_then(|p| p.extension())
                .and_then(|ext| ext.to_str())
                == Some("rs")
        })
        .expect("Repository should contain Rust files");

    let file_path = rust_file.path().expect("File should have valid path");

    // Act: generate complete blob page
    let blob_page = generate_blob_page(
        &repo_path,
        ref_name,
        file_path,
        "test-repo",
        "Catppuccin-Latte",
    )?;
    let html = blob_page.into_string();

    // Assert: generated HTML contains expected elements
    assert!(html.contains("<!DOCTYPE html>"), "Should be valid HTML5");
    assert!(
        html.contains("blob-container"),
        "Should contain blob container class"
    );
    assert!(html.contains("line-number"), "Should contain line numbers");
    assert!(
        html.contains("code-content"),
        "Should contain code content section"
    );
    assert!(
        html.contains(file_path.to_str().unwrap()),
        "Should contain file path in breadcrumb"
    );
    assert!(
        html.contains("test-repo"),
        "Should contain repository name in breadcrumb"
    );

    Ok(())
}

/// Tests blob page generation handles multiple file types correctly.
///
/// Verifies that the generator correctly processes different file extensions,
/// applying appropriate highlighting or falling back to plain text.
#[test]
fn test_workflow_multiple_file_types() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ref_name = "HEAD";

    // Test Rust file (supported syntax highlighting)
    let rust_result = generate_blob_page(
        &repo_path,
        ref_name,
        Path::new("src/lib.rs"),
        "test-repo",
        "Catppuccin-Latte",
    );
    match rust_result {
        Ok(html) => {
            let html_str = html.into_string();
            assert!(
                html_str.contains("hl-keyword") || html_str.contains("line-number"),
                "Rust file should have syntax highlighting or at minimum line numbers"
            );
            assert!(
                html_str.contains("test-repo"),
                "Should contain repository name in breadcrumb"
            );
        }
        Err(e) => {
            if format!("{:?}", e).contains("does not have any commits") {
                println!("Skipping: repository has no commits yet");
                return Ok(());
            }
            return Err(e);
        }
    }

    // Test TOML file (unsupported, should fallback to plain text)
    let toml_result = generate_blob_page(
        &repo_path,
        ref_name,
        Path::new("Cargo.toml"),
        "test-repo",
        "Catppuccin-Latte",
    );
    match toml_result {
        Ok(html) => {
            let html_str = html.into_string();
            assert!(html_str.contains("Cargo.toml"), "Should contain filename");
            assert!(
                html_str.contains("blob-container"),
                "Should have standard blob structure"
            );
            assert!(
                html_str.contains("test-repo"),
                "Should contain repository name in breadcrumb"
            );
        }
        Err(e) => {
            if !format!("{:?}", e).contains("does not have any commits") {
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Tests error handling when file does not exist at specified reference.
///
/// Verifies that the workflow gracefully handles missing files rather than
/// panicking or producing corrupt output.
#[test]
fn test_workflow_error_nonexistent_file() {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ref_name = "HEAD";
    let invalid_path = Path::new("this/file/does/not/exist.rs");

    // Act
    let result = generate_blob_page(
        &repo_path,
        ref_name,
        invalid_path,
        "test-repo",
        "Catppuccin-Latte",
    );

    // Assert
    assert!(result.is_err(), "Should return error for nonexistent file");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("not found") || error_msg.contains("Failed"),
        "Error message should indicate file not found"
    );
}

/// Tests error handling when reference does not exist.
///
/// Verifies that invalid git references are caught early and produce
/// meaningful error messages.
#[test]
fn test_workflow_error_invalid_reference() {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let invalid_ref = "refs/heads/this_branch_absolutely_does_not_exist_12345";
    let file_path = Path::new("src/lib.rs");

    // Act
    let result = generate_blob_page(
        &repo_path,
        invalid_ref,
        file_path,
        "test-repo",
        "Catppuccin-Latte",
    );

    // Assert
    assert!(result.is_err(), "Should return error for invalid reference");
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(
        error_msg.contains("reference") || error_msg.contains("Failed"),
        "Error message should indicate reference problem"
    );
}

/// Tests highlighting with HTML special characters in source code.
///
/// Verifies that the highlight to generate_blob_page pipeline correctly
/// escapes HTML entities to prevent XSS and rendering issues.
#[test]
fn test_workflow_html_escaping_in_code() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ref_name = "HEAD";

    // Act: find highlight.rs which contains HTML escaping logic
    let result = generate_blob_page(
        &repo_path,
        ref_name,
        Path::new("src/highlight.rs"),
        "test-repo",
        "Catppuccin-Latte",
    );

    match result {
        Ok(html) => {
            let html_str = html.into_string();

            // Assert: HTML entities should be escaped in code content
            // The source file contains escape_html function with < > & characters
            // These should appear as &lt; &gt; &amp; in the final HTML
            assert!(
                html_str.contains("&lt;")
                    || html_str.contains("&gt;")
                    || html_str.contains("&amp;"),
                "HTML special characters in code should be escaped"
            );

            // Assert: basic HTML structure should be intact
            assert!(
                html_str.contains("<!DOCTYPE html>"),
                "Should have valid HTML structure"
            );

            // Assert: repository name in breadcrumb
            assert!(
                html_str.contains("test-repo"),
                "Should contain repository name in breadcrumb"
            );
        }
        Err(e) => {
            if format!("{:?}", e).contains("does not have any commits") {
                println!("Skipping: repository has no commits yet");
                return Ok(());
            }
            return Err(e);
        }
    }

    Ok(())
}

/// Tests blob page generation preserves file path structure in breadcrumbs.
///
/// Verifies that nested file paths are correctly split and displayed
/// in the breadcrumb navigation component.
#[test]
fn test_workflow_breadcrumb_generation() -> Result<()> {
    // Arrange
    let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ref_name = "HEAD";
    let nested_path = Path::new("src/generators.rs");

    // Act
    let result = generate_blob_page(
        &repo_path,
        ref_name,
        nested_path,
        "test-repo",
        "Catppuccin-Latte",
    );

    match result {
        Ok(html) => {
            let html_str = html.into_string();

            // Assert: breadcrumb components should be present
            assert!(html_str.contains("src"), "Should contain directory name");
            assert!(
                html_str.contains("generators.rs"),
                "Should contain filename"
            );
            assert!(
                html_str.contains("breadcrumb"),
                "Should have breadcrumb class"
            );
            assert!(
                html_str.contains("breadcrumb-separator"),
                "Should have path separators"
            );
            assert!(
                html_str.contains("test-repo"),
                "Should contain repository name in breadcrumb"
            );
        }
        Err(e) => {
            if format!("{:?}", e).contains("does not have any commits") {
                println!("Skipping: repository has no commits yet");
                return Ok(());
            }
            return Err(e);
        }
    }

    Ok(())
}
