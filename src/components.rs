//! Reusable HTML components for page generation
//!
//! This module provides Maud component functions shared across multiple
//! page types (index, blob, tree, commits). Components handle specific UI
//! elements with consistent styling and behavior, eliminating duplication
//! across generator functions.

// Components are not yet integrated into page generators
#![allow(dead_code)]

pub mod commit;
pub mod file_list;
pub mod footer;
pub mod icons;
pub mod layout;
pub mod metadata;
pub mod nav;
