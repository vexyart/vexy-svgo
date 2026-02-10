// this_file: crates/core/src/optimizer/mod.rs

//! Core optimization engine
//!
//! This module provides the main optimization functionality that orchestrates
//! parsing, plugin application, and output generation.

#[cfg(feature = "parallel")]
pub mod parallel;

pub mod memory;

use serde::{Deserialize, Serialize};

use crate::error::VexyError;
use crate::parser::config::{Config, DataUriFormat};
use crate::parser::Parser;
use crate::plugin_registry::PluginRegistry;

/// Optimization result type
pub type OptimizeResult<T> = Result<T, VexyError>;

/// Options for the optimize function
pub struct OptimizeOptions {
    /// Configuration to use
    pub config:        Config,
    /// Plugin registry (if None, uses default)
    pub registry:      Option<PluginRegistry>,
    /// Optional memory budget
    pub memory_budget: Option<std::sync::Arc<memory::MemoryBudget>>,
    /// Enable parallel plugin execution (opt-in, default: false)
    pub parallel:      bool,
}

/// Result of an optimization operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// Optimized SVG data
    pub data:   String,
    /// Optimization information
    pub info:   OptimizationInfo,
    /// Error message (if any)
    pub error:  Option<String>,
    /// Whether modern parser was used
    pub modern: bool,
}

/// Information about the optimization process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationInfo {
    /// Original size in bytes
    pub original_size:     usize,
    /// Optimized size in bytes
    pub optimized_size:    usize,
    /// Compression ratio (0.0 to 1.0)
    pub compression_ratio: f64,
    /// Number of plugins applied
    pub plugins_applied:   usize,
    /// Number of optimization passes
    pub passes:            usize,
}

impl OptimizeOptions {
    /// Create new options with the given config
    pub fn new(config: Config) -> Self {
        Self {
            config,
            registry: None,
            memory_budget: None,
            parallel: false,
        }
    }

