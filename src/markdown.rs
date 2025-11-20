//! Markdown rendering with GitHub Flavored Markdown support.
//!
//! This module provides markdown rendering using comrak with GFM extensions
//! (tables, strikethrough, autolinks, task lists) and link resolution for
//! repository internal references.

mod links;
mod renderer;

pub use links::LinkResolver;
pub use renderer::MarkdownRenderer;
