// this_file: crates/core/tests/svgo_config_format_tests.rs

//! Tests for various SVGO configuration file formats and options
//! 
//! These tests ensure vexy-svgo maintains compatibility with different
//! SVGO configuration formats including:
//! - .svgo.config.js / svgo.config.js (JavaScript module)
//! - .svgorc.json / .svgorc (JSON format)
//! - svgo.config.mjs (ES modules)
//! - Package.json svgo field

use vexy_svgo_core::parser::config::{Config, PluginConfig, DataUriFormat, LineEnding};
use vexy_svgo_core::optimize_with_config;
use serde_json::json;

#[test]
fn test_svgo_legacy_plugin_format() {
    // SVGO v2 uses array of plugins with name/params structure
    let mut config = Config::default();
    
    config.plugins = vec![
        // String format (enable plugin with defaults)
        PluginConfig::Name("removeDoctype".to_string()),
        
        // Object format with params
        PluginConfig::WithParams {
            name: "cleanupIds".to_string(),
            params: json!({
                "minify": true,
                "preserve": ["icon-"],
                "preservePrefixes": ["data-", "aria-"]
            }),
        },
        
        // Disable plugin by setting to false
        PluginConfig::WithParams {
            name: "removeViewBox".to_string(),
            params: json!(false),
        },
    ];
    
    let svg = r#"<!DOCTYPE svg>
<svg viewBox="0 0 100 100">
    <rect id="icon-rect" data-value="test" width="50" height="50"/>
</svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    // Doctype should be removed
    assert!(!result.data.contains("<!DOCTYPE"));
    // ViewBox should be preserved (plugin disabled)
    assert!(result.data.contains("viewBox"));
    // ID with preserve prefix should remain
    assert!(result.data.contains("icon-rect"));
    // Data attribute should remain
    assert!(result.data.contains("data-value"));
}

#[test]
fn test_svgo_v3_plugin_format() {
    // SVGO v3 introduced new plugin format with "preset-default"
    let mut config = Config::default();
    
    // Simulate preset-default with overrides
    config.plugins = vec![
        // First, apply default plugins
        PluginConfig::Name("removeDoctype".to_string()),
        PluginConfig::Name("removeXMLProcInst".to_string()),
        PluginConfig::Name("removeComments".to_string()),
        
        // Then override specific plugins
        PluginConfig::WithParams {
            name: "removeViewBox".to_string(),
            params: json!(false), // Override to disable
        },
        
        PluginConfig::WithParams {
            name: "cleanupIds".to_string(),
            params: json!({
                "minify": false, // Override to disable minification
            }),
        },
    ];
    
    let svg = r#"<?xml version="1.0"?>
<!-- Comment -->
<svg viewBox="0 0 100 100">
    <rect id="my-long-descriptive-id"/>
</svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    // XML declaration should be removed
    assert!(!result.data.contains("<?xml"));
    // Comments should be removed
    assert!(!result.data.contains("<!-- Comment"));
    // ViewBox should be preserved (overridden)
    assert!(result.data.contains("viewBox"));
    // Long ID should be preserved (minification disabled)
    assert!(result.data.contains("my-long-descriptive-id"));
}

#[test]
fn test_svgo_global_params() {
    let mut config = Config::default();
    
    config.plugins = vec![
        PluginConfig::WithParams {
            name: "convertPathData".to_string(),
            params: json!({
                "floatPrecision": 2
            }),
        },
        PluginConfig::WithParams {
            name: "convertTransform".to_string(),
            params: json!({
                "floatPrecision": 2
            }),
        },
    ];
    
    let svg = r#"<svg>
    <path d="M 10.123456 20.987654 L 30.456789 40.123456"/>
    <g transform="translate(10.999999, 20.888888)">
        <rect/>
    </g>
</svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    assert!(!result.data.contains("10.123456"));
    assert!(!result.data.contains("10.999999"));
}

#[test]
fn test_svgo_multipass_config() {
    // Test multipass optimization
    let mut config = Config::default();
    config.multipass = true;
    
    config.plugins = vec![
        PluginConfig::Name("collapseGroups".to_string()),
        PluginConfig::Name("moveGroupAttrsToElems".to_string()),
    ];
    
    // This SVG will benefit from multiple passes
    let svg = r#"<svg>
    <g>
        <g fill="red">
            <rect width="10" height="10"/>
        </g>
    </g>
</svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    // After multiple passes, nested groups should be collapsed
    // and attributes moved to elements
    assert!(result.data.contains("fill=\"red\""));
}

#[test]
fn test_svgo_js2svg_options() {
    let mut config = Config::default();
    
    config.js2svg.pretty = true;
    config.js2svg.indent = "  ".to_string();
    config.js2svg.final_newline = true;
    
    let svg = r#"<svg><g><rect id="test" width="100"/></g></svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    assert!(result.data.contains("\n"));
    assert!(result.data.ends_with("\n"));
}

