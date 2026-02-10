// this_file: crates/plugin-sdk/src/lib.rs

//! Plugin system for SVG optimization
//!
//! This module defines the plugin trait and infrastructure for applying
//! optimization transformations to SVG documents.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::Document;
use vexy_svgo_core::error::VexyError;

#[macro_use]

// Visitor is no longer needed with the simplified core Plugin trait

pub mod plugins;
pub mod registry;
pub mod enhanced_registry;
pub mod css_matching;

#[macro_use]
#[doc(hidden)]
pub mod plugin_test_macros;

#[cfg(test)]
#[doc(hidden)]
pub mod property_tests;

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, VexyError>;



// PluginInfo is no longer needed with the simplified core Plugin trait

// Re-export the core Plugin trait
pub use vexy_svgo_core::Plugin;

/// Trait for plugins that can be created with parameters
pub trait PluginWithParams: Plugin + Sized {
    /// The configuration type for this plugin
    type Config;
    
    /// Create a new instance with the given configuration
    fn with_config(config: Self::Config) -> Self;
    
    /// Parse configuration from JSON value
    fn parse_config(params: &serde_json::Value) -> anyhow::Result<Self::Config>;
}

/// Plugin descriptor for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDescriptor {
    /// Plugin name
    pub name: String,
    /// Plugin parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Whether the plugin is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl PluginDescriptor {
    /// Create a new plugin configuration
    pub fn new(name: String) -> Self {
        Self {
            name,
            params: None,
            enabled: true,
        }
    }

    /// Create a new plugin configuration with parameters
    pub fn with_params(name: String, params: Value) -> Self {
        Self {
            name,
            params: Some(params),
            enabled: true,
        }
    }

    /// Disable this plugin
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Factory function type for creating plugins
type PluginFactory = Box<dyn Fn() -> Box<dyn Plugin> + Send + Sync>;

/// Plugin registry for managing available plugins
pub struct PluginRegistry {
    // Stores a map of plugin names to functions that can create a Box<dyn Plugin>
    plugin_factories: std::collections::HashMap<&'static str, PluginFactory>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry
    pub fn new() -> Self {
        Self {
            plugin_factories: std::collections::HashMap::new(),
        }
    }

    /// Register a plugin factory function
    pub fn register<F>(&mut self, name: &'static str, factory: F)
    where
        F: Fn() -> Box<dyn Plugin> + Send + Sync + 'static,
    {
        self.plugin_factories.insert(name, Box::new(factory));
    }

    /// Get a plugin instance by name
    pub fn get(&self, name: &str) -> Option<Box<dyn Plugin>> {
        self.plugin_factories.get(name).map(|factory| factory())
    }

    /// Get all registered plugin names
    pub fn plugin_names(&self) -> Vec<&'static str> {
        self.plugin_factories.keys().cloned().collect()
    }

    /// Apply a list of plugin configurations to a document
    pub fn apply_plugins(
        &self,
        document: &mut Document,
        configs: &[PluginDescriptor],
    ) -> Result<(), VexyError> {
        for config in configs {
            if !config.enabled {
                continue;
            }

            let plugin = self.get(&config.name).ok_or_else(|| {
                VexyError::Config(format!("Unknown plugin: {}", config.name))
            })?;

            // Validate parameters if provided
            if let Some(params) = config.params.as_ref() {
                plugin.validate_params(params)?;
            }

            // Apply the plugin directly to the document
            plugin.apply(document)?;
        }

        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the default plugin registry with all built-in plugins
pub fn create_default_registry() -> PluginRegistry {
    // TODO: Register built-in plugins using their factories
    // This will require a dependency on the vexy_svgo crate, which contains the plugin implementations.
    // For now, this function will return an empty registry.
    PluginRegistry::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test plugin for unit tests
    struct TestPlugin {
        name: &'static str,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            "Test plugin"
        }

        fn apply(&self, _document: &mut vexy_svgo_core::ast::Document) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register("test", || Box::new(TestPlugin { name: "test" }));

        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
        assert_eq!(registry.plugin_names(), vec!["test"]);
    }

    #[test]
    fn test_plugin_config() {
        let config = PluginDescriptor::new("test".to_string());
        assert_eq!(config.name, "test");
        assert!(config.enabled);
        assert!(config.params.is_none());

        let config_with_params =
            PluginDescriptor::with_params("test".to_string(), json!({"option": "value"}));
        assert!(config_with_params.params.is_some());

        let disabled_config = PluginDescriptor::new("test".to_string()).disabled();
        assert!(!disabled_config.enabled);
    }

    #[test]
    fn test_apply_plugins() {
        let mut registry = PluginRegistry::new();
        registry.register("test", || Box::new(TestPlugin { name: "test" }));

        let mut document = vexy_svgo_core::ast::Document::new();
        let configs = vec![PluginDescriptor::new("test".to_string())];

        // PluginInfo is no longer needed
        let result = registry.apply_plugins(&mut document, &configs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_unknown_plugin() {
        let registry = PluginRegistry::new();
        let mut document = vexy_svgo_core::ast::Document::new();
        let configs = vec![PluginDescriptor::new("unknown".to_string())];

        // PluginInfo is no longer needed
        let result = registry.apply_plugins(&mut document, &configs);
        assert!(result.is_err());
    }
}
