// this_file: examples/rust-integration.rs

//! Examples of using SVGN as a Rust library

use vexy_svgo_core::{
    Config, OptimizeOptions, OptimizationResult,
    optimize, optimize_default, optimize_with_config,
    parallel::ParallelConfig,
    features::{Feature, enable_feature},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    basic_usage()?;
    custom_configuration()?;
    parallel_optimization()?;
    plugin_configuration()?;
    streaming_large_files()?;
    feature_management()?;
    error_handling()?;
    
    Ok(())
}

/// Basic optimization with default settings
fn basic_usage() -> Result<(), Box<dyn std::error::Error>> {
    let svg = r#"
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <!-- This is a comment -->
            <rect x="10" y="10" width="80" height="80" fill="red"/>
        </svg>
    "#;
    
    // Optimize with default configuration
    let result = optimize_default(svg)?;
    
    println!("Original size: {} bytes", result.info.original_size);
    println!("Optimized size: {} bytes", result.info.optimized_size);
    println!("Reduction: {:.1}%", result.info.compression_percentage());
    println!("Optimized SVG:\n{}", result.data);
    
    Ok(())
}

/// Custom configuration example
fn custom_configuration() -> Result<(), Box<dyn std::error::Error>> {
    let svg = include_str!("../testdata/complex.svg");
    
    // Create custom configuration
    let mut config = Config::default();
    config.multipass = true;
    config.js2svg.pretty = true;
    config.js2svg.indent = 2;
    
    // Configure specific plugins
    config.plugins = vec![
        vexy_svgo_core::config::PluginConfig::Name("removeComments".to_string()),
        vexy_svgo_core::config::PluginConfig::Name("removeEmptyAttrs".to_string()),
        vexy_svgo_core::config::PluginConfig::WithParams {
            name: "convertColors".to_string(),
            params: serde_json::json!({
                "currentColor": true,
                "names2hex": true,
            }),
        },
        vexy_svgo_core::config::PluginConfig::WithParams {
            name: "cleanupIds".to_string(),
            params: serde_json::json!({
                "minify": true,
                "preserve": ["icon-"],
            }),
        },
    ];
    
    let result = optimize_with_config(svg, config)?;
    println!("Optimized with custom config: {} bytes", result.info.optimized_size);
    
    Ok(())
}

/// Parallel optimization for large files
fn parallel_optimization() -> Result<(), Box<dyn std::error::Error>> {
    // Enable parallel processing feature
    enable_feature(Feature::ParallelProcessing)?;
    
    let large_svg = generate_large_svg();
    
    // Configure parallel processing
    let parallel_config = ParallelConfig {
        size_threshold: 512 * 1024,  // 512KB
        element_threshold: 500,
        num_threads: 4,
    };
    
    let options = OptimizeOptions::new(Config::default())
        .with_parallel(parallel_config);
    
    let start = std::time::Instant::now();
    let result = optimize(&large_svg, options)?;
    let duration = start.elapsed();
    
    println!("Parallel optimization completed in {:?}", duration);
    println!("Elements processed: {}", result.info.plugins_applied);
    
    Ok(())
}

/// Configure individual plugins
fn plugin_configuration() -> Result<(), Box<dyn std::error::Error>> {
    use vexy_svgo_core::plugin_registry::{create_default_registry, PluginConfig};
    
    let svg = r#"<svg><g id="layer1"><rect fill="#ff0000"/></g></svg>"#;
    
    // Create custom plugin registry
    let mut registry = create_default_registry();
    
    // Configure plugins programmatically
    let plugin_configs = vec![
        PluginConfig {
            name: "convertColors".to_string(),
            params: serde_json::json!({ "rgb2hex": true }),
            enabled: true,
        },
        PluginConfig {
            name: "cleanupIds".to_string(),
            params: serde_json::json!({ "minify": false }),
            enabled: true,
        },
    ];
    
    let options = OptimizeOptions::new(Config::default())
        .with_registry(registry);
    
    let result = optimize(svg, options)?;
    println!("Custom plugin configuration result: {}", result.data);
    
    Ok(())
}

