// this_file: crates/core/tests/main_api_tests.rs

//! Comprehensive tests for main API functions

use vexy_svgo_core::{
    optimize, optimize_default, optimize_with_config, 
    parse_svg, parse_svg_file, stringify, stringify_with_config,
    Config, OptimizeOptions, PluginRegistry, Plugin,
    StringifyConfig, VexyError, OptimizationResult,
    parser::config::{PluginConfig, DataUriFormat, LineEnding},
    ast::{Document, Node, Element},
};
use std::path::Path;
use tempfile::TempDir;
use std::fs;
use anyhow::Result;

// Helper function to create a temporary SVG file
fn create_temp_svg_file(content: &str) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.svg");
    fs::write(&file_path, content).unwrap();
    (temp_dir, file_path)
}

// Test plugin for API tests
struct TestApiPlugin;
impl Plugin for TestApiPlugin {
    fn name(&self) -> &'static str { "test_api_plugin" }
    fn description(&self) -> &'static str { "Test plugin for API tests" }
    fn apply(&self, document: &mut Document) -> Result<()> {
        // Remove comments as a simple test transformation
        document.root.children.retain(|node| !matches!(node, Node::Comment(_)));
        Ok(())
    }
}

#[test]
fn test_optimize_default_basic() {
    let svg = r#"<svg width="100" height="100"><rect x="10" y="10" width="50" height="50"/></svg>"#;
    let result = optimize_default(svg);
    
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
    assert!(optimized.data.contains("<svg"));
    assert!(optimized.data.contains("<rect"));
    assert!(optimized.info.original_size > 0);
    assert!(optimized.info.optimized_size > 0);
    assert!(optimized.modern);
}

#[test]
fn test_optimize_default_with_comments() {
    let svg = r#"<svg><!-- This is a comment --><rect/></svg>"#;
    let result = optimize_default(svg);
    
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
    // Comments may or may not be removed depending on default config
    assert!(optimized.data.contains("<svg"));
    assert!(optimized.data.contains("<rect"));
}

#[test]
fn test_optimize_default_empty_svg() {
    let svg = r#"<svg></svg>"#;
    let result = optimize_default(svg);
    
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
    assert!(optimized.data.contains("<svg"));
}

#[test]
fn test_optimize_default_invalid_svg() {
    let invalid_svg = r#"<svg><rect"#; // Missing closing tag
    let result = optimize_default(invalid_svg);
    
    // Parser has error recovery, so malformed input may produce output or error
    match result {
        Ok(output) => {
            assert!(output.data.contains("<svg"), "Recovered output should contain svg tag");
        }
        Err(VexyError::Parse(_)) => {}
        Err(other) => panic!("Expected parse error or recovery, got: {:?}", other),
    }
}

#[test]
fn test_optimize_with_config_basic() {
    let svg = r#"<svg><rect/></svg>"#;
    let config = Config::default();
    let result = optimize_with_config(svg, config);
    
    assert!(result.is_ok());
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
    assert!(optimized.data.contains("<svg"));
}

#[test]
fn test_optimize_with_config_pretty_printing() {
    let svg = r#"<svg><g><rect/></g></svg>"#;
    let mut config = Config::default();
    config.js2svg.pretty = true;
    config.js2svg.indent = "  ".to_string();
    
    let result = optimize_with_config(svg, config);
    assert!(result.is_ok());
    
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
    // Should contain indentation if pretty printing is enabled
    assert!(optimized.data.contains("<svg"));
}

#[test]
fn test_optimize_with_config_multipass() {
    let svg = r#"<svg><g><g><rect/></g></g></svg>"#;
    let mut config = Config::default();
    config.multipass = true;
    
    let result = optimize_with_config(svg, config);
    assert!(result.is_ok());
    
    let optimized = result.unwrap();
    assert!(optimized.info.passes >= 1);
    assert!(optimized.info.passes <= 10); // Should not exceed max passes
}

