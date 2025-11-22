use anyhow::{Context, Result};
use gitkyl::{CommitInfo, Config, TreeItem};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use std::fs;
use std::path::Path;

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

/// Data container for index page generation.
pub struct IndexPageData<'a> {
    pub name: &'a str,
    pub owner: Option<&'a str>,
    pub default_branch: &'a str,
    pub branches: &'a [String],
    pub commit_count: usize,
    pub latest_commit: Option<&'a CommitInfo>,
    pub items: &'a [TreeItem],
    pub readme_html: Option<&'a str>,
}

/// Generate repository index page HTML with optional README rendering.
///
/// Creates the main repository landing page showing file tree and optionally
/// renders README content below the file table. Follows GitHub's visual
/// hierarchy: repository info, file explorer, then README.
///
/// # Arguments
///
/// * `data`: Index page data container with all required fields
///
/// # Returns
///
/// Complete HTML markup for index page
fn index_page(data: IndexPageData<'_>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (data.name) " - Gitkyl" }
                script src="https://unpkg.com/@phosphor-icons/web" {}
                link rel="stylesheet" href="assets/index.css";
                link rel="stylesheet" href="assets/markdown.css";
            }
            body {
                div class="container" {
                    header class="repo-header" {
                        @if let Some(owner_name) = data.owner {
                            span class="repo-owner" { (owner_name) " / " }
                        }
                        h1 class="repo-name" { (data.name) }
                    }

                    main class="repo-card" {
                        div class="repo-controls" {
                            div class="commit-info-group" {
                                @if data.branches.len() >= MIN_BRANCHES_FOR_SELECTOR {
                                    div class="branch-selector" {
                                        i class="ph ph-git-branch" {}
                                        @for branch in data.branches {
                                            @if branch == data.default_branch {
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
                                        span { (data.default_branch) }
                                    }
                                }

                                @if let Some(commit) = data.latest_commit {
                                    div class="commit-info-wrapper" {
                                        div class="commit-line" {
                                            span class="avatar-placeholder" {}
                                            span class="repo-commit-message" { (commit.message()) }
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

                            a href=(format!("commits/{}/index.html", data.default_branch)) class="history-link" {
                                i class="ph ph-clock-counter-clockwise" {}
                                " " (data.commit_count) " commits"
                            }
                        }

                        @if data.items.is_empty() {
                            p class="empty-state" { "No files in this repository" }
                        } @else {
                            div class="file-table" {
                                @for item in data.items.iter() {
                                    @match item {
                                        TreeItem::File { entry, commit } => {
                                            @if let Some(path) = entry.path()
                                                && let Some(path_str) = path.to_str() {
                                                @let (icon_class, icon_modifier) = get_file_icon_info(path_str);
                                                @let href = format!("blob/{}/{}.html", data.default_branch, path_str);
                                                a href=(href) class="file-row" {
                                                    div class="icon-box" {
                                                        @if let Some(modifier) = icon_modifier {
                                                            i class=(format!("{} {}", icon_class, modifier)) {}
                                                        } @else {
                                                            i class=(icon_class) {}
                                                        }
                                                    }
                                                    div class="file-link" { (path_str) }
                                                    div class="commit-message" title=(commit.message_full()) {
                                                        (commit.message())
                                                    }
                                                    div class="commit-date" {
                                                        (format_relative_time(commit.date()))
                                                    }
                                                }
                                            }
                                        },
                                        TreeItem::Directory { name, full_path, commit } => {
                                            @let display_path = if full_path.is_empty() { name } else { full_path };
                                            @let (icon_class, icon_modifier) = get_file_icon_info(&format!("{}/", display_path));
                                            @let href = format!("tree/{}/{}.html", data.default_branch, display_path);
                                            a href=(href) class="file-row" {
                                                div class="icon-box" {
                                                    @if let Some(modifier) = icon_modifier {
                                                        i class=(format!("{} {}", icon_class, modifier)) {}
                                                    } @else {
                                                        i class=(icon_class) {}
                                                    }
                                                }
                                                div class="file-link" { (name) }
                                                div class="commit-message" title=(commit.message_full()) {
                                                    (commit.message())
                                                }
                                                div class="commit-date" {
                                                    (format_relative_time(commit.date()))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    @if let Some(readme) = data.readme_html {
                        section class="readme-section" {
                            div class="readme-card" {
                                div class="readme-header" {
                                    i class="ph ph-info" {}
                                    span class="readme-title" { "README.md" }
                                }
                                div class="readme-content latte" {
                                    (PreEscaped(readme))
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

/// Finds and renders README file at repository root.
///
/// Searches tree items for README files with prioritized detection order:
/// README.md > README > readme.md. Only renders README files at repository
/// root level (not in subdirectories).
///
/// # Arguments
///
/// * `repo_path`: Path to git repository
/// * `ref_name`: Git reference (branch/tag/commit)
/// * `tree_items`: Tree items at repository root
///
/// # Returns
///
/// Optional rendered README HTML string, or None if no README found
///
/// # Errors
///
/// Returns error if blob reading or markdown rendering fails
fn find_and_render_readme(
    repo_path: impl AsRef<Path>,
    ref_name: &str,
    tree_items: &[TreeItem],
) -> Result<Option<String>> {
    // Prioritized README variants to check
    const README_VARIANTS: &[&str] = &["README.md", "README", "readme.md", "Readme.md"];

    // Find README file in tree items
    let readme_entry = tree_items.iter().find_map(|item| {
        if let TreeItem::File { entry, .. } = item
            && let Some(path) = entry.path()
            && let Some(path_str) = path.to_str()
            && gitkyl::is_readme(path)
        {
            // Check priority order
            for variant in README_VARIANTS {
                if path_str == *variant {
                    return Some(entry);
                }
            }
            // Fallback: any README match
            return Some(entry);
        }
        None
    });

    // If README found, render it
    if let Some(entry) = readme_entry
        && let Some(path) = entry.path()
    {
        let content_bytes = gitkyl::read_blob(&repo_path, Some(ref_name), path)
            .context("Failed to read README blob")?;

        let content = String::from_utf8(content_bytes).context("README contains invalid UTF8")?;

        let renderer = gitkyl::MarkdownRenderer::new();
        let rendered = renderer
            .render(&content)
            .context("Failed to render README markdown")?;

        return Ok(Some(rendered));
    }

    Ok(None)
}

/// Validates tree path for security.
///
/// Ensures path does not contain directory traversal attempts or
/// absolute paths that could escape the repository root.
///
/// # Arguments
///
/// * `path`: Path to validate
///
/// # Returns
///
/// Ok if path is safe, Err otherwise
///
/// # Errors
///
/// Returns error if path contains ".." or starts with "/"
fn validate_tree_path(path: &str) -> Result<()> {
    if path.contains("..") {
        anyhow::bail!("Path contains directory traversal: {}", path);
    }
    if path.starts_with('/') {
        anyhow::bail!("Path is absolute, must be relative: {}", path);
    }
    Ok(())
}

/// Formats Unix timestamp as human readable relative time.
///
/// # Arguments
///
/// * `seconds`: Unix timestamp in seconds since epoch
///
/// # Returns
///
/// Human readable relative time string (e.g., "2 days ago", "just now")
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

    let assets_dir = config.output.join("assets");
    fs::create_dir_all(&assets_dir).context("Failed to create assets directory")?;

    fs::write(
        assets_dir.join("index.css"),
        include_str!("../assets/index.css"),
    )
    .context("Failed to write index.css")?;
    fs::write(
        assets_dir.join("blob.css"),
        include_str!("../assets/blob.css"),
    )
    .context("Failed to write blob.css")?;
    fs::write(
        assets_dir.join("commits.css"),
        include_str!("../assets/commits.css"),
    )
    .context("Failed to write commits.css")?;
    fs::write(
        assets_dir.join("tree.css"),
        include_str!("../assets/tree.css"),
    )
    .context("Failed to write tree.css")?;
    fs::write(
        assets_dir.join("markdown.css"),
        include_str!("../assets/markdown.css"),
    )
    .context("Failed to write markdown.css")?;

    let latest_commit =
        gitkyl::list_commits(&config.repo, Some(repo_info.default_branch()), Some(1))
            .ok()
            .and_then(|commits| commits.into_iter().next());

    let files =
        gitkyl::list_files(&config.repo, Some(repo_info.default_branch())).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to list files: {:#}", e);
            vec![]
        });

    // Build tree structure once for O(depth) queries
    let tree = gitkyl::FileTree::from_files(files.clone());

    // Batch lookup commits for ALL files once (reused for root and tree pages)
    let all_file_paths: Vec<&str> = files.iter().filter_map(|f| f.path()?.to_str()).collect();

    let commit_map = gitkyl::get_last_commits_batch(
        &config.repo,
        Some(repo_info.default_branch()),
        &all_file_paths,
    )
    .unwrap_or_else(|e| {
        eprintln!("Warning: Failed to batch lookup commits: {:#}", e);
        std::collections::HashMap::new()
    });

    let top_level_files = tree.files_at("");
    let top_level_subdirs = tree.subdirs_at("");

    let mut tree_items = Vec::new();

    // Batch lookup for root level directories
    let root_dir_commit_map = if !top_level_subdirs.is_empty() {
        gitkyl::get_last_commits_batch(
            &config.repo,
            Some(repo_info.default_branch()),
            &top_level_subdirs,
        )
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to batch lookup directory commits: {:#}", e);
            std::collections::HashMap::new()
        })
    } else {
        std::collections::HashMap::new()
    };

    // Build tree items with pre-fetched commits
    for subdir in &top_level_subdirs {
        if let Some(commit) = root_dir_commit_map.get(*subdir) {
            tree_items.push(TreeItem::Directory {
                name: subdir.to_string(),
                full_path: subdir.to_string(),
                commit: commit.clone(),
            });
        } else {
            eprintln!("Warning: No commit found for directory {}", subdir);
        }
    }

    for file in top_level_files {
        if let Some(path) = file.path()
            && let Some(path_str) = path.to_str()
        {
            if let Some(commit) = commit_map.get(path_str) {
                tree_items.push(TreeItem::File {
                    entry: file.clone(),
                    commit: commit.clone(),
                });
            } else {
                eprintln!("Warning: No commit found for file {}", path_str);
            }
        }
    }

    // Detect and render README file at repository root
    let readme_html = find_and_render_readme(&config.repo, repo_info.default_branch(), &tree_items)
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to render README: {:#}", e);
            None
        });

    let html = index_page(IndexPageData {
        name: &config
            .project_name()
            .context("Failed to determine project name")?,
        owner: repo_info.owner(),
        default_branch: repo_info.default_branch(),
        branches: repo_info.branches(),
        commit_count: repo_info.commit_count(),
        latest_commit: latest_commit.as_ref(),
        items: &tree_items,
        readme_html: readme_html.as_deref(),
    });

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
    let mut markdown_count = 0;
    for entry in &files {
        if let Some(path) = entry.path() {
            if path.to_str().is_none() {
                eprintln!(
                    "Warning: Skipping file with invalid UTF-8 path: {}",
                    path.display()
                );
                continue;
            }

            // Detect README files for markdown rendering
            let result = if gitkyl::is_readme(path) {
                markdown_count += 1;
                gitkyl::generate_markdown_blob_page(
                    &config.repo,
                    repo_info.default_branch(),
                    path,
                    &config
                        .project_name()
                        .context("Failed to determine project name")?,
                )
            } else {
                gitkyl::generate_blob_page(
                    &config.repo,
                    repo_info.default_branch(),
                    path,
                    &config
                        .project_name()
                        .context("Failed to determine project name")?,
                    &config.theme,
                )
            };

            match result {
                Ok(html) => {
                    let blob_path = config
                        .output
                        .join("blob")
                        .join(repo_info.default_branch())
                        .join(format!("{}.html", path.display()));

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

    println!(
        "Generated {} file pages ({} markdown, {} code)",
        generated_count,
        markdown_count,
        generated_count - markdown_count
    );

    println!("Generating tree pages...");

    let directories = tree.all_dirs();
    let mut tree_count = 0;

    for dir_path in directories {
        validate_tree_path(&dir_path)
            .with_context(|| format!("Invalid tree path: {}", dir_path))?;

        let entries_at_this_level = tree.files_at(&dir_path);
        let subdirs_at_this_level = tree.subdirs_at(&dir_path);

        // Build full subdir paths for directory commit lookup
        let full_subdir_paths: Vec<String> = subdirs_at_this_level
            .iter()
            .map(|subdir| {
                if dir_path.is_empty() {
                    subdir.to_string()
                } else {
                    format!("{}/{}", dir_path, subdir)
                }
            })
            .collect();

        // Batch lookup for directories at this level
        let dir_paths_refs: Vec<&str> = full_subdir_paths.iter().map(|s| s.as_str()).collect();
        let level_dir_commit_map = if !dir_paths_refs.is_empty() {
            gitkyl::get_last_commits_batch(
                &config.repo,
                Some(repo_info.default_branch()),
                &dir_paths_refs,
            )
            .unwrap_or_else(|e| {
                eprintln!(
                    "Warning: Failed to batch lookup directory commits for {}: {:#}",
                    dir_path, e
                );
                std::collections::HashMap::new()
            })
        } else {
            std::collections::HashMap::new()
        };

        let mut tree_items_for_page = Vec::new();

        // Build directory items with pre-fetched commits
        for (i, subdir) in subdirs_at_this_level.iter().enumerate() {
            let full_subdir_path = &full_subdir_paths[i];

            if let Some(commit) = level_dir_commit_map.get(full_subdir_path.as_str()) {
                tree_items_for_page.push(TreeItem::Directory {
                    name: subdir.to_string(),
                    full_path: full_subdir_path.clone(),
                    commit: commit.clone(),
                });
            } else {
                eprintln!(
                    "Warning: No commit found for directory {}",
                    full_subdir_path
                );
            }
        }

        // Build file items with pre-fetched commits
        for entry in entries_at_this_level {
            if let Some(path) = entry.path()
                && let Some(path_str) = path.to_str()
            {
                if let Some(commit) = commit_map.get(path_str) {
                    tree_items_for_page.push(TreeItem::File {
                        entry: entry.clone(),
                        commit: commit.clone(),
                    });
                } else {
                    eprintln!("Warning: No commit found for file {}", path_str);
                }
            }
        }

        match gitkyl::generate_tree_page(
            &config.repo,
            repo_info.default_branch(),
            &dir_path,
            repo_info.name(),
            &tree_items_for_page,
        ) {
            Ok(html) => {
                let tree_path = config
                    .output
                    .join("tree")
                    .join(repo_info.default_branch())
                    .join(format!("{}.html", dir_path));

                if let Some(parent) = tree_path.parent() {
                    fs::create_dir_all(parent).context("Failed to create tree directory")?;
                }

                fs::write(&tree_path, html.into_string()).with_context(|| {
                    format!("Failed to write tree page {}", tree_path.display())
                })?;

                tree_count += 1;
            }
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Failed to generate tree page for {}", dir_path));
            }
        }
    }

    println!("Generated {} tree pages", tree_count);

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
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string(), "develop".to_string()];
        let commit_count = 42;
        let items = vec![];

        // Act
        let html = index_page(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
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
        // Arrange: Test with mock commit data
        use gitkyl::CommitInfo;

        let name = "TestRepo";
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 10;
        let items = vec![];

        // Create mock commit info
        let mock_commit = CommitInfo::new(
            "abc123def456".to_string(),
            "Initial commit".to_string(),
            "Initial commit\n\nThis is the first commit.".to_string(),
            "Test Author".to_string(),
            1234567890,
        );

        // Act
        let html = index_page(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: Some(&mock_commit),
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert
        assert!(
            html_string.contains("commit-info-group"),
            "Should have commit info group"
        );
        assert!(
            html_string.contains("abc123d"),
            "Should show commit hash (short form)"
        );
        assert!(
            html_string.contains("Initial commit"),
            "Should show commit message"
        );
        assert!(
            html_string.contains("Test Author"),
            "Should show author name"
        );
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
        use gitkyl::CommitInfo;
        use tempfile;

        // Arrange: Create mock commits with known data
        let temp_dir = tempfile::tempdir().expect("Should create temp directory");
        let output = temp_dir.path();

        let mock_commits = vec![
            CommitInfo::new(
                "abc123".to_string(),
                "Add feature X".to_string(),
                "Add feature X\n\nDetailed description.".to_string(),
                "Alice".to_string(),
                1234567890,
            ),
            CommitInfo::new(
                "def456".to_string(),
                "Fix bug Y".to_string(),
                "Fix bug Y".to_string(),
                "Bob".to_string(),
                1234567800,
            ),
            CommitInfo::new(
                "ghi789".to_string(),
                "Refactor module Z".to_string(),
                "Refactor module Z".to_string(),
                "Carol".to_string(),
                1234567700,
            ),
        ];

        let branch_name = "main";
        let repo_name = "test-repo";

        // Act
        let html = gitkyl::generate_commits_page(&mock_commits, branch_name, repo_name);

        let commits_dir = output.join("commits").join(branch_name);
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
        assert!(content.contains(branch_name), "Should contain branch name");
        assert!(
            content.contains("Add feature X"),
            "Should contain first commit message"
        );
        assert!(
            content.contains("Alice"),
            "Should contain first commit author"
        );
        assert_eq!(mock_commits.len(), 3, "Should have exactly 3 test commits");
    }

    #[test]
    fn test_index_page_with_file_table() {
        // Arrange: Test with mock TreeItem directory structure
        use gitkyl::CommitInfo;

        let name = "TestRepo";
        let owner = None;
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 5;

        let mock_commit = CommitInfo::new(
            "abc123".to_string(),
            "Add files".to_string(),
            "Add files".to_string(),
            "Test Author".to_string(),
            1234567890,
        );

        // Create directory items to test file table rendering
        let items = vec![
            TreeItem::Directory {
                name: "src".to_string(),
                full_path: "src".to_string(),
                commit: mock_commit.clone(),
            },
            TreeItem::Directory {
                name: "tests".to_string(),
                full_path: "tests".to_string(),
                commit: mock_commit.clone(),
            },
        ];

        // Act
        let html = index_page(IndexPageData {
            name,
            owner,
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert: Check that file table structure is present
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
            html_string.contains("commit-date"),
            "Should contain commit date"
        );
        assert!(html_string.contains("src"), "Should contain directory name");
    }

    #[test]
    fn test_index_page_file_table_structure() {
        // Arrange: Test file table structure with directory item
        use gitkyl::CommitInfo;

        let mock_commit = CommitInfo::new(
            "def456".to_string(),
            "Add directory".to_string(),
            "Add directory".to_string(),
            "Test Author".to_string(),
            1234567890,
        );

        let items = vec![TreeItem::Directory {
            name: "lib".to_string(),
            full_path: "lib".to_string(),
            commit: mock_commit,
        }];

        let branches = vec!["main".to_string()];

        // Act
        let html = index_page(IndexPageData {
            name: "test",
            owner: None,
            default_branch: "main",
            branches: &branches,
            commit_count: 1,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert: Check HTML structure elements are present
        assert!(html_string.contains("file-link"), "Should have file link");
        assert!(
            html_string.contains("commit-date"),
            "Should have commit date"
        );
        assert!(
            html_string.contains("class=\"icon-box\""),
            "Should have icon container element"
        );
        assert!(
            html_string.contains("class=\"ph ") || html_string.contains("class=\"ph-"),
            "Should have Phosphor icon class"
        );
        assert!(html_string.contains("lib"), "Should contain directory name");
    }

    #[test]
    fn test_validate_tree_path_valid() {
        // Arrange: Test valid paths
        let valid_paths = vec!["src", "src/main.rs", "docs/README.md", "a/b/c/d"];

        // Act & Assert: All should pass validation
        for path in valid_paths {
            let result = validate_tree_path(path);
            assert!(
                result.is_ok(),
                "Path '{}' should be valid but got error: {:?}",
                path,
                result.err()
            );
        }
    }

    #[test]
    fn test_validate_tree_path_traversal() {
        // Arrange: Test path traversal attempts
        let invalid_paths = vec![
            "../etc/passwd",
            "src/../../../etc/passwd",
            "foo/bar/../../../baz",
            "/absolute/path",
            "/etc/passwd",
        ];

        // Act & Assert: All should fail validation
        for path in invalid_paths {
            let result = validate_tree_path(path);
            assert!(
                result.is_err(),
                "Path '{}' should be invalid but passed validation",
                path
            );
        }
    }

    #[test]
    fn test_validate_tree_path_absolute() {
        // Arrange: Test absolute path rejection
        let absolute_paths = vec!["/", "/usr", "/home/user"];

        // Act & Assert
        for path in absolute_paths {
            let result = validate_tree_path(path);
            assert!(
                result.is_err(),
                "Absolute path '{}' should be rejected",
                path
            );
            assert!(
                result.unwrap_err().to_string().contains("absolute"),
                "Error should mention absolute path"
            );
        }
    }

    #[test]
    fn test_get_file_icon_info_readme() {
        // Arrange & Act & Assert: Test README file icon detection
        let (icon, modifier) = get_file_icon_info("README.md");
        assert_eq!(icon, "ph ph-info", "README should use info icon");
        assert_eq!(
            modifier,
            Some("icon-readme"),
            "README should have icon-readme modifier"
        );

        let (icon, modifier) = get_file_icon_info("readme.txt");
        assert_eq!(icon, "ph ph-info", "lowercase readme should use info icon");
        assert_eq!(modifier, Some("icon-readme"), "Should have modifier");

        let (icon, modifier) = get_file_icon_info("README");
        assert_eq!(
            icon, "ph ph-info",
            "README without extension should use info icon"
        );
        assert_eq!(modifier, Some("icon-readme"), "Should have modifier");
    }

    #[test]
    fn test_get_file_icon_info_rust_files() {
        // Arrange & Act & Assert
        let (icon, modifier) = get_file_icon_info("main.rs");
        assert_eq!(icon, "ph ph-file-rs", "Rust files should use rs icon");
        assert_eq!(modifier, Some("icon-rust"), "Should have rust modifier");
    }

    #[test]
    fn test_get_file_icon_info_config_files() {
        // Arrange & Act & Assert
        let (icon, modifier) = get_file_icon_info("Cargo.toml");
        assert_eq!(icon, "ph ph-gear", "TOML should use gear icon");
        assert_eq!(modifier, Some("icon-config"), "Should have config modifier");

        let (icon, _modifier) = get_file_icon_info("config.yaml");
        assert_eq!(icon, "ph ph-gear", "YAML should use gear icon");
    }

    #[test]
    fn test_get_file_icon_info_directories() {
        // Arrange & Act & Assert
        let (icon, modifier) = get_file_icon_info("src/");
        assert_eq!(
            icon, "ph-fill ph-folder",
            "Directories should use folder icon"
        );
        assert_eq!(modifier, Some("icon-folder"), "Should have folder modifier");
    }

    #[test]
    fn test_get_file_icon_info_generic() {
        // Arrange & Act & Assert
        let (icon, modifier) = get_file_icon_info("unknown.xyz");
        assert_eq!(icon, "ph ph-file", "Unknown files should use generic icon");
        assert_eq!(modifier, None, "Should have no modifier");
    }

    #[test]
    fn test_readme_vs_other_markdown() {
        // Arrange & Act: Test that README is prioritized over extension
        let (readme_icon, readme_mod) = get_file_icon_info("README.md");
        let (doc_icon, doc_mod) = get_file_icon_info("CONTRIBUTING.md");

        // Assert: README should use info icon, other .md should use generic
        assert_eq!(readme_icon, "ph ph-info", "README should use info icon");
        assert_eq!(
            readme_mod,
            Some("icon-readme"),
            "README has readme modifier"
        );

        assert_eq!(doc_icon, "ph ph-file", "Other markdown uses generic icon");
        assert_eq!(doc_mod, None, "Other markdown has no modifier");
    }

    #[test]
    fn test_find_and_render_readme_found() {
        // Arrange
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "master";

        let files = gitkyl::list_files(&repo_path, Some(ref_name)).expect("Should list files");

        let readme_file = files
            .iter()
            .find(|f| gitkyl::is_readme(f.path().unwrap_or(Path::new(""))))
            .expect("Repository should have README");

        let commit = gitkyl::get_file_last_commit(
            &repo_path,
            Some(ref_name),
            readme_file.path().unwrap().to_str().unwrap(),
        )
        .expect("Should get README commit");

        let tree_items = vec![TreeItem::File {
            entry: readme_file.clone(),
            commit,
        }];

        // Act
        let result = find_and_render_readme(&repo_path, ref_name, &tree_items);

        // Assert
        assert!(result.is_ok(), "Should successfully render README");
        let html = result.unwrap();
        assert!(html.is_some(), "Should find and render README");
        let rendered = html.unwrap();
        assert!(!rendered.is_empty(), "Rendered HTML should not be empty");
        assert!(
            rendered.contains("<h1>") || rendered.contains("<h2>") || rendered.contains("<p>"),
            "Should contain HTML tags from rendered markdown"
        );
    }

    #[test]
    fn test_find_and_render_readme_not_found() {
        // Arrange: Empty tree items
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "master";
        let tree_items = vec![];

        // Act
        let result = find_and_render_readme(&repo_path, ref_name, &tree_items);

        // Assert
        assert!(result.is_ok(), "Should handle missing README gracefully");
        let html = result.unwrap();
        assert!(html.is_none(), "Should return None when no README found");
    }

    #[test]
    fn test_find_and_render_readme_priority() {
        // Arrange: Multiple README variants
        let repo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ref_name = "master";

        let files = gitkyl::list_files(&repo_path, Some(ref_name)).expect("Should list files");

        let readme_files: Vec<_> = files
            .iter()
            .filter(|f| gitkyl::is_readme(f.path().unwrap_or(Path::new(""))))
            .collect();

        if readme_files.is_empty() {
            return;
        }

        let mut tree_items = vec![];
        for file in readme_files {
            if let Ok(commit) = gitkyl::get_file_last_commit(
                &repo_path,
                Some(ref_name),
                file.path().unwrap().to_str().unwrap(),
            ) {
                tree_items.push(TreeItem::File {
                    entry: file.clone(),
                    commit,
                });
            }
        }

        // Act
        let result = find_and_render_readme(&repo_path, ref_name, &tree_items);

        // Assert
        assert!(result.is_ok(), "Should handle multiple README files");
        if let Ok(Some(html)) = result {
            assert!(!html.is_empty(), "Should render README content");
        }
    }

    #[test]
    fn test_index_page_with_readme() {
        // Arrange
        let name = "TestRepo";
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 10;
        let items = vec![];
        let readme_html = Some("<h1>Test README</h1><p>This is a test.</p>");

        // Act
        let html = index_page(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html,
        });
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("TestRepo"), "Should contain repo name");
        assert!(
            html_string.contains("readme-section"),
            "Should have README section"
        );
        assert!(
            html_string.contains("readme-card"),
            "Should have README card"
        );
        assert!(
            html_string.contains("Test README"),
            "Should contain README content"
        );
        assert!(
            html_string.contains("This is a test"),
            "Should contain README text"
        );
    }

    #[test]
    fn test_index_page_without_readme() {
        // Arrange
        let name = "TestRepo";
        let owner = "testuser";
        let default_branch = "main";
        let branches = vec!["main".to_string()];
        let commit_count = 10;
        let items = vec![];

        // Act
        let html = index_page(IndexPageData {
            name,
            owner: Some(owner),
            default_branch,
            branches: &branches,
            commit_count,
            latest_commit: None,
            items: &items,
            readme_html: None,
        });
        let html_string = html.into_string();

        // Assert
        assert!(html_string.contains("TestRepo"), "Should contain repo name");
        assert!(
            !html_string.contains("readme-section"),
            "Should not have README section when no README provided"
        );
    }
}
