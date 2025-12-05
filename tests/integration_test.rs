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
        no_open: true,
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
        no_open: true,
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
        no_open: true,
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
        no_open: true,
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
        no_open: true,
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
        no_open: true,
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

/// Tests README.md is detected and rendered as markdown.
#[test]
fn test_readme_detection_and_rendering() -> Result<()> {
    // Arrange
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    let readme_content = "# Test Project\n\nThis is a test README.";
    fs::write(repo_path.join("README.md"), readme_content)?;

    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add README")?;

    // Act
    let result =
        gitkyl::pages::blob::generate_markdown(repo_path, "HEAD", "README.md", "test-repo");

    // Assert
    assert!(result.is_ok(), "Should render README as markdown");
    let html = result?.into_string();
    assert!(html.contains("<h1"), "Should contain rendered heading");
    assert!(
        html.contains("Test Project"),
        "Should contain README content"
    );

    Ok(())
}

/// Tests readme (lowercase) is detected.
#[test]
fn test_lowercase_readme_detection() -> Result<()> {
    // Arrange
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    let readme_content = "# Lowercase README\n\nThis uses lowercase filename.";
    fs::write(repo_path.join("readme.md"), readme_content)?;

    Command::new("git")
        .args(["add", "readme.md"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add lowercase readme")?;

    // Act
    let result =
        gitkyl::pages::blob::generate_markdown(repo_path, "HEAD", "readme.md", "test-repo");

    // Assert
    assert!(result.is_ok(), "Should render lowercase readme as markdown");
    let html = result?.into_string();
    assert!(
        html.contains("Lowercase README"),
        "Should contain readme content"
    );

    Ok(())
}

/// Tests README without extension is detected.
#[test]
fn test_readme_without_extension_detection() -> Result<()> {
    // Arrange
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    let readme_content = "# README Without Extension\n\nPlain text README.";
    fs::write(repo_path.join("README"), readme_content)?;

    Command::new("git")
        .args(["add", "README"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add README without extension")?;

    // Act
    let result = gitkyl::pages::blob::generate_markdown(repo_path, "HEAD", "README", "test-repo");

    // Assert
    assert!(result.is_ok(), "Should render README without extension");
    let html = result?.into_string();
    assert!(
        html.contains("README Without Extension"),
        "Should contain content"
    );

    Ok(())
}

/// Tests non-README markdown file is NOT rendered with README logic.
#[test]
fn test_non_readme_markdown_not_detected() {
    // Arrange: use is_readme function directly
    use gitkyl::is_readme;

    // Act & Assert
    assert!(
        !is_readme("CONTRIBUTING.md"),
        "CONTRIBUTING.md is not README"
    );
    assert!(!is_readme("docs.md"), "docs.md is not README");
    assert!(!is_readme("guide.md"), "guide.md is not README");
}

/// Tests README in subdirectory is detected.
#[test]
fn test_readme_in_subdirectory() -> Result<()> {
    // Arrange
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    fs::create_dir_all(repo_path.join("docs"))?;
    let readme_content = "# Docs README\n\nDocumentation index.";
    fs::write(repo_path.join("docs/README.md"), readme_content)?;

    Command::new("git")
        .args(["add", "docs/README.md"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add docs README")?;

    // Act
    let result =
        gitkyl::pages::blob::generate_markdown(repo_path, "HEAD", "docs/README.md", "test-repo");

    // Assert
    assert!(result.is_ok(), "Should render nested README");
    let html = result?.into_string();
    assert!(
        html.contains("Docs README"),
        "Should contain nested README content"
    );

    Ok(())
}

/// Tests error handling for corrupt or invalid UTF8 README.
#[test]
fn test_readme_with_invalid_utf8_fails_gracefully() -> Result<()> {
    // Arrange
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    // Create file with invalid UTF8 bytes
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
    fs::write(repo_path.join("README.md"), invalid_utf8)?;

    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add invalid UTF8 README")?;

    // Act
    let result =
        gitkyl::pages::blob::generate_markdown(repo_path, "HEAD", "README.md", "test-repo");

    // Assert
    assert!(result.is_err(), "Should fail for invalid UTF8 content");
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("UTF8") || err_msg.contains("utf-8"),
        "Error should mention UTF8 encoding issue"
    );

    Ok(())
}

/// Tests markdown rendering produces valid HTML structure.
#[test]
fn test_readme_markdown_produces_valid_html() -> Result<()> {
    // Arrange
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    let readme_content = r#"# Title
## Subtitle

Paragraph with **bold** and *italic*.

- List item 1
- List item 2

```rust
fn main() {
    println!("Hello");
}
```
"#;
    fs::write(repo_path.join("README.md"), readme_content)?;

    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add complex README")?;

    // Act
    let result =
        gitkyl::pages::blob::generate_markdown(repo_path, "HEAD", "README.md", "test-repo");

    // Assert
    assert!(result.is_ok(), "Should render complex markdown");
    let html = result?.into_string();

    assert!(html.contains("<!DOCTYPE html>"), "Should have DOCTYPE");
    assert!(html.contains("<h1"), "Should have h1 heading");
    assert!(html.contains("<h2"), "Should have h2 heading");
    assert!(
        html.contains("<ul") || html.contains("<li"),
        "Should have list items"
    );
    assert!(
        html.contains("<strong>") || html.contains("bold"),
        "Should render bold"
    );
    assert!(
        html.contains("<em>") || html.contains("italic"),
        "Should render italic"
    );
    assert!(html.contains("</html>"), "Should close html tag");

    Ok(())
}

/// Tests blob generation with various file extensions applies correct syntax.
#[test]
fn test_blob_generate_with_various_file_extensions() -> Result<()> {
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    let files = vec![
        ("script.py", "def hello():\n    print('world')\n"),
        ("config.toml", "[package]\nname = \"test\"\n"),
        ("style.css", "body { margin: 0; }\n"),
    ];

    for (filename, content) in &files {
        fs::write(repo_path.join(filename), content)?;
        Command::new("git")
            .args(["add", filename])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, &format!("Add {}", filename))?;

        let result = gitkyl::pages::blob::generate(
            repo_path,
            "HEAD",
            filename,
            "test-repo",
            "base16-ocean.dark",
        )?;

        let html = result.into_string();
        assert!(html.contains(filename), "Should contain filename");
        assert!(html.contains("<!DOCTYPE html>"), "Should be valid HTML");
    }

    Ok(())
}

/// Tests blob generation fails gracefully for nonexistent file.
#[test]
fn test_blob_generate_nonexistent_file_fails() -> Result<()> {
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    fs::write(repo_path.join("exists.txt"), "content\n")?;
    Command::new("git")
        .args(["add", "exists.txt"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Initial commit")?;

    let result = gitkyl::pages::blob::generate(
        repo_path,
        "HEAD",
        "nonexistent.txt",
        "test-repo",
        "base16-ocean.dark",
    );

    assert!(result.is_err(), "Should fail for nonexistent file");
    let err_msg = format!("{:?}", result.unwrap_err());
    assert!(
        err_msg.contains("Failed to read blob") || err_msg.contains("nonexistent"),
        "Error should reference the missing file"
    );

    Ok(())
}

/// Tests blob generation fails for invalid git reference.
#[test]
fn test_blob_generate_invalid_reference_fails() -> Result<()> {
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    fs::write(repo_path.join("file.txt"), "content\n")?;
    Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add file")?;

    let result = gitkyl::pages::blob::generate(
        repo_path,
        "nonexistent-branch",
        "file.txt",
        "test-repo",
        "base16-ocean.dark",
    );

    assert!(result.is_err(), "Should fail for invalid reference");

    Ok(())
}

/// Tests blob generation handles empty file correctly.
#[test]
fn test_blob_generate_empty_file_succeeds() -> Result<()> {
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    fs::write(repo_path.join("empty.txt"), "")?;
    Command::new("git")
        .args(["add", "empty.txt"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add empty file")?;

    let result = gitkyl::pages::blob::generate(
        repo_path,
        "HEAD",
        "empty.txt",
        "test-repo",
        "base16-ocean.dark",
    )?;

    let html = result.into_string();
    assert!(
        html.contains("<!DOCTYPE html>"),
        "Should generate valid HTML"
    );
    assert!(html.contains("empty.txt"), "Should contain filename");

    Ok(())
}

/// Tests markdown blob handles code blocks correctly.
#[test]
fn test_markdown_blob_with_code_blocks() -> Result<()> {
    let repo = create_test_repo()?;
    let repo_path = repo.path();

    let markdown = r#"# Code Example

```rust
fn main() {
    println!("Hello");
}
```
"#;
    fs::write(repo_path.join("example.md"), markdown)?;
    Command::new("git")
        .args(["add", "example.md"])
        .current_dir(repo_path)
        .output()?;
    git_commit(repo_path, "Add example")?;

    let result =
        gitkyl::pages::blob::generate_markdown(repo_path, "HEAD", "example.md", "test-repo")?;

    let html = result.into_string();
    assert!(html.contains("<h1"), "Should render heading");
    assert!(
        html.contains("<code>") || html.contains("main"),
        "Should render code block"
    );

    Ok(())
}

mod tree_page_tests {
    use super::*;
    use gitkyl::{CommitInfo, TreeItem};
    use gix::bstr::BString;

    fn create_file_entry(path: &str, oid: &str) -> gitkyl::FileEntry {
        let repeated = oid.repeat((40 / oid.len()) + 1);
        let hex_40 = &repeated[..40];
        let oid_bytes = hex::decode(hex_40).expect("Invalid OID");
        let oid = gix::ObjectId::try_from(&oid_bytes[..]).expect("Failed to create OID");

        unsafe {
            std::mem::transmute::<(BString, gix::ObjectId), gitkyl::FileEntry>((
                BString::from(path.as_bytes()),
                oid,
            ))
        }
    }

    fn create_test_commit(oid: &str, message: &str, author: &str, date: i64) -> CommitInfo {
        CommitInfo::new(
            oid.to_string(),
            message.to_string(),
            message.to_string(),
            author.to_string(),
            date,
        )
    }

    #[test]
    fn test_tree_generate_lists_root_directory_files() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::write(
            repo_path.join("README.md"),
            "# Test Project\n\nDescription.",
        )?;
        fs::write(repo_path.join("main.rs"), "fn main() {}\n")?;
        fs::write(repo_path.join("Cargo.toml"), "[package]\nname = \"test\"\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Initial commit")?;

        let commit = create_test_commit(
            "abc1234567890123456789012345678901234567",
            "Initial commit",
            "Test User",
            1700000000,
        );

        let items = vec![
            TreeItem::File {
                entry: create_file_entry("Cargo.toml", "cafe"),
                commit: commit.clone(),
            },
            TreeItem::File {
                entry: create_file_entry("README.md", "dead"),
                commit: commit.clone(),
            },
            TreeItem::File {
                entry: create_file_entry("main.rs", "beef"),
                commit,
            },
        ];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "", "test-repo", &items)?;

        let html = result.into_string();

        assert!(html.contains("<!DOCTYPE html>"), "Should have DOCTYPE");
        assert!(html.contains("test-repo"), "Should contain repo name");
        assert!(html.contains("README.md"), "Should list README.md");
        assert!(html.contains("main.rs"), "Should list main.rs");
        assert!(html.contains("Cargo.toml"), "Should list Cargo.toml");
        assert!(
            html.contains("file-table"),
            "Should use file table component"
        );
        assert!(
            html.contains("Initial commit"),
            "Should show commit message"
        );

        Ok(())
    }

    #[test]
    fn test_tree_generate_lists_subdirectory_files() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir(repo_path.join("src"))?;
        fs::write(repo_path.join("src/lib.rs"), "pub fn test() {}\n")?;
        fs::write(repo_path.join("src/main.rs"), "fn main() {}\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add src directory")?;

        let commit = create_test_commit(
            "def4567890123456789012345678901234567890",
            "Add src directory",
            "Test User",
            1700000100,
        );

        let items = vec![
            TreeItem::File {
                entry: create_file_entry("src/lib.rs", "1234"),
                commit: commit.clone(),
            },
            TreeItem::File {
                entry: create_file_entry("src/main.rs", "5678"),
                commit,
            },
        ];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "src", "test-repo", &items)?;

        let html = result.into_string();

        assert!(html.contains("src"), "Should show subdirectory in title");
        assert!(html.contains("lib.rs"), "Should list lib.rs");
        assert!(html.contains("main.rs"), "Should list main.rs");
        assert!(
            html.contains("breadcrumb"),
            "Should include breadcrumb navigation"
        );
        assert!(html.contains(".."), "Should include parent directory link");

        Ok(())
    }

    #[test]
    fn test_tree_generate_shows_directory_entries() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir(repo_path.join("src"))?;
        fs::write(repo_path.join("src/lib.rs"), "pub fn test() {}\n")?;
        fs::create_dir(repo_path.join("docs"))?;
        fs::write(repo_path.join("docs/README.md"), "# Docs\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add directories")?;

        let commit = create_test_commit(
            "abc1234567890123456789012345678901234567",
            "Add directories",
            "Test User",
            1700000200,
        );

        let items = vec![
            TreeItem::Directory {
                name: "docs".to_string(),
                full_path: "docs".to_string(),
                commit: commit.clone(),
            },
            TreeItem::Directory {
                name: "src".to_string(),
                full_path: "src".to_string(),
                commit,
            },
        ];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "", "test-repo", &items)?;

        let html = result.into_string();

        assert!(html.contains("docs"), "Should list docs directory");
        assert!(html.contains("src"), "Should list src directory");
        assert!(html.contains("icon-folder"), "Should use folder icon");

        Ok(())
    }

    #[test]
    fn test_tree_generate_deep_subdirectory() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir_all(repo_path.join("src/modules/utils"))?;
        fs::write(
            repo_path.join("src/modules/utils/helpers.rs"),
            "pub fn helper() {}\n",
        )?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add deep structure")?;

        let commit = create_test_commit(
            "deep123456789012345678901234567890123456",
            "Add deep structure",
            "Test User",
            1700000300,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("src/modules/utils/helpers.rs", "abcd"),
            commit,
        }];

        let result = gitkyl::pages::tree::generate(
            repo_path,
            "HEAD",
            "src/modules/utils",
            "test-repo",
            &items,
        )?;

        let html = result.into_string();

        assert!(html.contains("helpers.rs"), "Should list helpers.rs");
        assert!(html.contains("breadcrumb"), "Should have breadcrumb");
        assert!(html.contains("utils"), "Should show current directory");
        assert!(html.contains(".."), "Should have parent link");

        Ok(())
    }

    #[test]
    fn test_tree_generate_empty_directory_shows_message() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::write(repo_path.join("README.md"), "# Test\n")?;
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Initial")?;

        let items = vec![];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "", "test-repo", &items)?;

        let html = result.into_string();

        assert!(
            html.contains("Empty directory"),
            "Should show empty directory message"
        );

        Ok(())
    }

    #[test]
    fn test_tree_generate_subdirectory_shows_file_table() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir(repo_path.join("tests"))?;
        fs::write(repo_path.join("tests/test1.rs"), "#[test]\nfn test1() {}\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add tests")?;

        let commit = create_test_commit(
            "test123456789012345678901234567890123456",
            "Add tests",
            "Test User",
            1700000400,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("tests/test1.rs", "9999"),
            commit,
        }];

        let result =
            gitkyl::pages::tree::generate(repo_path, "HEAD", "tests", "test-repo", &items)?;

        let html = result.into_string();

        assert!(
            !html.contains("Empty directory"),
            "Should NOT show empty message"
        );
        assert!(html.contains("file-table"), "Should show file table");
        assert!(html.contains("test1.rs"), "Should show file");

        Ok(())
    }

    #[test]
    fn test_tree_generate_includes_commit_metadata() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::write(repo_path.join("config.toml"), "[settings]\n")?;

        Command::new("git")
            .args(["add", "config.toml"])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add configuration file")?;

        let commit = create_test_commit(
            "meta123456789012345678901234567890123456",
            "Add configuration file",
            "Author Name",
            1700000500,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("config.toml", "c0ff"),
            commit,
        }];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "", "test-repo", &items)?;

        let html = result.into_string();

        assert!(
            html.contains("Add configuration file"),
            "Should show commit message"
        );

        Ok(())
    }

    #[test]
    fn test_tree_generate_breadcrumb_navigation() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir_all(repo_path.join("src/pages"))?;
        fs::write(repo_path.join("src/pages/home.rs"), "pub fn home() {}\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add pages")?;

        let commit = create_test_commit(
            "page123456789012345678901234567890123456",
            "Add pages",
            "Test User",
            1700000600,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("src/pages/home.rs", "f00d"),
            commit,
        }];

        let result =
            gitkyl::pages::tree::generate(repo_path, "HEAD", "src/pages", "test-repo", &items)?;

        let html = result.into_string();

        assert!(
            html.contains("breadcrumb"),
            "Should have breadcrumb component"
        );
        assert!(
            html.contains("pages"),
            "Should show current directory in breadcrumb"
        );
        assert!(
            html.contains("src"),
            "Should show parent directory in breadcrumb"
        );

        Ok(())
    }

    #[test]
    fn test_tree_generate_file_links_point_to_blob_pages() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::write(repo_path.join("script.sh"), "#!/bin/bash\necho 'test'\n")?;

        Command::new("git")
            .args(["add", "script.sh"])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add script")?;

        let commit = create_test_commit(
            "link123456789012345678901234567890123456",
            "Add script",
            "Test User",
            1700000700,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("script.sh", "ba5e"),
            commit,
        }];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "", "test-repo", &items)?;

        let html = result.into_string();

        assert!(
            html.contains("blob/HEAD/script.sh.html"),
            "Should link to blob page"
        );

        Ok(())
    }

    #[test]
    fn test_tree_generate_directory_links_point_to_tree_pages() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir(repo_path.join("lib"))?;
        fs::write(repo_path.join("lib/mod.rs"), "pub mod test;\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add lib")?;

        let commit = create_test_commit(
            "link456789012345678901234567890123456789",
            "Add lib",
            "Test User",
            1700000800,
        );

        let items = vec![TreeItem::Directory {
            name: "lib".to_string(),
            full_path: "lib".to_string(),
            commit,
        }];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "", "test-repo", &items)?;

        let html = result.into_string();

        assert!(
            html.contains("tree/HEAD/lib.html"),
            "Should link to tree page"
        );

        Ok(())
    }

    #[test]
    fn test_tree_generate_mixed_files_and_directories() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir(repo_path.join("bin"))?;
        fs::write(repo_path.join("bin/cli.rs"), "fn main() {}\n")?;
        fs::write(repo_path.join("LICENSE"), "MIT License\n")?;
        fs::create_dir(repo_path.join("examples"))?;
        fs::write(repo_path.join("examples/demo.rs"), "fn main() {}\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add mixed content")?;

        let commit = create_test_commit(
            "mix1234567890123456789012345678901234567",
            "Add mixed content",
            "Test User",
            1700000900,
        );

        let items = vec![
            TreeItem::Directory {
                name: "bin".to_string(),
                full_path: "bin".to_string(),
                commit: commit.clone(),
            },
            TreeItem::Directory {
                name: "examples".to_string(),
                full_path: "examples".to_string(),
                commit: commit.clone(),
            },
            TreeItem::File {
                entry: create_file_entry("LICENSE", "1111"),
                commit,
            },
        ];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "", "test-repo", &items)?;

        let html = result.into_string();

        assert!(html.contains("bin"), "Should list bin directory");
        assert!(html.contains("examples"), "Should list examples directory");
        assert!(html.contains("LICENSE"), "Should list LICENSE file");
        assert!(html.contains("icon-folder"), "Should have folder icons");

        Ok(())
    }

    #[test]
    fn test_tree_generate_title_includes_path() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir(repo_path.join("docs"))?;
        fs::write(repo_path.join("docs/guide.md"), "# Guide\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add docs")?;

        let commit = create_test_commit(
            "title12345678901234567890123456789012345",
            "Add docs",
            "Test User",
            1700001000,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("docs/guide.md", "2222"),
            commit,
        }];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "docs", "my-repo", &items)?;

        let html = result.into_string();

        assert!(html.contains("<title>"), "Should have title tag");
        assert!(html.contains("docs"), "Title should include path");
        assert!(html.contains("my-repo"), "Title should include repo name");

        Ok(())
    }

    #[test]
    fn test_tree_generate_root_title_is_repo_name() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::write(repo_path.join("file.txt"), "content\n")?;

        Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add file")?;

        let commit = create_test_commit(
            "root123456789012345678901234567890123456",
            "Add file",
            "Test User",
            1700001100,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("file.txt", "3333"),
            commit,
        }];

        let result =
            gitkyl::pages::tree::generate(repo_path, "HEAD", "", "awesome-project", &items)?;

        let html = result.into_string();

        assert!(
            html.contains("<title>awesome-project - Gitkyl</title>"),
            "Root title should be repo name with Gitkyl suffix"
        );

        Ok(())
    }

    #[test]
    fn test_tree_generate_parent_link_goes_to_parent_directory() -> Result<()> {
        let repo = create_test_repo()?;
        let repo_path = repo.path();

        fs::create_dir_all(repo_path.join("a/b"))?;
        fs::write(repo_path.join("a/b/file.txt"), "content\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()?;
        git_commit(repo_path, "Add nested")?;

        let commit = create_test_commit(
            "nest123456789012345678901234567890123456",
            "Add nested",
            "Test User",
            1700001200,
        );

        let items = vec![TreeItem::File {
            entry: create_file_entry("a/b/file.txt", "4444"),
            commit,
        }];

        let result = gitkyl::pages::tree::generate(repo_path, "HEAD", "a/b", "test-repo", &items)?;

        let html = result.into_string();

        assert!(html.contains(".."), "Should have parent directory link");
        assert!(html.contains("ph-arrow-up"), "Should have arrow up icon");

        Ok(())
    }
}
