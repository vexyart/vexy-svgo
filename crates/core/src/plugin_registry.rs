// this_file: crates/core/src/plugin_registry.rs

//! Plugin registry for managing and executing plugins
//!
//! This module provides a registry system for plugins using the visitor pattern
//! and composition-based architecture as specified in REFACTOR.md.

use crate::ast::Document;
use anyhow::Result;
use std::collections::HashMap;

/// Plugin trait using composition over inheritance
pub trait Plugin: Send + Sync {
    /// Plugin name (must be unique)
    fn name(&self) -> &'static str;

    /// Plugin description
    fn description(&self) -> &'static str;
    
    /// Plugin category for parallel processing
    /// - `global`: Needs to see the whole document
    /// - `element`: Can process elements independently  
    /// - `cleanup`: Runs after other plugins
    fn category(&self) -> &'static str {
        "element" // Default to element-level
    }

    /// Validate plugin parameters
    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        let _ = params;
        Ok(())
    }
    
    /// Check if the plugin is enabled for the given config
    fn is_enabled(&self, config: &crate::parser::config::Config) -> bool {
        config.plugins.iter().any(|p| match p {
            crate::parser::config::PluginConfig::Name(name) => name == self.name(),
            crate::parser::config::PluginConfig::WithParams { name, .. } => name == self.name(),
        })
    }
    
    /// Process the document (for compatibility with parallel optimizer)
    fn process(&self, document: &mut Document, _config: &crate::parser::config::Config) {
        // Default implementation calls apply
        let _ = self.apply(document);
    }

    /// Apply the plugin to the document
    /// This method directly modifies the document or returns a visitor
    fn apply<'a>(&self, document: &mut Document<'a>) -> Result<()>;
}

// Use PluginConfig from parser::config module
use crate::parser::config::PluginConfig;

/// Plugin factory function type
pub type PluginFactory = Box<dyn Fn() -> Box<dyn Plugin> + Send + Sync>;

