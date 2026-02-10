// this_file: crates/core/src/utils/mod.rs

//! Shared utilities for SVG processing
//!
//! This module provides common utility functions that are used across
//! multiple plugins and core components.

pub mod attributes;
pub mod colors;
pub mod numbers;
pub mod paths;
pub mod selectors;

// Re-export commonly used utilities
pub use attributes::*;
pub use colors::*;
pub use numbers::*;
pub use paths::*;
pub use selectors::*;