/// Stream processing for very large files
fn streaming_large_files() -> Result<(), Box<dyn std::error::Error>> {
    use vexy_svgo_core::parser::{parse_svg_file, StreamingConfig};
    use vexy_svgo_core::stringifier::{StreamingStringifier, StringifyConfig};
    use std::fs::File;
    use std::io::BufWriter;
    
    // Parse large file with streaming
    let streaming_config = StreamingConfig {
        buffer_size: 64 * 1024,     // 64KB buffer
        max_depth: 100,
        max_text_length: 1024 * 1024, // 1MB max text
        enable_entity_expansion: true,
    };
    
    // This would work with a real file
    // let document = parse_svg_file( PROTECTED_28_ )?;
    
    // For example, let PROTECTED_48_ a> Visitor< PROTECTED_49_ a>) -> anyhow::Result<()> {
            self.element_count += 1;
            Ok(())
        }
        
        fn visit_text(&mut self, _text: &mut String) -> anyhow::Result<()> {
            self.text_count += 1;
            Ok(())
        }
    }
    
    pub fn count_document_nodes(document: &mut Document) -> (usize, usize) {
        let mut visitor = CountingVisitor {
            element_count: 0,
            text_count: 0,
        };
        
        let _ = visitor.visit_document(document);
        
        (visitor.element_count, visitor.text_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_optimization() {
        let svg = r#"<svg><!-- comment --><rect/></svg>"#;
        let result = optimize_default(svg).unwrap();
        assert!(!result.data.contains("<!--"));
        assert!(result.info.optimized_size < result.info.original_size);
    }
}
    if is_feature_enabled(Feature::DebugMode) {
        println!("Debug mode is enabled");
    }
    
    // Use feature macro
    vexy_svgo_core::if_feature!(
        Feature::ParallelProcessing,
        println!("Parallel processing is available"),
        println!("Running in single-threaded mode")
    );
    
    Ok(())
}

/// Error handling examples
fn error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let invalid_svg = "<svg><invalid></svg>";
    
    match optimize_default(invalid_svg) {
        Ok(result) => {
            println!("Optimization succeeded: {}", result.data);
        }
        Err(e) => {
            eprintln!("Optimization failed: {}", e);
            
            // Handle specific error types
            use vexy_svgo_core::OptimizeError;
            match e {
                OptimizeError::ParseError(parse_err) => {
                    eprintln!("Parse error: {}", parse_err);
                }
                OptimizeError::PluginError(plugin_err) => {
                    eprintln!("Plugin error: {}", plugin_err);
                }
                _ => {
                    eprintln!("Other error: {}", e);
                }
            }
        }
    }
    
    Ok(())
}

/// Generate a large SVG for testing
fn generate_large_svg() -> String {
    let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1000 1000">"#);
    
    for i in 0..1000 {
        svg.push_str(&format!(
            r#"<rect x="{}" y="{}" width="10" height="10" fill="#{:06x}"/>"#,
            i * 10 % 1000,
            (i * 10) / 1000 * 10,
            i * 1000
        ));
    }
    
    svg.push_str("</svg>");
    svg
}

/// Advanced: Custom visitor for AST manipulation
mod custom_visitor {
    use vexy_svgo_core::visitor::Visitor;
    use vexy_svgo_core::ast::{Document, Element, Node};
    
    pub struct CountingVisitor {
        pub element_count: usize,
        pub text_count: usize,
    }
    
    impl<'a> Visitor<'a> for CountingVisitor {
        fn visit_element_enter(&mut self, _element: &mut Element<'a>) -> anyhow::Result<()> {
            self.element_count += 1;
            Ok(())
        }
        
        fn visit_text(&mut self, _text: &mut String) -> anyhow::Result<()> {
            self.text_count += 1;
            Ok(())
        }
    }
    
    pub fn count_document_nodes(document: &mut Document) -> (usize, usize) {
        let mut visitor = CountingVisitor {
            element_count: 0,
            text_count: 0,
        };
        
        let _ = visitor.visit_document(document);
        
        (visitor.element_count, visitor.text_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_optimization() {
        let svg = r#"<svg><!-- comment --><rect/></svg>"#;
        let result = optimize_default(svg).unwrap();
        assert!(!result.data.contains("<!--"));
        assert!(result.info.optimized_size < result.info.original_size);
    }
}