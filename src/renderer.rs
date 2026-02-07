//! Page generation orchestration.
//!
//! This module organizes static site generation into focused submodules,
//! each handling a specific page type or setup task.

pub mod setup;
pub mod tree;

pub use setup::setup_output_directories;
pub use tree::{build_tree_items, generate_tree_pages_for_branch};
