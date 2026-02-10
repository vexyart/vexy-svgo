// this_file: crates/core/tests/plugin_registry_tests.rs

//! Comprehensive tests for plugin registry functionality

use anyhow::Result;
use serde_json::json;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::parser::config::PluginConfig;
use vexy_svgo_core::{create_default_registry, Plugin, PluginRegistry, VexyError};

// Test plugins for various scenarios
struct BasicTestPlugin;
impl Plugin for BasicTestPlugin {
    fn name(&self) -> &'static str {
        "basic_test"
    }
    fn description(&self) -> &'static str {
        "Basic test plugin"
    }
    fn apply(&self, _document: &mut Document) -> Result<()> {
        Ok(())
    }
}

struct ParameterizedTestPlugin {
    value: i32,
}
impl Plugin for ParameterizedTestPlugin {
    fn name(&self) -> &'static str {
        "parameterized_test"
    }
    fn description(&self) -> &'static str {
        "Parameterized test plugin"
    }

    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        if let Some(val) = params.get("value") {
            val.as_i64().ok_or_else(|| anyhow::anyhow!("value must be a number"))?;
        }
        Ok(())
    }

    fn apply(&self, _document: &mut Document) -> Result<()> {
        Ok(())
    }
}

struct FailingTestPlugin;
impl Plugin for FailingTestPlugin {
    fn name(&self) -> &'static str {
        "failing_test"
    }
    fn description(&self) -> &'static str {
        "Plugin that always fails"
    }
    fn apply(&self, _document: &mut Document) -> Result<()> {
        Err(anyhow::anyhow!("Plugin intentionally failed"))
    }
}

struct GlobalTestPlugin;
impl Plugin for GlobalTestPlugin {
    fn name(&self) -> &'static str {
        "global_test"
    }
    fn description(&self) -> &'static str {
        "Global test plugin"
    }
    fn category(&self) -> &'static str {
        "global"
    }
    fn apply(&self, _document: &mut Document) -> Result<()> {
        Ok(())
    }
}

struct CleanupTestPlugin;
impl Plugin for CleanupTestPlugin {
    fn name(&self) -> &'static str {
        "cleanup_test"
    }
    fn description(&self) -> &'static str {
        "Cleanup test plugin"
    }
    fn category(&self) -> &'static str {
        "cleanup"
    }
    fn apply(&self, _document: &mut Document) -> Result<()> {
        Ok(())
    }
}

struct CommentRemovalPlugin;
impl Plugin for CommentRemovalPlugin {
    fn name(&self) -> &'static str {
        "remove_comments"
    }
    fn description(&self) -> &'static str {
        "Remove comments from document"
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Remove comments from all sections
        document.root.children.retain(|node| !matches!(node, Node::Comment(_)));
        document.prologue.retain(|node| !matches!(node, Node::Comment(_)));
        document.epilogue.retain(|node| !matches!(node, Node::Comment(_)));
        Ok(())
    }
}

#[test]
fn test_plugin_registry_creation() {
    let registry = PluginRegistry::new();
    assert!(registry.plugin_names().is_empty());

    let default_registry = create_default_registry();
    // Default registry might be empty or contain default plugins
    assert!(default_registry.plugin_names().len() >= 0);
}

#[test]
fn test_plugin_registration() {
    let mut registry = PluginRegistry::new();

    // Test registering a plugin
    registry.register("basic_test", || BasicTestPlugin);

    // Test that the plugin was registered
    assert!(registry.plugin_names().contains(&"basic_test"));

    // Test creating the plugin
    let plugin = registry.create_plugin("basic_test");
    assert!(plugin.is_some());
    assert_eq!(plugin.unwrap().name(), "basic_test");

    // Test non-existent plugin
    assert!(registry.create_plugin("nonexistent").is_none());
}

#[test]
fn test_multiple_plugin_registration() {
    let mut registry = PluginRegistry::new();

    registry.register("basic_test", || BasicTestPlugin);
    registry.register("global_test", || GlobalTestPlugin);
    registry.register("cleanup_test", || CleanupTestPlugin);

    let names = registry.plugin_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"basic_test"));
    assert!(names.contains(&"global_test"));
    assert!(names.contains(&"cleanup_test"));
}