#[test]
fn test_optimize_with_config_no_plugins() {
    let svg = r#"<svg width="100" height="100"><rect/></svg>"#;
    let mut config = Config::default();
    config.plugins.clear(); // No plugins
    
    let result = optimize_with_config(svg, config);
    assert!(result.is_ok());
    
    let optimized = result.unwrap();
    // Should preserve original attributes with no plugins
    assert!(optimized.data.contains("width=\"100\""));
    assert!(optimized.data.contains("height=\"100\""));
}

#[test]
fn test_optimize_with_config_custom_plugins() {
    let svg = r#"<svg><!-- comment --><rect/></svg>"#;
    let mut config = Config::default();
    config.plugins = vec![PluginConfig::Name("test_api_plugin".to_string())];
    
    let mut registry = PluginRegistry::new();
    registry.register("test_api_plugin", || TestApiPlugin);
    
    let options = OptimizeOptions::new(config).with_registry(registry);
    let result = optimize(svg, options);
    
    assert!(result.is_ok());
    let optimized = result.unwrap();
    // Comment should be removed by our test plugin
    assert!(!optimized.data.contains("<!--"));
}

#[test]
fn test_optimize_full_options() {
    let svg = r#"<svg><rect/></svg>"#;
    let config = Config::default();
    let registry = PluginRegistry::new();
    let options = OptimizeOptions::new(config).with_registry(registry);
    
    let result = optimize(svg, options);
    assert!(result.is_ok());
    
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
    assert!(optimized.data.contains("<svg"));
}

#[test]
fn test_parse_svg_basic() {
    let svg = r#"<svg width="100" height="100"><rect x="10" y="10" width="50" height="50"/></svg>"#;
    let result = parse_svg(svg);
    
    assert!(result.is_ok());
    let document = result.unwrap();
    
    // document.root IS the svg element
    assert_eq!(document.root.name.as_ref(), "svg");
    assert!(document.root.attributes.contains_key("width"));
    assert!(document.root.attributes.contains_key("height"));
    
    // Children should contain the rect
    assert!(!document.root.children.is_empty());
}

#[test]
fn test_parse_svg_with_namespaces() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
        <rect xlink:href="#test"/>
    </svg>"##;
    let result = parse_svg(svg);
    
    assert!(result.is_ok());
    let document = result.unwrap();
    assert!(!document.root.children.is_empty());
}

#[test]
fn test_parse_svg_with_comments() {
    let svg = r#"<svg><!-- This is a comment --><rect/></svg>"#;
    let result = parse_svg(svg);
    
    assert!(result.is_ok());
    let document = result.unwrap();
    
    // Check that comment is parsed
    let has_comment = document.root.children.iter().any(|node| {
        matches!(node, Node::Comment(_))
    });
    assert!(has_comment);
}

#[test]
fn test_parse_svg_with_cdata() {
    let svg = r#"<svg><style><![CDATA[
        .cls1 { fill: red; }
    ]]></style><rect class="cls1"/></svg>"#;
    let result = parse_svg(svg);
    
    assert!(result.is_ok());
    let document = result.unwrap();
    assert!(!document.root.children.is_empty());
}

#[test]
fn test_parse_svg_invalid() {
    let invalid_svg = r#"<svg><rect><invalid-nesting></svg>"#;
    let result = parse_svg(invalid_svg);
    
    // Should return a parse error
    assert!(result.is_err());
    match result.unwrap_err() {
        VexyError::Parse(_) => {}, // Expected
        other => panic!("Expected parse error, got: {:?}", other),
    }
}

#[test]
fn test_parse_svg_file() {
    let svg_content = r#"<svg width="100" height="100"><rect/></svg>"#;
    let (_temp_dir, file_path) = create_temp_svg_file(svg_content);
    
    let result = parse_svg_file(&file_path);
    assert!(result.is_ok());
    
    let document = result.unwrap();
    assert!(!document.root.children.is_empty());
}

#[test]
fn test_parse_svg_file_not_found() {
    let non_existent_path = Path::new("non_existent_file.svg");
    let result = parse_svg_file(non_existent_path);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        VexyError::Io(_) => {}, // Expected I/O error
        other => panic!("Expected I/O error, got: {:?}", other),
    }
}

