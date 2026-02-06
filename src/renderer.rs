//! Page generation orchestration.
//!
//! This module organizes static site generation into focused submodules,
//! each handling a specific page type or setup task.

pub mod setup;

pub use setup::setup_output_directories;
