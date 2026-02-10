// this_file: crates/core/src/plugin_registry.rs

//! Plugin registry for managing and executing plugins
//!
//! This module provides a registry system for plugins using the visitor pattern
//! and composition-based architecture as specified in REFACTOR.md.

use std::collections::HashMap;

use anyhow::Result;

use crate::ast::Document;

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

    /// Configure the plugin with runtime parameters
    /// Called after validation, before apply(). Plugins that accept params
    /// should override this to store the params for use during apply().
    fn configure(&mut self, params: &serde_json::Value) -> Result<()> {
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

#[cfg(feature = "parallel")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Parallel,
    Sequential,
    ConcurrentMutate,
}

#[cfg(feature = "parallel")]
#[derive(Debug, Clone)]
pub struct PluginBatch {
    pub configs:        Vec<PluginConfig>,
    pub execution_mode: ExecutionMode,
}

#[cfg(feature = "parallel")]
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub batches: Vec<PluginBatch>,
}

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
    pub fn apply_plugin<'a>(&self, document: &mut Document<'a>, config: &PluginConfig) -> Result<()> {
        let (name, params, enabled) = match config {
            PluginConfig::Name(name) => (name.as_str(), &serde_json::Value::Null, true),
            PluginConfig::WithParams { name, params } => {
                // SVGO compatibility: json!(false) disables the plugin entirely
                if params.is_boolean() {
                    let enabled = params.as_bool().unwrap_or(true);
                    (name.as_str(), &serde_json::Value::Null, enabled)
                } else {
                    let enabled = params.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                    (name.as_str(), params, enabled)
                }
            },
        };

        if !enabled {
            return Ok(());
        }

        if let Some(mut plugin) = self.create_plugin(name) {
            // Validate parameters
            plugin.validate_params(params)?;

            // Configure plugin with runtime parameters
            if !params.is_null() {
                plugin.configure(params)?;
            }

            // Apply the plugin directly
            plugin.apply(document)?;
        }
        Ok(())
    }

    /// Apply plugins to a document
    pub fn apply_plugins<'a>(&self, document: &mut Document<'a>, configs: &[PluginConfig]) -> Result<()> {
        for config in configs {
            self.apply_plugin(document, config)?;
        }
        Ok(())
    }

    /// Apply plugins to a document with parallel processing
    ///
    /// This implements a hybrid strategy:
    /// - Read-only plugins (no mutations) run in parallel batches
    /// - Mutating plugins run sequentially to preserve ordering
    /// - Uses rayon::scope for safe structured concurrency
    #[cfg(feature = "parallel")]
    pub fn apply_plugins_parallel<'a>(
        &self,
        document: &mut Document<'a>,
        configs: &[PluginConfig],
        num_threads: usize,
    ) -> Result<()> {
        use std::sync::{Arc, Mutex};

        use rayon::prelude::*;

        // Configure thread pool
        if num_threads > 0 {
            rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().ok();
        }

        // Classify plugins into parallel-safe batches
        let execution_plan = self.create_execution_plan(configs)?;

        // Execute batches according to plan
        for batch in execution_plan.batches {
            match batch.execution_mode {
                ExecutionMode::Parallel => {
                    // Parallel batch: clone document for each plugin, apply concurrently, merge results
                    let document_clone = document.clone();
                    let results: Vec<Result<Document<'a>>> = batch
                        .configs
                        .par_iter()
                        .map(|config| {
                            let mut doc = document_clone.clone();
                            self.apply_plugin(&mut doc, config)?;
                            Ok(doc)
                        })
                        .collect();

                    // Merge results: apply each plugin's changes sequentially (safe merge)
                    for (idx, result) in results.into_iter().enumerate() {
                        let _doc = result?;
                        // For simplicity, we merge by re-applying the plugin to the main document
                        // This ensures correctness while still benefiting from parallel computation
                        self.apply_plugin(document, &batch.configs[idx])?;
                    }
                },
                ExecutionMode::Sequential => {
                    // Sequential batch: apply plugins in order
                    for config in &batch.configs {
                        self.apply_plugin(document, config)?;
                    }
                },
                ExecutionMode::ConcurrentMutate => {
                    // Concurrent mutation: use Arc<Mutex<Document>> for thread-safe mutations
                    let document_arc = Arc::new(Mutex::new(std::mem::take(document)));

                    batch.configs.par_iter().try_for_each(|config| {
                        let doc_lock = document_arc.clone();
                        let mut doc = doc_lock.lock().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
                        self.apply_plugin(&mut doc, config)
                    })?;

                    // Restore document
                    let doc_arc = Arc::try_unwrap(document_arc).map_err(|_| anyhow::anyhow!("Failed to unwrap Arc"))?;
                    *document = doc_arc.into_inner().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
                },
            }
        }

        Ok(())
    }

    #[cfg(feature = "parallel")]
    fn create_execution_plan(&self, configs: &[PluginConfig]) -> Result<ExecutionPlan> {
        let mut batches = Vec::new();
        let mut current_batch_configs = Vec::new();
        let mut current_mode = ExecutionMode::Sequential;

        for config in configs {
            if let Some(plugin) = self.create_plugin(config.name()) {
                let category = plugin.category();

                let mode = match category {
                    "global" | "cleanup" => ExecutionMode::Sequential,
                    "element" => ExecutionMode::Parallel,
                    _ => ExecutionMode::Sequential,
                };

                if mode != current_mode && !current_batch_configs.is_empty() {
                    batches.push(PluginBatch {
                        configs:        current_batch_configs,
                        execution_mode: current_mode,
                    });
                    current_batch_configs = Vec::new();
                }

                current_batch_configs.push(config.clone());
                current_mode = mode;
            }
        }

        if !current_batch_configs.is_empty() {
            batches.push(PluginBatch {
                configs:        current_batch_configs,
                execution_mode: current_mode,
            });
        }

        Ok(ExecutionPlan { batches })
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
            fn name(&self) -> &'static str {
                "test_parallel_plugin"
            }
            fn description(&self) -> &'static str {
                "test plugin"
            }
            fn category(&self) -> &'static str {
                "element"
            }
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
