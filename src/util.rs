//! Utility functions for gitkyl

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Calculates relative path depth for HTML pages.
///
/// Determines how many `../` prefixes are needed to reach repository root
/// from generated HTML pages. Accounts for slashes in branch names
/// (e.g., "fix/dashboard-delay") and nested file paths.
///
/// # Arguments
///
/// * `branch`: Branch or reference name (may contain slashes)
/// * `path`: File or directory path (empty string for root level pages)
///
/// # Returns
///
/// Number of directory levels needed to traverse back to root
pub fn calculate_depth(branch: &str, path: &str) -> usize {
    let branch_depth = branch.matches('/').count() + 1;
    let path_depth = if path.is_empty() {
        0
    } else {
        path.matches('/').count()
    };
    branch_depth + path_depth + 1
}

/// Formats Unix timestamp as human readable relative time
///
/// Converts Unix epoch seconds to relative time strings like "5 min ago"
/// or "2 weeks ago". Future timestamps are treated as "just now".
///
/// # Arguments
///
/// * `seconds`: Unix timestamp in seconds since epoch
///
/// # Returns
///
/// Human readable relative time string
pub fn format_timestamp(seconds: i64) -> String {
    let timestamp = UNIX_EPOCH + Duration::from_secs(seconds as u64);
    let now = SystemTime::now();

    // Handle future timestamps gracefully by treating as present
    // Occurs when clock skew or invalid timestamps are present in git history
    let duration = now.duration_since(timestamp).unwrap_or(Duration::ZERO);
    let secs = duration.as_secs();
    let minutes = secs / 60;
    let hours = secs / 3600;
    let days = secs / 86400;

    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{} min ago", minutes)
    } else if hours < 24 {
        format!("{} hr ago", hours)
    } else if days < 7 {
        format!("{} days ago", days)
    } else if days < 30 {
        format!("{} weeks ago", days / 7)
    } else if days < 365 {
        format!("{} months ago", days / 30)
    } else {
        format!("{} years ago", days / 365)
    }
}

/// Formats byte count as human readable file size
///
/// Converts byte count to appropriate unit (bytes, KB, MB) with two decimal
/// places for KB and MB. Uses binary prefixes.
///
/// # Arguments
///
/// * `bytes`: File size in bytes
///
/// # Returns
///
/// Formatted string like "512 bytes", "1.50 KB", or "2.00 MB"
pub fn format_file_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_level_branch_root() {
        assert_eq!(calculate_depth("dev", ""), 2);
        assert_eq!(calculate_depth("master", ""), 2);
        assert_eq!(calculate_depth("main", ""), 2);
    }

    #[test]
    fn test_multi_level_branch_root() {
        assert_eq!(calculate_depth("fix/bug", ""), 3);
        assert_eq!(calculate_depth("feature/new-ui", ""), 3);
        assert_eq!(calculate_depth("fix/dashboard-delay", ""), 3);
        assert_eq!(calculate_depth("a/b/c", ""), 4);
    }

    #[test]
    fn test_single_level_branch_with_path() {
        assert_eq!(calculate_depth("dev", "src"), 2);
        assert_eq!(calculate_depth("dev", "README.md"), 2);
        assert_eq!(calculate_depth("dev", "src/main.rs"), 3);
        assert_eq!(calculate_depth("dev", "src/pages/index.rs"), 4);
    }

    #[test]
    fn test_multi_level_branch_with_path() {
        assert_eq!(calculate_depth("fix/bug", "src"), 3);
        assert_eq!(calculate_depth("fix/bug", "src/main.rs"), 4);
        assert_eq!(calculate_depth("feature/ui", "assets/styles.css"), 4);
    }

    #[test]
    fn test_format_timestamp_just_now() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert_eq!(format_timestamp(now), "just now");
    }

    #[test]
    fn test_format_timestamp_minutes() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let five_min_ago = (now - 300) as i64;
        assert_eq!(format_timestamp(five_min_ago), "5 min ago");

        let thirty_min_ago = (now - 1800) as i64;
        assert_eq!(format_timestamp(thirty_min_ago), "30 min ago");
    }

    #[test]
    fn test_format_timestamp_hours() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_hr_ago = (now - 7200) as i64;
        assert_eq!(format_timestamp(two_hr_ago), "2 hr ago");

        let twelve_hr_ago = (now - 43200) as i64;
        assert_eq!(format_timestamp(twelve_hr_ago), "12 hr ago");
    }

    #[test]
    fn test_format_timestamp_days() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_days_ago = (now - 172800) as i64;
        assert_eq!(format_timestamp(two_days_ago), "2 days ago");

        let five_days_ago = (now - 432000) as i64;
        assert_eq!(format_timestamp(five_days_ago), "5 days ago");
    }

    #[test]
    fn test_format_timestamp_weeks() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_weeks_ago = (now - 1209600) as i64;
        assert_eq!(format_timestamp(two_weeks_ago), "2 weeks ago");

        let three_weeks_ago = (now - 1814400) as i64;
        assert_eq!(format_timestamp(three_weeks_ago), "3 weeks ago");
    }

    #[test]
    fn test_format_timestamp_months() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_months_ago = (now - 5184000) as i64;
        assert_eq!(format_timestamp(two_months_ago), "2 months ago");

        let six_months_ago = (now - 15552000) as i64;
        assert_eq!(format_timestamp(six_months_ago), "6 months ago");
    }

    #[test]
    fn test_format_timestamp_years() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let two_years_ago = (now - 63072000) as i64;
        assert_eq!(format_timestamp(two_years_ago), "2 years ago");

        let three_years_ago = (now - 94608000) as i64;
        assert_eq!(format_timestamp(three_years_ago), "3 years ago");
    }

    #[test]
    fn test_format_timestamp_future_treated_as_now() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let future = (now + 3600) as i64;
        assert_eq!(format_timestamp(future), "just now");
    }

    #[test]
    fn test_format_timestamp_boundary() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let fifty_nine_sec = (now - 59) as i64;
        assert_eq!(format_timestamp(fifty_nine_sec), "just now");

        let sixty_sec = (now - 60) as i64;
        assert_eq!(format_timestamp(sixty_sec), "1 min ago");
    }

    #[test]
    fn test_format_file_size_bytes() {
        assert_eq!(format_file_size(0), "0 bytes");
        assert_eq!(format_file_size(1), "1 bytes");
        assert_eq!(format_file_size(512), "512 bytes");
        assert_eq!(format_file_size(1023), "1023 bytes");
    }

    #[test]
    fn test_format_file_size_kilobytes() {
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1536), "1.50 KB");
        assert_eq!(format_file_size(10240), "10.00 KB");
        assert_eq!(format_file_size(1048575), "1024.00 KB");
    }

    #[test]
    fn test_format_file_size_megabytes() {
        assert_eq!(format_file_size(1048576), "1.00 MB");
        assert_eq!(format_file_size(1572864), "1.50 MB");
        assert_eq!(format_file_size(10485760), "10.00 MB");
    }
}
