//! View toggle component for switching between file views

use maud::{Markup, html};

use crate::icons::{code_icon, commit_icon, eye_icon};

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
    /// Returns the label for this view mode
    pub fn label(&self) -> &'static str {
        match self {
            ViewMode::Preview => "Preview",
            ViewMode::Code => "Code",
            ViewMode::Blame => "Blame",
        }
    }

    /// Returns the icon markup for this view mode
    pub fn icon(&self) -> Markup {
        match self {
            ViewMode::Preview => eye_icon(),
            ViewMode::Code => code_icon(),
            ViewMode::Blame => commit_icon(),
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
                @if let Some(href) = tab.link() {
                    a href=(href) class="view-tab" {
                        (tab.mode.icon())
                        span { (tab.mode.label()) }
                    }
                } @else {
                    span class="view-tab active" {
                        (tab.mode.icon())
                        span { (tab.mode.label()) }
                    }
                }
            }
        }
    }
}
