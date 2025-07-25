// this_file: tests/integration_test.rs

//! Integration tests for svgn

use vexy_svgo::config::{DataUriFormat, Js2SvgOptions, LineEnding, QuoteAttrsStyle};
use vexy_svgo::{optimize, Config, OptimizeOptions, PluginConfig};

#[test]
fn test_full_optimization_pipeline() {
    let svg = r#"<?xml version="1.0"?>
    <!-- This is a comment -->
    <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" 
         enable-background="new 0 0 100 100">
        <metadata>Some metadata</metadata>
        <title>Test SVG</title>
        <defs>
            <linearGradient id="myVeryLongGradientId">
                <stop offset="0%" stop-color="red"/>
            </linearGradient>
            <linearGradient id="unused-gradient">
                <stop offset="0%" stop-color="blue"/>
            </linearGradient>
        </defs>
        <rect x="10
20" y="30  40" fill="url(#myVeryLongGradientId)" 
              width="50" height="50" class="  foo   bar  "/>
    </svg>"#;

    let config = Config {
        plugins: vec![
            PluginConfig::new("cleanupAttrs".to_string()),
            PluginConfig::new("cleanupEnableBackground".to_string()),
            PluginConfig::new("cleanupIds".to_string()),
            PluginConfig::new("removeComments".to_string()),
            PluginConfig::new("removeMetadata".to_string()),
            PluginConfig::new("removeTitle".to_string()),
        ],
        multipass: false,
        js2svg: Js2SvgOptions {
            pretty: false,
            indent: 2,
            quote_attrs: QuoteAttrsStyle::Always,
            self_closing: true,
            eol: LineEnding::default(),
            final_newline: false,
            ..Js2SvgOptions::default()
        },
        path: None,
        datauri: None,
        parser: Default::default(),
    };

    let options = OptimizeOptions::new(config);
    let result = optimize(svg, options).unwrap();

    // Debug: print the optimized SVG
    println!("Optimized SVG:\n{}", result.data);

    // Verify optimizations were applied
    assert!(!result.data.contains("<!-- This is a comment -->"));
    assert!(!result.data.contains("<metadata>"));
    assert!(!result.data.contains("<title>"));
    assert!(!result.data.contains("enable-background"));
    assert!(!result.data.contains("unused-gradient"));
    assert!(!result.data.contains("myVeryLongGradientId")); // Should be minified
                                                            // Accept url(#b) or url(#a) depending on optimization result
    assert!(result.data.contains("url(#b)") || result.data.contains("url(#a)"));
    assert!(result.data.contains(r#"x="10 20""#)); // Newline replaced with space
    assert!(result.data.contains(r#"y="30 40""#)); // Multiple spaces reduced
    assert!(result.data.contains(r#"class="foo bar""#)); // Trimmed and cleaned

    // Check optimization info
    assert!(result.info.original_size > 0);
    assert!(result.info.optimized_size < result.info.original_size);
    assert!(result.info.compression_ratio > 0.0);
}

#[test]
fn test_default_preset_pipeline() {
    // Test the default preset of plugins (equivalent to SVGO PROTECTED_69_ s expect it to either succeed with fixes or fail gracefully
    match result {
        Ok(r) => {
            // If it succeeds, it should have attempted to fix the markup
            assert!(!r.data.is_empty());
        }
        Err(_) => {
            // If it fails, that PROTECTED_70_ s valid
    let base64_part = result_base64
        .data
        .strip_prefix("data:image/svg+xml;base64,")
        .unwrap();
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let decoded = STANDARD.decode(base64_part).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    assert!(decoded_str.contains("<svg"));
    assert!(decoded_str.contains("<circle"));

    // Test URL encoding
    let config_enc = Config {
        plugins: vec![PluginConfig::new("removeMetadata".to_string())],
        datauri: Some(DataUriFormat::Enc),
        ..Default::default()
    };

    let result_enc = optimize(svg, OptimizeOptions::new(config_enc)).unwrap();
    assert!(result_enc.data.starts_with("data:image/svg+xml,"));
    assert!(result_enc.data.contains("%3Csvg"));
    assert!(result_enc.data.contains("%3Ccircle"));

    // Test unencoded
    let config_unenc = Config {
        plugins: vec![PluginConfig::new("removeMetadata".to_string())],
        datauri: Some(DataUriFormat::Unenc),
        ..Default::default()
    };

    let result_unenc = optimize(svg, OptimizeOptions::new(config_unenc)).unwrap();
    assert!(result_unenc.data.starts_with("data:image/svg+xml,"));
    assert!(result_unenc.data.contains("<svg"));
    assert!(result_unenc.data.contains("<circle"));
}
        ],
        multipass: true,
        js2svg: Js2SvgOptions {
            pretty: false,
            indent: 2,
            quote_attrs: QuoteAttrsStyle::Always,
            self_closing: true,
            eol: LineEnding::default(),
            final_newline: false,
            ..Js2SvgOptions::default()
        },
        path: None,
        datauri: None,
        parser: Default::default(),
    };

    let options = OptimizeOptions::new(config);
    let result = optimize(svg, options).unwrap();

    // Debug print
    println!("Optimized SVG:\n{}", result.data);

    // Verify comprehensive optimizations
    assert!(!result.data.contains("<!-- SVG comment -->"));
    assert!(!result.data.contains("<metadata>"));
    assert!(!result.data.contains("<title>"));
    assert!(!result.data.contains("enable-background"));
    assert!(!result.data.contains("veryLongGradientName")); // Should be minified
    assert!(!result.data.contains("unusedClipPath")); // Should be removed
    assert!(!result.data.contains("width=\"200\"")); // Should be removed due to viewBox
    assert!(!result.data.contains("height=\"100\"")); // Should be removed due to viewBox
    assert!(!result.data.contains("fill=\"\"")); // Empty fill should be removed
                                                 // Accept transform removal, or tolerate quote/whitespace variation. Robust check:
    use regex::Regex;
    let re = Regex::new(r#"transform\s*=\s*['\"]translate\(0,0\)['\"]"#).unwrap();
    assert!(
        !re.is_match(&result.data),
        "Output should not contain identity transform: {:?}",
        result.data
    );
    assert!(!result.data.contains("my-class  other-class")); // Classes should be cleaned
    assert!(result.data.contains("fill=\"#f00\"")); // Color converted to hex
    assert!(result.data.contains("viewBox=\"0 0 200 100\"")); // ViewBox preserved
    assert!(result.data.contains("<circle")); // Ellipse should be converted to circle
    assert!(!result.data.contains("<ellipse")); // No ellipse should remain

    // Check compression achieved
    assert!(result.info.original_size > 0);
    assert!(result.info.optimized_size < result.info.original_size);
    assert!(result.info.compression_ratio > 0.3); // Should achieve significant compression
}

#[test]
fn test_error_handling_invalid_svg() {
    let invalid_svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
        <rect unclosed element
    </svg>"#;

    let config = Config::default();
    let options = OptimizeOptions::new(config);

    // Should handle invalid SVG gracefully
    let result = optimize(invalid_svg, options);

    // Depending on implementation, this might error or attempt to fix
    // For now, let PROTECTED_68_ s also acceptable for malformed input
            // The error should be informative
        }
    }
}

#[test]
fn test_pretty_print_formatting() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g><rect width="50" height="50"/><circle r="25"/></g></svg>"#;

    let config = Config {
        plugins: vec![],
        multipass: false,
        js2svg: Js2SvgOptions {
            pretty: true,
            indent: 2,
            quote_attrs: QuoteAttrsStyle::Always,
            self_closing: true,
            eol: LineEnding::default(),
            final_newline: false,
            ..Js2SvgOptions::default()
        },
        path: None,
        datauri: None,
        parser: Default::default(),
    };

    let options = OptimizeOptions::new(config);
    let result = optimize(svg, options).unwrap();

    // Should be pretty-printed with proper indentation
    assert!(result.data.contains("  <g>"));
    assert!(result.data.contains("    <rect"));
    assert!(result.data.contains("    <circle"));
    assert!(result.data.contains("  </g>"));
    assert!(result.data.trim_end().ends_with("</svg>"));
}

#[test]
fn test_datauri_output_formats() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
        <circle cx="50" cy="50" r="40" fill="red"/>
    </svg>"#;

    // Test Base64 encoding
    let config_base64 = Config {
        plugins: vec![PluginConfig::new("removeMetadata".to_string())],
        datauri: Some(DataUriFormat::Base64),
        ..Default::default()
    };

    let result_base64 = optimize(svg, OptimizeOptions::new(config_base64)).unwrap();
    assert!(result_base64.data.starts_with("data:image/svg+xml;base64,"));

    // Decode the base64 to verify it's valid
    let base64_part = result_base64
        .data
        .strip_prefix("data:image/svg+xml;base64,")
        .unwrap();
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let decoded = STANDARD.decode(base64_part).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    assert!(decoded_str.contains("<svg"));
    assert!(decoded_str.contains("<circle"));

    // Test URL encoding
    let config_enc = Config {
        plugins: vec![PluginConfig::new("removeMetadata".to_string())],
        datauri: Some(DataUriFormat::Enc),
        ..Default::default()
    };

    let result_enc = optimize(svg, OptimizeOptions::new(config_enc)).unwrap();
    assert!(result_enc.data.starts_with("data:image/svg+xml,"));
    assert!(result_enc.data.contains("%3Csvg"));
    assert!(result_enc.data.contains("%3Ccircle"));

    // Test unencoded
    let config_unenc = Config {
        plugins: vec![PluginConfig::new("removeMetadata".to_string())],
        datauri: Some(DataUriFormat::Unenc),
        ..Default::default()
    };

    let result_unenc = optimize(svg, OptimizeOptions::new(config_unenc)).unwrap();
    assert!(result_unenc.data.starts_with("data:image/svg+xml,"));
    assert!(result_unenc.data.contains("<svg"));
    assert!(result_unenc.data.contains("<circle"));
}
    };

    let result_unenc = optimize(svg, OptimizeOptions::new(config_unenc)).unwrap();
    assert!(result_unenc.data.starts_with("data:image/svg+xml,"));
    assert!(result_unenc.data.contains("<svg"));
    assert!(result_unenc.data.contains("<circle"));
}
