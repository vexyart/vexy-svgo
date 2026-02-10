// this_file: crates/core/tests/memory_tests.rs

use vexy_svgo_core::parser::config::StreamingConfig;
use vexy_svgo_core::parser::Parser;

#[test]
fn test_memory_limit_enforcement() {
    let svg = r#"<svg><rect x="10" y="10" width="50" height="50"/></svg>"#;
    
    // Set a very low memory limit that should be exceeded
    // 50 bytes is not enough for the Element structure + attributes
    let config = StreamingConfig {
        memory_limit: Some(50), 
        ..StreamingConfig::default()
    };
    
    let parser = Parser::new().streaming_config(config);
    let result = parser.parse(svg);
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Memory limit exceeded"));
}

#[test]
fn test_memory_limit_sufficient() {
    let svg = r#"<svg><rect x="10" y="10" width="50" height="50"/></svg>"#;
    
    // 1MB should be plenty
    let config = StreamingConfig {
        memory_limit: Some(1024 * 1024), 
        ..StreamingConfig::default()
    };
    
    let parser = Parser::new().streaming_config(config);
    let result = parser.parse(svg);
    
    assert!(result.is_ok());
}
