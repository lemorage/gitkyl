use anyhow::{Context, Result};
use gitkyl::{CommitInfo, Config, FileEntry};
use maud::{DOCTYPE, Markup, html};
use std::fs;

/// Default limit for commits displayed on commit log page.
///
/// Limits display to 35 commits to balance page load time and commit
/// visibility. Repositories with extensive history should implement
/// pagination in future versions.
const DEFAULT_COMMIT_LIMIT: usize = 35;

/// Minimum number of branches required to display branch selector.
///
/// When repository has fewer branches than this threshold, shows static
/// branch badge instead of interactive selector to reduce visual noise.
const MIN_BRANCHES_FOR_SELECTOR: usize = 2;

/// Returns Phosphor icon class for file type detection.
///
/// Matches file paths to appropriate icon classes based on extension
/// and filename patterns. Icon colors are controlled via CSS classes.
///
/// # Arguments
///
/// * `path`: File path relative to repository root
///
/// # Returns
///
/// Phosphor icon class name and optional CSS modifier class for styling
fn get_file_icon_info(path: &str) -> (&'static str, Option<&'static str>) {
    use std::path::Path;

    if path.ends_with('/') {
        return ("ph-fill ph-folder", Some("icon-folder"));
    }

    let path_lower = path.to_lowercase();
    let file_name = Path::new(&path_lower)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if file_name.starts_with("readme") {
        return ("ph ph-info", Some("icon-readme"));
    }

    if let Some(ext) = Path::new(&path_lower).extension().and_then(|e| e.to_str()) {
        match ext {
            "rs" => ("ph ph-file-rs", Some("icon-rust")),
            "toml" | "yaml" | "yml" => ("ph ph-gear", Some("icon-config")),
            _ => ("ph ph-file", None),
        }
    } else {
        ("ph ph-file", None)
    }
}

/// Generate repository index page HTML
fn index_page(
    name: &str,
    owner: &Option<String>,
    default_branch: &str,
    branches: &[String],
    commit_count: usize,
    latest_commit: Option<&CommitInfo>,
    files_with_commits: &[(FileEntry, CommitInfo)],
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (name) " - Gitkyl" }
                script src="https://unpkg.com/@phosphor-icons/web" {}
                style { (include_str!("../assets/index.css")) }
            }
            body {
                div class="container" {
                    header class="repo-header" {
                        @if let Some(owner_name) = owner {
                            span class="repo-owner" { (owner_name) " / " }
                        }
                        h1 class="repo-name" { (name) }
                    }

                    main class="repo-card" {
                        div class="repo-controls" {
                            div class="commit-info-group" {
                                @if branches.len() >= MIN_BRANCHES_FOR_SELECTOR {
                                    div class="branch-selector" {
                                        i class="ph ph-git-branch" {}
                                        @for branch in branches {
                                            @if branch == default_branch {
                                                span class="branch-name branch-active" { (branch) }
                                            } @else {
                                                span class="branch-name" { (branch) }
                                            }
                                        }
                                        i class="ph ph-caret-down branch-caret" {}
                                    }
                                } @else {
                                    div class="branch-selector" {
                                        i class="ph ph-git-branch" {}
                                        span { (default_branch) }
                                    }
                                }

                                @if let Some(commit) = latest_commit {
                                    div class="commit-info-wrapper" {
                                        div class="commit-line" {
                                            span class="avatar-placeholder" {}
                                            span class="commit-message" { (commit.message()) }
                                        }
                                        div class="commit-meta" {
                                            span { (commit.author()) }
                                            span { "·" }
                                            code class="commit-hash" { (commit.short_oid()) }
                                            span { "·" }
                                            span { (format_relative_time(commit.date())) }
                                        }
                                    }
                                }
                            }

                            a href=(format!("commits/{}/index.html", default_branch)) class="history-link" {
                                i class="ph ph-clock-counter-clockwise" {}
                                " " (commit_count) " commits"
                            }
                        }

                        @if files_with_commits.is_empty() {
                            p class="empty-state" { "No files in this repository" }
                        } @else {
                            div class="file-table" {
                                @for (file, file_commit) in files_with_commits.iter().take(50) {
                                    @if let Some(path) = file.path()
                                        && let Some(path_str) = path.to_str() {
                                        @let (icon_class, icon_modifier) = get_file_icon_info(path_str);
                                        a href=(format!("blob/{}/{}.html", default_branch, path_str)) class="file-row" {
                                            div class="icon-box" {
                                                @if let Some(modifier) = icon_modifier {
                                                    i class=(format!("{} {}", icon_class, modifier)) {}
                                                } @else {
                                                    i class=(icon_class) {}
                                                }
                                            }
                                            div class="file-link" { (path_str) }
                                            div class="file-meta" { (format_relative_time(file_commit.date())) }
                                        }
                                    }
                                }
                                @if files_with_commits.len() > 50 {
                                    div class="file-count-note" {
                                        "Showing 50 of " (files_with_commits.len()) " files"
                                    }
                                }
                            }
                        }
                    }

                    footer {
                        p {
                            "Generated by "
                            a href="https://github.com/lemorage/gitkyl" target="_blank" { "Gitkyl" }
                        }
                    }
                }
            }
        }
    }
}

