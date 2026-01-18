//! Footer component for all pages

use maud::{Markup, html};
use std::sync::LazyLock;

static HEART_COLOR: LazyLock<&'static str> = LazyLock::new(|| {
    let colors = [
        "#FF6B6B", // Red
        "#4ECDC4", // Teal
        "#45B7D1", // Blue
        "#96CEB4", // Green
        "#FFEAA7", // Yellow
        "#A29BFE", // Purple
        "#FD79A8", // Pink
        "#FDCB6E", // Orange
        "#00B894", // Emerald
        "#E17055", // Coral
    ];

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id() as u128;
    let seed = (time.wrapping_mul(31).wrapping_add(pid)) as usize;

    colors[seed % colors.len()]
});

/// Renders page footer with Gitkyl attribution
pub fn footer() -> Markup {
    html! {
        footer {
            p class="footer-text" {
                "Built with "
                span class="heart" style=(format!("color: {}", *HEART_COLOR)) { "♥" }
                " by "
                a href="https://github.com/lemorage/gitkyl"
                   target="_blank"
                   class="footer-link" {
                    "Gitkyl"
                }
            }
        }
    }
}