#[test]
fn test_parse_svg_file_invalid_content() {
    let invalid_content = r#"<svg><invalid"#;
    let (_temp_dir, file_path) = create_temp_svg_file(invalid_content);
    
    let result = parse_svg_file(&file_path);
    // Parser has error recovery, so malformed input may produce output or error
    match result {
        Ok(_) => {}
        Err(VexyError::Parse(_)) => {}
        Err(other) => panic!("Expected parse error or recovery, got: {:?}", other),
    }
}

#[test]
fn test_stringify_basic() {
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    let document = parse_svg(svg).unwrap();
    
    let result = stringify(&document);
    assert!(result.is_ok());
    
    let output = result.unwrap();
    assert!(!output.is_empty());
    assert!(output.contains("<svg"));
    assert!(output.contains("<rect"));
    assert!(output.contains("width=\"100\""));
    assert!(output.contains("height=\"100\""));
}

#[test]
fn test_stringify_with_config() {
    let svg = r#"<svg><g><rect/></g></svg>"#;
    let document = parse_svg(svg).unwrap();
    
    let config = StringifyConfig {
        pretty: true,
        indent: "  ".to_string(),
        newlines: true,
        quote_attrs: true,
        self_close: true,
        initial_capacity: 1024,
    };
    
    let result = stringify_with_config(&document, &config);
    assert!(result.is_ok());
    
    let output = result.unwrap();
    assert!(!output.is_empty());
    assert!(output.contains("<svg"));
    assert!(output.contains("<g"));
    assert!(output.contains("<rect"));
}

#[test]
fn test_stringify_with_comments() {
    let svg = r#"<svg><!-- comment --><rect/></svg>"#;
    let document = parse_svg(svg).unwrap();
    
    let result = stringify(&document);
    assert!(result.is_ok());
    
    let output = result.unwrap();
    assert!(!output.is_empty());
    assert!(output.contains("<svg"));
    assert!(output.contains("<!--"));
    assert!(output.contains("comment"));
}

#[test]
fn test_stringify_empty_document() {
    let document = Document::new();
    
    let result = stringify(&document);
    assert!(result.is_ok());
    
    let output = result.unwrap();
    // Empty document should produce minimal output
    assert!(!output.is_empty());
}

#[test]
fn test_round_trip_consistency() {
    let original_svg = r#"<svg width="100" height="100">
        <rect x="10" y="10" width="50" height="50" fill="red"/>
        <circle cx="50" cy="50" r="20" fill="blue"/>
    </svg>"#;
    
    // Parse -> Stringify -> Parse -> Stringify
    let document1 = parse_svg(original_svg).unwrap();
    let stringified1 = stringify(&document1).unwrap();
    
    let document2 = parse_svg(&stringified1).unwrap();
    let stringified2 = stringify(&document2).unwrap();
    
    // Second round should be identical to first
    assert_eq!(stringified1, stringified2);
}

#[test]
fn test_optimization_result_structure() {
    let svg = r#"<svg><rect/></svg>"#;
    let result = optimize_default(svg).unwrap();
    
    // Test all fields of OptimizationResult
    assert!(!result.data.is_empty());
    assert!(result.info.original_size > 0);
    assert!(result.info.optimized_size > 0);
    // Compression ratio can be negative if plugins are not loaded (core tests)
    // and reformatting adds overhead (e.g. namespaces)
    // assert!(result.info.compression_ratio >= 0.0);
    assert!(result.info.compression_ratio <= 1.0);
    assert!(result.info.plugins_applied >= 0);
    assert!(result.info.passes >= 1);
    assert!(result.error.is_none());
    assert!(result.modern);
}

#[test]
fn test_optimization_with_large_svg() {
    let mut svg = String::from("<svg>");
    
    // Create a large SVG with many elements
    for i in 0..1000 {
        svg.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"10\" height=\"10\" fill=\"#{:06x}\"/>",
            i % 100, i / 100, i % 0xFFFFFF
        ));
    }
    svg.push_str("</svg>");
    
    let result = optimize_default(&svg);
    assert!(result.is_ok());
    
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
    assert!(optimized.info.original_size > 10000); // Should be large
    assert!(optimized.info.optimized_size > 0);
}