/// Plugin registry managing the plugin system
pub struct PluginRegistry {
    plugin_factories: HashMap<String, PluginFactory>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            plugin_factories: HashMap::new(),
        }
    }

    /// Register a plugin with a factory function
    pub fn register<P: Plugin + 'static>(&mut self, name: &str, factory: impl Fn() -> P + 'static + Send + Sync) {
        let factory_fn = Box::new(move || Box::new(factory()) as Box<dyn Plugin>);
        self.plugin_factories.insert(name.to_string(), factory_fn);
    }

    /// Create a plugin instance by name
    pub fn create_plugin(&self, name: &str) -> Option<Box<dyn Plugin>> {
        self.plugin_factories.get(name).map(|factory| factory())
    }

    /// Apply a single plugin to a document
    pub fn apply_plugin<'a>(
        &self,
        document: &mut Document<'a>,
        config: &PluginConfig,
    ) -> Result<()> {
        let (name, params, enabled) = match config {
            PluginConfig::Name(name) => (name.as_str(), &serde_json::Value::Null, true),
            PluginConfig::WithParams { name, params } => {
                let enabled = params.get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                (name.as_str(), params, enabled)
            }
        };

        if !enabled {
            return Ok(());
        }

        if let Some(plugin) = self.create_plugin(name) {
            // Validate parameters
            plugin.validate_params(params)?;

            // Apply the plugin directly
            plugin.apply(document)?;
        }
        Ok(())
    }

    /// Apply plugins to a document
    pub fn apply_plugins<'a>(
        &self,
        document: &mut Document<'a>,
        configs: &[PluginConfig],
    ) -> Result<()> {
        for config in configs {
            self.apply_plugin(document, config)?;
        }
        Ok(())
    }
    
    /// Apply plugins to a document with parallel processing
    #[cfg(feature = "parallel")]
    pub fn apply_plugins_parallel<'a>(
        &self,
        document: &mut Document<'a>,
        configs: &[PluginConfig],
        num_threads: usize,
    ) -> Result<()> {
        use std::collections::HashMap;
        
        // Configure thread pool
        if num_threads > 0 {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .map_err(|e| anyhow::anyhow!("Failed to build thread pool: {}", e))?;
        }
        
        // Group plugins by category for parallel execution
        let mut plugin_groups: HashMap<&str, Vec<&PluginConfig>> = HashMap::new();
        for config in configs {
            if let Some(plugin) = self.create_plugin(config.name()) {
                let category = plugin.category();
                plugin_groups.entry(category).or_default().push(config);
            }
        }
        
        // Execute plugins in order: global -> element -> cleanup
        for category in &["global", "element", "cleanup"] {
            if let Some(plugins) = plugin_groups.get(category) {
                match *category {
                    "global" | "cleanup" => {
                        // Execute global and cleanup plugins sequentially
                        for config in plugins {
                            self.apply_plugin(document, config)?;
                        }
                    }
                    "element" => {
                        // Execute element-level plugins in parallel
                        self.apply_element_plugins_parallel(document, plugins)?;
                    }
                    _ => {
                        // Unknown category, execute sequentially
                        for config in plugins {
                            self.apply_plugin(document, config)?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply element-level plugins in parallel
    #[cfg(feature = "parallel")]
    fn apply_element_plugins_parallel<'a>(
        &self,
        document: &mut Document<'a>,
        configs: &[&PluginConfig],
    ) -> Result<()> {
        use std::sync::Arc;
        
        // Create a thread-safe reference to the registry
        let _registry = Arc::new(self);
        
        // For element-level plugins, we can process independent elements in parallel
        if let Some(_root_element) = document.root.children.iter_mut().find_map(|child| {
            if let crate::ast::Node::Element(elem) = child {
                Some(elem)
            } else {
                None
            }
        }) {
            // TODO: Revisit parallel processing for element-level plugins.
            // The current approach of cloning the entire document for each element
            // is inefficient and does not modify the original document in place.
            // For now, we will fall back to sequential processing for element-level plugins.
            for config in configs {
                self.apply_plugin(document, config)?;
            }
        }
        
        Ok(())
    }

    /// Get list of registered plugin names
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugin_factories.keys().map(|k| k.as_str()).collect()
    }
    
    /// Get plugins by category
    pub fn get_plugins_by_category(&self, category: &str) -> Vec<Box<dyn Plugin>> {
        self.plugin_factories
            .iter()
            .filter_map(|(_name, factory)| {
                let plugin = factory();
                if plugin.category() == category {
                    Some(plugin)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default plugin registry with standard plugins
pub fn create_default_registry() -> PluginRegistry {
    // We need a way to create a registry with default plugins
    // This is temporarily empty but should be populated by the application layer
    // that has access to both core and plugin-sdk
    PluginRegistry::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Document;

    struct TestPlugin;

    impl Plugin for TestPlugin {
        fn name(&self) -> &'static str {
            "test"
        }

        fn description(&self) -> &'static str {
            "Test plugin"
        }

        fn apply<'a>(&self, _document: &mut Document<'a>) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register("test", || TestPlugin);

        assert!(registry.create_plugin("test").is_some());
        assert!(registry.create_plugin("nonexistent").is_none());

        let names = registry.plugin_names();
        assert!(names.contains(&"test"));
    }
    
    #[test]
    #[cfg(feature = "parallel")]
    fn test_parallel_plugin_execution() {
        use crate::ast::*;
        
        let mut registry = PluginRegistry::new();
        
        // Create a test plugin
        struct TestPlugin;
        impl Plugin for TestPlugin {
            fn name(&self) -> &'static str { "test_parallel_plugin" }
            fn description(&self) -> &'static str { "test plugin" }
            fn category(&self) -> &'static str { "element" }
            fn apply(&self, _document: &mut Document) -> anyhow::Result<()> {
                // Test plugin that does nothing
                Ok(())
            }
        }
        
        registry.register("test_parallel_plugin", || TestPlugin);
        
        let mut doc = Document::new();
        let configs = vec![PluginConfig::Name("test_parallel_plugin".to_string())];
        
        // Test parallel execution
        let result = registry.apply_plugins_parallel(&mut doc, &configs, 2);
        assert!(result.is_ok());
    }
}