    /// Set the plugin registry
    pub fn with_registry(mut self, registry: PluginRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Set the memory budget
    pub fn with_memory_budget(mut self, budget: std::sync::Arc<memory::MemoryBudget>) -> Self {
        self.memory_budget = Some(budget);
        self
    }

    /// Enable or disable parallel plugin execution
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl OptimizationInfo {
    /// Create new optimization info
    pub fn new(original_size: usize, optimized_size: usize, plugins_applied: usize, passes: usize) -> Self {
        let compression_ratio = if original_size > 0 {
            1.0 - (optimized_size as f64 / original_size as f64)
        } else {
            0.0
        };

        Self {
            original_size,
            optimized_size,
            compression_ratio,
            plugins_applied,
            passes,
        }
    }

    /// Get the size reduction in bytes
    pub fn size_reduction(&self) -> i64 {
        self.original_size as i64 - self.optimized_size as i64
    }

    /// Get the compression percentage (0-100)
    pub fn compression_percentage(&self) -> f64 {
        self.compression_ratio * 100.0
    }
}

/// Main optimization function
///
/// This is the primary entry point for SVG optimization, equivalent to SVGO's `optimize` function.
/// It parses the SVG, applies a series of plugins, and then stringifies the result.
///
/// # Arguments
///
/// * `input` - The SVG string to optimize
/// * `options` - Configuration options including plugins and output settings
///
/// # Returns
///
/// An `OptimizationResult` containing the optimized SVG and metadata about the optimization
///
/// # Examples
///
/// ```rust
/// use vexy_svgo_core::{optimize, Config, OptimizeOptions};
///
/// let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
/// let options = OptimizeOptions::new(Config::default());
/// let result = optimize(svg, options).unwrap();
/// println!("Optimized SVG: {}", result.data);
/// println!("Compression: {:.1}%", result.info.compression_percentage());
/// ```
pub fn optimize(input: &str, options: OptimizeOptions) -> Result<OptimizationResult, VexyError> {
    let original_size = input.len();
    #[cfg(feature = "parallel")]
    let parallel_enabled = options.parallel;
    let config = options.config;
    let registry = options.registry.unwrap_or_default();

    let mut document = Parser::parse_svg_string(input)?;

    if let Some(budget) = options.memory_budget {
        document.memory_budget = Some(budget);
    }

    #[cfg(feature = "parallel")]
    {
        if parallel_enabled {
            let parallel_config = parallel::ParallelConfig {
                num_threads: config.parallel.unwrap_or(0),
                ..parallel::ParallelConfig::default()
            };
            parallel::configure_thread_pool(&parallel_config);
            crate::debug_log!("Configured parallel processing with {} threads", parallel_config.num_threads);
        }
    }

    // Use sequential optimization (with potential internal parallelism)
    let mut passes = 0;
    let mut plugins_applied = 0;
    let mut previous_output = String::new();

    loop {
        passes += 1;

        // Convert config plugins to registry plugins format
        // Use the plugin configs directly from the config
        let registry_plugins = &config.plugins;

        // Check if parallel processing is enabled
        #[cfg(feature = "parallel")]
        {
            if parallel_enabled {
                let num_threads = config.parallel.unwrap_or(0);
                registry
                    .apply_plugins_parallel(&mut document, registry_plugins, num_threads)
                    .map_err(VexyError::from)?;
            } else {
                registry.apply_plugins(&mut document, registry_plugins).map_err(VexyError::from)?;
            }
        }
        #[cfg(not(feature = "parallel"))]
        {
            registry.apply_plugins(&mut document, registry_plugins).map_err(VexyError::from)?;
        }
        plugins_applied += config.plugins.len();

        // Create stringifier config from js2svg options
        let stringify_config = crate::stringifier::StringifyConfig {
            pretty:           config.js2svg.pretty,
            indent:           config.js2svg.indent.clone(),
            newlines:         config.js2svg.pretty,
            quote_attrs:      true,
            self_close:       config.js2svg.use_short_tags,
            initial_capacity: 4096,
        };

        let current_output = crate::stringifier::stringify_with_config(&document, &stringify_config)?;
        if !config.multipass || current_output == previous_output || passes >= 10 {
            // Apply data URI encoding if configured
            let mut final_output = if let Some(ref format) = config.datauri {
                apply_datauri_encoding(&current_output, format)
            } else {
                current_output
            };
            if config.js2svg.final_newline && !final_output.ends_with('\n') {
                final_output.push('\n');
            }
            let optimized_size = final_output.len();
            let info = OptimizationInfo::new(original_size, optimized_size, plugins_applied, passes);
            return Ok(OptimizationResult {
                data: final_output,
                info,
                error: None,
                modern: true,
            });
        }
        previous_output = current_output;
    }
}

/// Convenience function with default options
///
/// This is a convenience function that optimizes an SVG using the default
/// configuration with all standard plugins enabled.
///
/// # Arguments
///
/// * `input` - The SVG string to optimize
///
/// # Returns
///
/// An `OptimizationResult` containing the optimized SVG
///
/// # Examples
///
/// ```rust
/// use vexy_svgo_core::optimize_default;
///
/// let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
/// let result = optimize_default(svg).unwrap();
/// assert!(!result.data.is_empty());
/// ```
pub fn optimize_default(input: &str) -> Result<OptimizationResult, VexyError> {
    optimize(input, OptimizeOptions::new(Config::with_default_preset()))
}

/// Optimize with a custom configuration
///
/// This function allows you to specify a custom configuration for the optimization process.
/// Use this when you need fine-grained control over which plugins to run and their parameters.
///
/// # Arguments
///
/// * `input` - The SVG string to optimize
/// * `config` - Custom configuration specifying plugins and their settings
///
/// # Returns
///
/// An `OptimizationResult` containing the optimized SVG
///
/// # Examples
///
/// ```rust
/// use vexy_svgo_core::{optimize_with_config, Config, PluginConfig};
///
/// let mut config = Config::default();
/// config.multipass = true;
/// config.plugins = vec![
///     PluginConfig::Name("removeComments".to_string()),
///     PluginConfig::Name("removeDoctype".to_string()),
/// ];
///
/// let svg = r#"<!DOCTYPE svg><!-- test --><svg><rect/></svg>"#;
/// let result = optimize_with_config(svg, config).unwrap();
/// ```
pub fn optimize_with_config(input: &str, config: Config) -> Result<OptimizationResult, VexyError> {
    optimize(input, OptimizeOptions::new(config))
}

/// Apply data URI encoding to SVG content
#[cfg(feature = "data-uri")]
pub fn apply_datauri_encoding(svg: &str, format: &DataUriFormat) -> String {
    match format {
        DataUriFormat::Base64 => {
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine as _;
            let encoded = STANDARD.encode(svg.as_bytes());
            format!("data:image/svg+xml;base64,{}", encoded)
        },
        DataUriFormat::Enc => {
            use urlencoding::encode;
            let encoded = encode(svg);
            format!("data:image/svg+xml,{encoded}")
        },
        DataUriFormat::Unenc => {
            format!("data:image/svg+xml,{svg}")
        },
    }
}

#[cfg(not(feature = "data-uri"))]
pub fn apply_datauri_encoding(svg: &str, _format: &DataUriFormat) -> String {
    // Fallback implementation without encoding
    format!("data:image/svg+xml,{svg}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Document, Node};
    use crate::parser::config::Config;
    use crate::plugin_registry::PluginRegistry;
    use crate::Plugin;

    // Simple test plugin to remove comments
    struct RemoveCommentsPlugin;

    impl Plugin for RemoveCommentsPlugin {
        fn name(&self) -> &'static str {
            "removeComments"
        }

        fn description(&self) -> &'static str {
            "Remove comments"
        }

        fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
            // Remove comments from root children
            document.root.children.retain(|node| !matches!(node, Node::Comment(_)));
            // Remove comments from prologue
            document.prologue.retain(|node| !matches!(node, Node::Comment(_)));
            // Remove comments from epilogue
            document.epilogue.retain(|node| !matches!(node, Node::Comment(_)));
            Ok(())
        }
    }