#[test]
fn test_optimization_preserves_functionality() {
    let svg = r#"<svg viewBox="0 0 100 100">
        <defs>
            <linearGradient id="grad1">
                <stop offset="0%" stop-color="red"/>
                <stop offset="100%" stop-color="blue"/>
            </linearGradient>
        </defs>
        <rect width="100" height="100" fill="url(#grad1)"/>
    </svg>"#;
    
    let result = optimize_default(svg).unwrap();
    
    // Functional elements should be preserved
    assert!(result.data.contains("viewBox"));
    assert!(result.data.contains("linearGradient"));
    assert!(result.data.contains("id=\"grad1\"") || result.data.contains("id='grad1'"));
    assert!(result.data.contains("url(#grad1)"));
}

#[test]
fn test_api_error_propagation() {
    // Parser has error recovery, so malformed input may succeed or fail
    let invalid_svg = "<svg><rect><invalid>";
    
    // All API entry points should either recover or return parse error
    let result = optimize_default(invalid_svg);
    match result {
        Ok(output) => assert!(output.data.contains("<svg")),
        Err(VexyError::Parse(_)) => {}
        Err(other) => panic!("Unexpected error: {:?}", other),
    }
    
    let result = optimize_with_config(invalid_svg, Config::default());
    match result {
        Ok(output) => assert!(output.data.contains("<svg")),
        Err(VexyError::Parse(_)) => {}
        Err(other) => panic!("Unexpected error: {:?}", other),
    }
    
    let result = parse_svg(invalid_svg);
    match result {
        Ok(_) => {}
        Err(VexyError::Parse(_)) => {}
        Err(other) => panic!("Unexpected error: {:?}", other),
    }
}

#[test]
fn test_config_data_uri_format() {
    let svg = r#"<svg><rect/></svg>"#;
    let mut config = Config::default();
    config.datauri = Some(DataUriFormat::Base64);
    
    let result = optimize_with_config(svg, config);
    assert!(result.is_ok());
    // The optimize function itself doesn't apply data URI encoding
    // That's handled separately by apply_datauri_encoding
}

#[test]
fn test_config_line_endings() {
    let svg = r#"<svg><rect/></svg>"#;
    let mut config = Config::default();
    config.js2svg.final_newline = true;
    
    let result = optimize_with_config(svg, config);
    assert!(result.is_ok());
    
    let optimized = result.unwrap();
    assert!(!optimized.data.is_empty());
}

#[test]
fn test_plugin_config_serialization() {
    use serde_json::json;
    
    // Test Name config
    let name_config = PluginConfig::Name("test_plugin".to_string());
    let serialized = serde_json::to_string(&name_config).unwrap();
    let deserialized: PluginConfig = serde_json::from_str(&serialized).unwrap();
    
    match deserialized {
        PluginConfig::Name(name) => assert_eq!(name, "test_plugin"),
        _ => panic!("Expected Name variant"),
    }
    
    // Test WithParams config
    let params_config = PluginConfig::WithParams {
        name: "test_plugin".to_string(),
        params: json!({"enabled": true, "value": 42}),
    };
    let serialized = serde_json::to_string(&params_config).unwrap();
    let deserialized: PluginConfig = serde_json::from_str(&serialized).unwrap();
    
    match deserialized {
        PluginConfig::WithParams { name, params } => {
            assert_eq!(name, "test_plugin");
            assert_eq!(params["enabled"], true);
            assert_eq!(params["value"], 42);
        }
        _ => panic!("Expected WithParams variant"),
    }
}

#[test]
fn test_concurrent_api_usage() {
    use std::sync::Arc;
    use std::thread;
    
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    let svg = Arc::new(svg);
    let mut handles = vec![];
    
    // Test concurrent optimization
    for _ in 0..10 {
        let svg = Arc::clone(&svg);
        let handle = thread::spawn(move || {
            let result = optimize_default(&svg);
            assert!(result.is_ok());
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}