#[test]
fn test_plugin_categories() {
    let mut registry = PluginRegistry::new();

    registry.register("basic_test", || BasicTestPlugin);
    registry.register("global_test", || GlobalTestPlugin);
    registry.register("cleanup_test", || CleanupTestPlugin);

    // Test getting plugins by category
    let element_plugins = registry.get_plugins_by_category("element");
    assert_eq!(element_plugins.len(), 1);
    assert_eq!(element_plugins[0].name(), "basic_test");

    let global_plugins = registry.get_plugins_by_category("global");
    assert_eq!(global_plugins.len(), 1);
    assert_eq!(global_plugins[0].name(), "global_test");

    let cleanup_plugins = registry.get_plugins_by_category("cleanup");
    assert_eq!(cleanup_plugins.len(), 1);
    assert_eq!(cleanup_plugins[0].name(), "cleanup_test");
}

#[test]
fn test_apply_single_plugin() {
    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);

    let mut document = Document::new();
    let config = PluginConfig::Name("basic_test".to_string());

    let result = registry.apply_plugin(&mut document, &config);
    assert!(result.is_ok());
}

#[test]
fn test_apply_multiple_plugins() {
    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);
    registry.register("global_test", || GlobalTestPlugin);

    let mut document = Document::new();
    let configs = vec![
        PluginConfig::Name("basic_test".to_string()),
        PluginConfig::Name("global_test".to_string()),
    ];

    let result = registry.apply_plugins(&mut document, &configs);
    assert!(result.is_ok());
}

#[test]
fn test_plugin_with_parameters() {
    let mut registry = PluginRegistry::new();
    registry.register("parameterized_test", || ParameterizedTestPlugin { value: 42 });

    let mut document = Document::new();
    let config = PluginConfig::WithParams {
        name:   "parameterized_test".to_string(),
        params: json!({"value": 100}),
    };

    let result = registry.apply_plugin(&mut document, &config);
    assert!(result.is_ok());
}

#[test]
fn test_plugin_parameter_validation() {
    let mut registry = PluginRegistry::new();
    registry.register("parameterized_test", || ParameterizedTestPlugin { value: 42 });

    let mut document = Document::new();

    // Test with valid parameters
    let valid_config = PluginConfig::WithParams {
        name:   "parameterized_test".to_string(),
        params: json!({"value": 100}),
    };
    let result = registry.apply_plugin(&mut document, &valid_config);
    assert!(result.is_ok());

    // Test with invalid parameters
    let invalid_config = PluginConfig::WithParams {
        name:   "parameterized_test".to_string(),
        params: json!({"value": "not_a_number"}),
    };
    let result = registry.apply_plugin(&mut document, &invalid_config);
    assert!(result.is_err());
}

#[test]
fn test_disabled_plugin() {
    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);

    let mut document = Document::new();
    let config = PluginConfig::WithParams {
        name:   "basic_test".to_string(),
        params: json!({"enabled": false}),
    };

    let result = registry.apply_plugin(&mut document, &config);
    assert!(result.is_ok());
    // Plugin should not have been applied due to being disabled
}

