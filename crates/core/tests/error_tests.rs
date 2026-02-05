// this_file: crates/core/tests/error_tests.rs

//! Comprehensive tests for error handling in vexy_svgo_core

use vexy_svgo_core::error::VexyError;
use vexy_svgo_core::parser::error::ParseError;
use std::io;

#[test]
fn test_vexy_error_from_io_error() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let vexy_error = VexyError::from(io_error);
    
    match vexy_error {
        VexyError::Io(err) => {
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
            assert_eq!(err.to_string(), "File not found");
        }
        _ => panic!("Expected VexyError::Io variant"),
    }
}

#[test]
fn test_vexy_error_from_parse_error() {
    let parse_error = ParseError::StructureError("invalid element".to_string());
    let vexy_error = VexyError::from(parse_error);
    
    match vexy_error {
        VexyError::Parse(ParseError::StructureError(ref msg)) => {
            assert_eq!(msg, "invalid element");
        }
        _ => panic!("Expected VexyError::Parse variant"),
    }
}

#[test]
fn test_vexy_error_plugin_variant() {
    let error = VexyError::Plugin("test_plugin".to_string(), "Plugin failed".to_string());
    
    match error {
        VexyError::Plugin(name, msg) => {
            assert_eq!(name, "test_plugin");
            assert_eq!(msg, "Plugin failed");
        }
        _ => panic!("Expected VexyError::Plugin variant"),
    }
    
    // Test display
    let error = VexyError::Plugin("test_plugin".to_string(), "Plugin failed".to_string());
    assert_eq!(error.to_string(), "Plugin 'test_plugin' failed: Plugin failed");
}

#[test]
fn test_vexy_error_config_variant() {
    let error = VexyError::Config("Invalid configuration value".to_string());
    
    match error {
        VexyError::Config(ref msg) => {
            assert_eq!(msg, "Invalid configuration value");
        }
        _ => panic!("Expected VexyError::Config variant"),
    }
    
    // Test display
    assert_eq!(error.to_string(), "Invalid configuration: Invalid configuration value");
}

#[test]
fn test_vexy_error_regex_variant() {
    let error = VexyError::Regex("Invalid regex pattern".to_string());
    
    match error {
        VexyError::Regex(ref msg) => {
            assert_eq!(msg, "Invalid regex pattern");
        }
        _ => panic!("Expected VexyError::Regex variant"),
    }
    
    // Test display
    assert_eq!(error.to_string(), "Regex error: Invalid regex pattern");
}

#[test]
fn test_vexy_error_general_variant() {
    let error = VexyError::General("General error message".to_string());
    
    match error {
        VexyError::General(ref msg) => {
            assert_eq!(msg, "General error message");
        }
        _ => panic!("Expected VexyError::General variant"),
    }
    
    // Test display
    assert_eq!(error.to_string(), "General error message");
}

#[test]
fn test_vexy_error_from_string() {
    let error_msg = "Error from string".to_string();
    let error = VexyError::from(error_msg);
    
    match error {
        VexyError::General(msg) => {
            assert_eq!(msg, "Error from string");
        }
        _ => panic!("Expected VexyError::General variant"),
    }
}

#[test]
fn test_vexy_error_from_str() {
    let error_msg = "Error from str";
    let error = VexyError::from(error_msg);
    
    match error {
        VexyError::General(msg) => {
            assert_eq!(msg, "Error from str");
        }
        _ => panic!("Expected VexyError::General variant"),
    }
}

#[test]
fn test_vexy_error_from_anyhow_error() {
    let anyhow_error = anyhow::anyhow!("Anyhow error message");
    let vexy_error = VexyError::from(anyhow_error);
    
    match vexy_error {
        VexyError::General(msg) => {
            assert_eq!(msg, "Anyhow error message");
        }
        _ => panic!("Expected VexyError::General variant"),
    }
}

#[test]
fn test_vexy_error_from_regex_error() {
    // Create an invalid regex to generate a regex error
    let regex_result = regex::Regex::new("[");
    let regex_error = regex_result.unwrap_err();
    let vexy_error = VexyError::from(regex_error);
    
    match vexy_error {
        VexyError::Regex(msg) => {
            assert!(msg.contains("unclosed character class"));
        }
        _ => panic!("Expected VexyError::Regex variant"),
    }
}

#[test]
fn test_vexy_error_from_fmt_error() {
    let fmt_error = std::fmt::Error;
    let vexy_error = VexyError::from(fmt_error);
    
    match vexy_error {
        VexyError::General(msg) => {
            assert_eq!(msg, "an error occurred when formatting an argument");
        }
        _ => panic!("Expected VexyError::General variant"),
    }
}

#[test]
fn test_error_propagation_through_optimize() {
    use vexy_svgo_core::optimize_default;
    
    // Test with malformed SVG - parser may recover or error depending on severity
    let malformed_svg = "<svg><rect></rect"; // Missing closing >
    let result = optimize_default(malformed_svg);
    
    // Parser has error recovery, so minor malformed input may still produce output
    // Check that we either get a parse error OR recovered output
    match result {
        Ok(output) => {
            // Parser recovered - output should contain valid SVG root
            assert!(output.data.contains("<svg"), "Recovered output should contain svg tag");
        }
        Err(VexyError::Parse(_)) => {
            // Expected parse error for unrecoverable malformed input
        }
        Err(other) => panic!("Unexpected error type: {:?}", other),
    }
}

#[test]
fn test_error_propagation_through_parser() {
    use vexy_svgo_core::parse_svg;
    
    // Test with completely invalid XML
    let invalid_xml = "<svg><rect><invalid></svg>";
    let result = parse_svg(invalid_xml);
    
    // Should return an error
    assert!(result.is_err());
    
    // The error should be a parse error
    match result.unwrap_err() {
        VexyError::Parse(_) => {
            // Expected parse error
        }
        other => panic!("Expected parse error, got: {:?}", other),
    }
}

#[test]
fn test_error_propagation_through_stringifier() {
    use vexy_svgo_core::{parse_svg, stringify};
    
    // Create a valid document first
    let valid_svg = "<svg><rect width=\"100\" height=\"100\"/></svg>";
    let document = parse_svg(valid_svg).unwrap();
    
    // Stringifier should work normally
    let result = stringify(&document);
    assert!(result.is_ok());
    
    // Note: Stringifier errors are rare in normal operation
    // Most errors would come from invalid AST states, but those
    // are prevented by the parser and type system
}

#[test]
fn test_error_debug_formatting() {
    let error = VexyError::Plugin("test_plugin".to_string(), "Error details".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("Plugin"));
    assert!(debug_str.contains("test_plugin"));
    assert!(debug_str.contains("Error details"));
}

#[test]
fn test_error_display_formatting() {
    let error = VexyError::Config("Invalid option".to_string());
    let display_str = format!("{}", error);
    assert_eq!(display_str, "Invalid configuration: Invalid option");
}

#[test]
fn test_multiple_error_conversions() {
    // Test chaining multiple error conversions
    let original_error = io::Error::new(io::ErrorKind::PermissionDenied, "Access denied");
    let vexy_error = VexyError::from(original_error);
    
    // Convert to string and back
    let error_string = vexy_error.to_string();
    let new_error = VexyError::from(error_string);
    
    match new_error {
        VexyError::General(msg) => {
            assert!(msg.contains("Access denied"));
        }
        _ => panic!("Expected VexyError::General variant"),
    }
}