    #[test]
    fn test_optimize_simple_svg() {
        let svg = r#"<svg width="100" height="100">
            <!-- This is a comment -->
            <rect x="10" y="10" width="50" height="50"/>
        </svg>"#;

        // Create config with removeComments plugin
        let mut config = Config::new();
        config.plugins = vec![crate::parser::config::PluginConfig::Name("removeComments".to_string())];

        // Create a registry with our test plugin
        let mut registry = PluginRegistry::new();
        registry.register("removeComments", || RemoveCommentsPlugin);

        let result = optimize(svg, OptimizeOptions::new(config).with_registry(registry)).unwrap();

        assert!(!result.data.is_empty());
        assert!(result.info.original_size > 0);
        assert!(result.info.optimized_size > 0);
        assert!(result.modern);
        // Check that comment was removed
        assert!(!result.data.contains("<!--"));
    }

    #[test]
    fn test_optimize_with_config() {
        let svg = r#"<svg><rect/></svg>"#;
        let mut config = Config::with_default_preset();
        config.js2svg.pretty = true;
        config.js2svg.indent = "    ".to_string();

        let result = optimize_with_config(svg, config).unwrap();
        assert!(!result.data.is_empty());
    }

    #[test]
    fn test_optimization_info() {
        let info = OptimizationInfo::new(1000, 800, 5, 2);

        assert_eq!(info.original_size, 1000);
        assert_eq!(info.optimized_size, 800);
        assert_eq!(info.size_reduction(), 200);
        assert!((info.compression_percentage() - 20.0).abs() < 0.01);
        assert_eq!(info.plugins_applied, 5);
        assert_eq!(info.passes, 2);
    }

    #[test]
    #[cfg(feature = "data-uri")]
    fn test_datauri_encoding() {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine as _;

        use crate::parser::config::DataUriFormat;

        let svg = "<svg><circle r=\"5\"/></svg>";

        // Test Base64 encoding
        let base64_result = apply_datauri_encoding(svg, &DataUriFormat::Base64);
        assert!(base64_result.starts_with("data:image/svg+xml;base64,"));
        let expected_base64 = STANDARD.encode(svg.as_bytes());
        assert_eq!(base64_result, format!("data:image/svg+xml;base64,{}", expected_base64));

        // Test URL encoding
        let enc_result = apply_datauri_encoding(svg, &DataUriFormat::Enc);
        assert!(enc_result.starts_with("data:image/svg+xml,"));
        assert!(enc_result.contains("%3Csvg%3E%3Ccircle%20r%3D%225%22%2F%3E%3C%2Fsvg%3E"));

        // Test unencoded
        let unenc_result = apply_datauri_encoding(svg, &DataUriFormat::Unenc);
        assert_eq!(unenc_result, format!("data:image/svg+xml,{}", svg));
    }
}
