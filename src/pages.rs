//! Page generation modules for different view types
//!
//! This module organizes HTML page generators by page type (index, blob,
//! tree, etc.). Each page module handles its specific view logic and
//! utilizes shared components from the components module.

pub mod blob;
pub mod commits;
pub mod index;
pub mod tree;