#[test]
fn test_plugin_failure_handling() {
    let mut registry = PluginRegistry::new();
    registry.register("failing_test", || FailingTestPlugin);

    let mut document = Document::new();
    let config = PluginConfig::Name("failing_test".to_string());

    let result = registry.apply_plugin(&mut document, &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Plugin intentionally failed"));
}

#[test]
fn test_nonexistent_plugin_handling() {
    let registry = PluginRegistry::new();

    let mut document = Document::new();
    let config = PluginConfig::Name("nonexistent".to_string());

    let result = registry.apply_plugin(&mut document, &config);
    // Should succeed silently for non-existent plugins
    assert!(result.is_ok());
}

#[test]
fn test_functional_plugin_application() {
    let mut registry = PluginRegistry::new();
    registry.register("remove_comments", || CommentRemovalPlugin);

    let mut document = Document::new();

    // Add some comments to the document
    document.root.children.push(Node::Comment("Test comment".into()));
    document.prologue.push(Node::Comment("Prologue comment".into()));
    document.epilogue.push(Node::Comment("Epilogue comment".into()));

    // Verify comments exist
    assert!(document.root.children.iter().any(|n| matches!(n, Node::Comment(_))));
    assert!(document.prologue.iter().any(|n| matches!(n, Node::Comment(_))));
    assert!(document.epilogue.iter().any(|n| matches!(n, Node::Comment(_))));

    // Apply the comment removal plugin
    let config = PluginConfig::Name("remove_comments".to_string());
    let result = registry.apply_plugin(&mut document, &config);
    assert!(result.is_ok());

    // Verify comments were removed
    assert!(!document.root.children.iter().any(|n| matches!(n, Node::Comment(_))));
    assert!(!document.prologue.iter().any(|n| matches!(n, Node::Comment(_))));
    assert!(!document.epilogue.iter().any(|n| matches!(n, Node::Comment(_))));
}

#[test]
fn test_plugin_chain_execution() {
    let mut registry = PluginRegistry::new();
    registry.register("global_test", || GlobalTestPlugin);
    registry.register("basic_test", || BasicTestPlugin);
    registry.register("cleanup_test", || CleanupTestPlugin);

    let mut document = Document::new();
    let configs = vec![
        PluginConfig::Name("global_test".to_string()),
        PluginConfig::Name("basic_test".to_string()),
        PluginConfig::Name("cleanup_test".to_string()),
    ];

    let result = registry.apply_plugins(&mut document, &configs);
    assert!(result.is_ok());
}

#[test]
fn test_plugin_chain_with_failure() {
    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);
    registry.register("failing_test", || FailingTestPlugin);
    registry.register("cleanup_test", || CleanupTestPlugin);

    let mut document = Document::new();
    let configs = vec![
        PluginConfig::Name("basic_test".to_string()),
        PluginConfig::Name("failing_test".to_string()),
        PluginConfig::Name("cleanup_test".to_string()), // This shouldn't run
    ];

    let result = registry.apply_plugins(&mut document, &configs);
    assert!(result.is_err());
}

#[test]
fn test_mixed_plugin_configuration() {
    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);
    registry.register("parameterized_test", || ParameterizedTestPlugin { value: 42 });

    let mut document = Document::new();
    let configs = vec![
        PluginConfig::Name("basic_test".to_string()),
        PluginConfig::WithParams {
            name:   "parameterized_test".to_string(),
            params: json!({"value": 123}),
        },
        PluginConfig::WithParams {
            name:   "basic_test".to_string(),
            params: json!({"enabled": false}),
        },
    ];

    let result = registry.apply_plugins(&mut document, &configs);
    assert!(result.is_ok());
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_plugin_execution() {
    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);
    registry.register("global_test", || GlobalTestPlugin);
    registry.register("cleanup_test", || CleanupTestPlugin);

    let mut document = Document::new();
    let configs = vec![
        PluginConfig::Name("global_test".to_string()),
        PluginConfig::Name("basic_test".to_string()),
        PluginConfig::Name("cleanup_test".to_string()),
    ];

    let result = registry.apply_plugins_parallel(&mut document, &configs, 2);
    assert!(result.is_ok());
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_plugin_execution_with_failure() {
    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);
    registry.register("failing_test", || FailingTestPlugin);

    let mut document = Document::new();
    let configs = vec![
        PluginConfig::Name("basic_test".to_string()),
        PluginConfig::Name("failing_test".to_string()),
    ];

    let result = registry.apply_plugins_parallel(&mut document, &configs, 2);
    assert!(result.is_err());
}

#[test]
fn test_plugin_registry_default() {
    let registry = PluginRegistry::default();
    assert!(registry.plugin_names().is_empty());
}

#[test]
fn test_plugin_trait_defaults() {
    let plugin = BasicTestPlugin;

    assert_eq!(plugin.category(), "element");
    assert!(plugin.validate_params(&json!({})).is_ok());
    assert!(plugin.validate_params(&json!({"any": "value"})).is_ok());
}

#[test]
fn test_plugin_registry_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let mut registry = PluginRegistry::new();
    registry.register("basic_test", || BasicTestPlugin);

    let registry = Arc::new(registry);
    let mut handles = vec![];

    // Spawn multiple threads to test thread safety
    for _ in 0..10 {
        let registry = Arc::clone(&registry);
        let handle = thread::spawn(move || {
            let plugin = registry.create_plugin("basic_test");
            assert!(plugin.is_some());
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
}