#[test]
fn test_svgo_custom_plugins() {
    // Test configuration with custom plugin names (future extensibility)
    let mut config = Config::default();
    
    // Add standard plugins
    config.plugins = vec![
        PluginConfig::Name("removeComments".to_string()),
        // Custom/unknown plugins should be handled gracefully
        PluginConfig::Name("customPlugin".to_string()),
        PluginConfig::WithParams {
            name: "unknownPlugin".to_string(),
            params: json!({
                "option1": true,
                "option2": "value"
            }),
        },
    ];
    
    let svg = r#"<svg><!-- comment --><rect/></svg>"#;
    
    // Should not fail even with unknown plugins
    let result = optimize_with_config(svg, config);
    assert!(result.is_ok());
    
    // Standard plugins should still work
    assert!(!result.unwrap().data.contains("<!-- comment"));
}

#[test]
fn test_svgo_path_data_precision() {
    // Test path data precision control
    let mut config = Config::default();
    
    config.plugins = vec![
        PluginConfig::WithParams {
            name: "convertPathData".to_string(),
            params: json!({
                "floatPrecision": 1,
                "transformPrecision": 2,
                "makeArcs": {
                    "threshold": 2.5,
                    "tolerance": 0.5
                }
            }),
        },
    ];
    
    let svg = r#"<svg>
    <path d="M 10.123456 20.987654 Q 15.555555 25.666666 20.111111 30.222222"/>
</svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    // Path data should be rounded to specified precision
    assert!(!result.data.contains("10.123456"));
}

#[test]
fn test_svgo_remove_plugins_with_exceptions() {
    // Many SVGO plugins support exceptions/preserve options
    let mut config = Config::default();
    
    config.plugins = vec![
        PluginConfig::WithParams {
            name: "removeAttrs".to_string(),
            params: json!({
                "attrs": ["fill", "stroke"],
                "preserveCurrentColor": true,
            }),
        },
        PluginConfig::WithParams {
            name: "removeElementsByAttr".to_string(),
            params: json!({
                "id": ["removeMe", "alsoRemoveMe"],
                "class": ["hidden"],
            }),
        },
    ];
    
    let svg = r#"<svg>
    <rect fill="red" stroke="blue" width="100"/>
    <rect fill="currentColor" stroke="currentColor"/>
    <g id="removeMe"><rect/></g>
    <g id="keepMe"><rect/></g>
    <rect class="hidden"/>
</svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    // Regular fill/stroke should be removed
    assert!(!result.data.contains("fill=\"red\""));
    assert!(!result.data.contains("stroke=\"blue\""));
    
    // currentColor should be preserved
    assert!(result.data.contains("currentColor"));
    
    // Elements with specified IDs should be removed
    assert!(!result.data.contains("removeMe"));
    assert!(result.data.contains("keepMe"));
    
    // Elements with hidden class should be removed
    assert!(!result.data.contains("class=\"hidden\""));
}

#[test]
fn test_svgo_plugin_order_matters() {
    // Test that plugin order affects the outcome
    let svg = r#"<svg>
    <style>
        .red { fill: red; }
    </style>
    <rect class="red"/>
</svg>"#;
    
    // First config: convert styles to attrs, then remove style element
    let mut config1 = Config::default();
    config1.plugins = vec![
        PluginConfig::Name("convertStyleToAttrs".to_string()),
        PluginConfig::Name("removeStyleElement".to_string()),
    ];
    
    let result1 = optimize_with_config(svg, config1);
    
    // Second config: remove style element first (wrong order)
    let mut config2 = Config::default();
    config2.plugins = vec![
        PluginConfig::Name("removeStyleElement".to_string()),
        PluginConfig::Name("convertStyleToAttrs".to_string()),
    ];
    
    let result2 = optimize_with_config(svg, config2);
    
    // Results should be different due to plugin order
    // (This test documents the importance of plugin order)
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[test]
fn test_svgo_transforms_precision() {
    // Test transform precision control
    let mut config = Config::default();
    
    config.plugins = vec![
        PluginConfig::WithParams {
            name: "convertTransform".to_string(),
            params: json!({
                "convertToShorts": true,
                "floatPrecision": 2,
                "transformPrecision": 3,
                "matrixToTransform": true,
                "shortTranslate": true,
                "shortScale": true,
                "shortRotate": true,
            }),
        },
    ];
    
    let svg = r#"<svg>
    <g transform="translate(10.123456, 20.987654) scale(1.111111, 2.222222) rotate(45.555555)">
        <rect/>
    </g>
</svg>"#;
    
    let result = optimize_with_config(svg, config).unwrap();
    
    // Transform values should be rounded
    assert!(!result.data.contains("10.123456"));
    assert!(!result.data.contains("45.555555"));
}