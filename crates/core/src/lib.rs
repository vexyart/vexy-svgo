// this_file: crates/core/src/lib.rs

//! # vexy_svgo-core
//!
//! Core SVG processing engine for vexy_svgo, providing parsing, AST manipulation,
//! optimization, and stringification capabilities.
//!
//! ## Overview
//!
//! This crate provides the fundamental building blocks for SVG optimization:
//!
//! - **Parsing**: Convert SVG strings into an Abstract Syntax Tree (AST)
//! - **Optimization**: Apply various optimization plugins to reduce file size
//! - **Stringification**: Convert the optimized AST back to an SVG string
//!
//! ## Quick Start
//!
//! ```rust
//! use vexy_svgo_core::{optimize_default, parse_svg, stringify};
//!
//! // Simple optimization with default settings
//! let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
//! let result = optimize_default(svg).unwrap();
//! println!("Optimized: {}", result.data);
//!
//! // Manual parsing and stringification
//! let doc = parse_svg(svg).unwrap();
//! let output = stringify(&doc).unwrap();
//! ```
//!
//! ## Features
//!
//! - `parallel`: Enable parallel processing for large SVG files
//! - `wasm`: WebAssembly compatibility
//! - `python`: Python bindings support

// Public modules
pub mod ast;
pub mod collections;
pub mod css;
pub mod error;
pub mod features;
pub mod optimizer;
pub mod parser;
pub mod plugin_registry;
pub mod stringifier;
pub mod utils;
pub mod visitor;

// Re-exports for a flatter API
pub use ast::{Document, DocumentMetadata, Element, Node};
pub use css::variables::{CssScope, CssVariableResolver};
pub use error::VexyError;
pub use optimizer::memory::{MemoryBudget, MemoryError};
#[cfg(feature = "parallel")]
pub use optimizer::parallel;
pub use optimizer::{
    optimize, optimize_default, optimize_with_config, OptimizationInfo, OptimizationResult, OptimizeOptions,
};
pub use parser::config::{Config, DataUriFormat, LineEnding, PluginConfig, StreamingConfig};
pub use parser::error::ParseError;
pub use parser::{load_config_from_directory, parse_svg, parse_svg_file, parse_svg_streaming, Parser};
pub use plugin_registry::{create_default_registry, Plugin, PluginRegistry};
pub use stringifier::{stringify, stringify_with_config, StreamingStringifier, StringifyConfig};

/// Version string for vexy_svgo
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
