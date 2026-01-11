//! View toggle component for switching between file views

use maud::{Markup, html};

/// The mode of blob view being rendered
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Rendered markdown preview
    Preview,
    /// Source code with syntax highlighting
    Code,
    /// Git blame annotations
    Blame,
}

impl ViewMode {
    /// Returns the icon class and label for this view mode
    pub fn icon_and_label(&self) -> (&'static str, &'static str) {
        match self {
            ViewMode::Preview => ("ph ph-eye", "Preview"),
            ViewMode::Code => ("ph ph-code", "Code"),
            ViewMode::Blame => ("ph ph-git-commit", "Blame"),
        }
    }
}

/// View tab with optional link (None means active tab)
pub struct ViewTab {
    pub mode: ViewMode,
    pub link: Option<String>,
}

impl ViewTab {
    /// Creates a new view tab
    pub fn new(mode: ViewMode, link: Option<String>) -> Self {
        Self { mode, link }
    }

    /// Returns the icon class and label for this tab
    fn icon_and_label(&self) -> (&'static str, &'static str) {
        self.mode.icon_and_label()
    }

    /// Returns the link if this tab is not active
    fn link(&self) -> Option<&str> {
        self.link.as_deref()
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
