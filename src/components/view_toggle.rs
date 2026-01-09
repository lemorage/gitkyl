//! View toggle component for switching between file views

use maud::{Markup, html};

/// Available view tabs for file display
pub enum ViewTab {
    /// Rendered markdown preview
    Preview { link: Option<String> },
    /// Source code with syntax highlighting
    Code { link: Option<String> },
    /// Git blame annotations
    Blame { link: Option<String> },
}

impl ViewTab {
    /// Returns the icon class and label for this tab
    fn icon_and_label(&self) -> (&'static str, &'static str) {
        match self {
            ViewTab::Preview { .. } => ("ph ph-eye", "Preview"),
            ViewTab::Code { .. } => ("ph ph-code", "Code"),
            ViewTab::Blame { .. } => ("ph ph-git-commit", "Blame"),
        }
    }

    /// Returns the link if this tab is not active
    fn link(&self) -> Option<&str> {
        match self {
            ViewTab::Preview { link } | ViewTab::Code { link } | ViewTab::Blame { link } => {
                link.as_deref()
            }
        }
    }
}

/// Renders a view toggle with the specified tabs
pub fn view_toggle(tabs: &[ViewTab]) -> Markup {
    html! {
        div class="view-toggle" {
            @for tab in tabs {
                @let (icon, label) = tab.icon_and_label();
                @if let Some(href) = tab.link() {
                    a href=(href) class="view-tab" {
                        i class=(icon) {}
                        " " (label)
                    }
                } @else {
                    span class="view-tab active" {
                        i class=(icon) {}
                        " " (label)
                    }
                }
            }
        }
    }
}