/// Formats Unix timestamp as human readable relative time.
fn format_relative_time(seconds: i64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let timestamp = UNIX_EPOCH + Duration::from_secs(seconds as u64);
    let now = SystemTime::now();

    if let Ok(duration) = now.duration_since(timestamp) {
        let secs = duration.as_secs();
        let minutes = secs / 60;
        let hours = secs / 3600;
        let days = secs / 86400;

        if minutes < 1 {
            return "just now".to_string();
        } else if minutes < 60 {
            return format!("{} min ago", minutes);
        } else if hours < 24 {
            return format!("{} hr ago", hours);
        } else if days < 7 {
            return format!("{} days ago", days);
        } else if days < 30 {
            return format!("{} weeks ago", days / 7);
        } else if days < 365 {
            return format!("{} months ago", days / 30);
        } else {
            return format!("{} years ago", days / 365);
        }
    }

    "unknown".to_string()
}

fn main() -> Result<()> {
    let config = Config::parse();
    config.validate().context("Invalid configuration")?;

    let repo_info = gitkyl::analyze_repository(&config.repo, config.owner.clone())
        .context("Failed to analyze repository")?;

    fs::create_dir_all(&config.output).context("Failed to create output directory")?;

    let latest_commit =
        gitkyl::list_commits(&config.repo, Some(repo_info.default_branch()), Some(1))
            .ok()
            .and_then(|commits| commits.into_iter().next());

    let files =
        gitkyl::list_files(&config.repo, Some(repo_info.default_branch())).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to list files: {:#}", e);
            vec![]
        });

    let mut files_with_commits = Vec::new();
    for file in files.iter().take(50) {
        if let Some(path) = file.path()
            && let Some(path_str) = path.to_str()
        {
            match gitkyl::get_file_last_commit(
                &config.repo,
                Some(repo_info.default_branch()),
                path_str,
            ) {
                Ok(commit) => {
                    files_with_commits.push((file.clone(), commit));
                }
                Err(e) => {
                    eprintln!("Warning: Failed to get commit for {}: {:#}", path_str, e);
                }
            }
        }
    }

    let html = index_page(
        &config
            .project_name()
            .context("Failed to determine project name")?,
        &repo_info.owner().map(ToOwned::to_owned),
        repo_info.default_branch(),
        repo_info.branches(),
        repo_info.commit_count(),
        latest_commit.as_ref(),
        &files_with_commits,
    );

    let index_path = config.output.join("index.html");
    fs::write(&index_path, html.into_string())
        .with_context(|| format!("Failed to write index page to {}", index_path.display()))?;

    println!("Generated: {}", index_path.display());

    let commits = gitkyl::list_commits(
        &config.repo,
        Some(repo_info.default_branch()),
        Some(DEFAULT_COMMIT_LIMIT),
    )
    .context("Failed to list commits")?;

    let commits_html =
        gitkyl::generate_commits_page(&commits, repo_info.default_branch(), repo_info.name());

    let commits_dir = config
        .output
        .join("commits")
        .join(repo_info.default_branch());

    fs::create_dir_all(&commits_dir).context("Failed to create commits directory")?;

    let commits_path = commits_dir.join("index.html");
    fs::write(&commits_path, commits_html.into_string())
        .with_context(|| format!("Failed to write commits page to {}", commits_path.display()))?;

    println!(
        "Generated: {} ({} commits)",
        commits_path.display(),
        commits.len()
    );

    println!("Generating file pages...");

    let mut generated_count = 0;
    for entry in &files {
        if let Some(path) = entry.path() {
            if path.to_str().is_none() {
                eprintln!(
                    "Warning: Skipping file with invalid UTF-8 path: {}",
                    path.display()
                );
                continue;
            }

            match gitkyl::generate_blob_page(&config.repo, repo_info.default_branch(), path) {
                Ok(html) => {
                    let blob_path = config
                        .output
                        .join("blob")
                        .join(repo_info.default_branch())
                        .join(path)
                        .with_extension("html");

                    if let Some(parent) = blob_path.parent() {
                        fs::create_dir_all(parent).context("Failed to create blob directory")?;
                    }

                    fs::write(&blob_path, html.into_string()).with_context(|| {
                        format!("Failed to write blob page {}", blob_path.display())
                    })?;

                    generated_count += 1;
                }
                Err(e) => {
                    let err_msg = format!("{:?}", e);
                    if err_msg.contains("not a blob") || err_msg.contains("invalid UTF8") {
                        continue;
                    }
                    return Err(e).with_context(|| {
                        format!("Failed to generate blob page for {}", path.display())
                    });
                }
            }
        }
    }

    println!("Generated {} file pages", generated_count);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_index_page_basic() {
        // Arrange
        let name = "TestRepo";
        let owner = Some("testuser".to_string());
        let default_branch = "main";
        let branches = vec!["main".to_string(), "develop".to_string()];
        let commit_count = 42;
        let files_with_commits = vec![];

        // Act
        let html = index_page(
            name,
            &owner,
            default_branch,
            &branches,
            commit_count,
            None,
            &files_with_commits,
        );
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("TestRepo"), "Should contain repo name");
        assert!(html_string.contains("testuser"), "Should contain owner");
        assert!(
            html_string.contains("42 commits"),
            "Should contain commit count link"
        );
    }

    #[test]
    fn test_index_page_with_latest_commit() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_info =
            gitkyl::analyze_repository(&repo_path, None).expect("Should analyze repository");

        let latest_commit =
            gitkyl::list_commits(&repo_path, Some(repo_info.default_branch()), Some(1))
                .ok()
                .and_then(|commits| commits.into_iter().next());

        let files_with_commits = vec![];

        // Act
        let html = index_page(
            repo_info.name(),
            &repo_info.owner().map(ToOwned::to_owned),
            repo_info.default_branch(),
            repo_info.branches(),
            repo_info.commit_count(),
            latest_commit.as_ref(),
            &files_with_commits,
        );
        let html_string = html.into_string();

        // Assert
        assert!(
            html_string.contains("commit-info-group"),
            "Should have commit info group"
        );
        if let Some(commit) = latest_commit {
            assert!(
                html_string.contains(commit.short_oid()),
                "Should show commit hash"
            );
        }
    }

    #[test]
    fn test_format_relative_time_just_now() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_relative_time(now), "just now");
    }

    #[test]
    fn test_format_relative_time_minutes() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let five_min_ago = (now - 300) as i64;
        assert_eq!(format_relative_time(five_min_ago), "5 min ago");
    }

    #[test]
    fn test_commits_page_generation_workflow() {
        use tempfile;

        // Arrange
        let temp_dir = tempfile::tempdir().expect("Should create temp directory");
        let output = temp_dir.path();
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let repo_info =
            gitkyl::analyze_repository(&repo_path, None).expect("Should analyze repository");

        let commits = gitkyl::list_commits(&repo_path, Some(repo_info.default_branch()), Some(10))
            .expect("Should list commits");

        let html =
            gitkyl::generate_commits_page(&commits, repo_info.default_branch(), repo_info.name());

        let commits_dir = output.join("commits").join(repo_info.default_branch());
        fs::create_dir_all(&commits_dir).expect("Should create commits directory");

        let commits_path = commits_dir.join("index.html");
        fs::write(&commits_path, html.into_string()).expect("Should write commits page");

        // Assert
        assert!(commits_path.exists(), "Commits page should be created");

        let content = fs::read_to_string(&commits_path).expect("Should read commits page");

        assert!(
            content.contains("Commit History"),
            "Should contain commit log title"
        );
        assert!(
            content.contains(repo_info.default_branch()),
            "Should contain branch name"
        );
        assert!(commits.len() > 0, "Should have at least one commit");
    }

    #[test]
    fn test_index_page_with_file_table() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_info =
            gitkyl::analyze_repository(&repo_path, None).expect("Should analyze repository");

        let files = gitkyl::list_files(&repo_path, Some(repo_info.default_branch()))
            .expect("Should list files");

        let mut files_with_commits = Vec::new();
        for file in files.iter().take(3) {
            if let Some(path) = file.path()
                && let Some(path_str) = path.to_str()
            {
                if let Ok(commit) = gitkyl::get_file_last_commit(
                    &repo_path,
                    Some(repo_info.default_branch()),
                    path_str,
                ) {
                    files_with_commits.push((file.clone(), commit));
                }
            }
        }

        // Act
        let html = index_page(
            repo_info.name(),
            &repo_info.owner().map(ToOwned::to_owned),
            repo_info.default_branch(),
            repo_info.branches(),
            repo_info.commit_count(),
            None,
            &files_with_commits,
        );
        let html_string = html.into_string();

        // Assert
        assert!(
            html_string.contains("file-table"),
            "Should contain file table"
        );
        assert!(html_string.contains("file-row"), "Should contain file rows");
        assert!(
            html_string.contains("file-link"),
            "Should contain file link"
        );
        assert!(
            html_string.contains("file-meta"),
            "Should contain file metadata"
        );
    }

    #[test]
    fn test_index_page_file_table_structure() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let file = gitkyl::list_files(&repo_path, None)
            .expect("Should list files")
            .into_iter()
            .next()
            .expect("Should have at least one file");

        let path_str = file
            .path()
            .and_then(|p| p.to_str())
            .expect("Should have valid path");
        let commit =
            gitkyl::get_file_last_commit(&repo_path, None, path_str).expect("Should get commit");

        let files_with_commits = vec![(file, commit)];

        // Act
        let html = index_page(
            "test",
            &None,
            "main",
            &["main".to_string()],
            1,
            None,
            &files_with_commits,
        );
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("file-link"), "Should have file link");
        assert!(
            html_string.contains("file-meta"),
            "Should have file metadata"
        );
        assert!(
            html_string.contains("class=\"icon-box\""),
            "Should have icon container element"
        );
        assert!(
            html_string.contains("class=\"ph ") || html_string.contains("class=\"ph-"),
            "Should have Phosphor icon class"
        );
    }
}
