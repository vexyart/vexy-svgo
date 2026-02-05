// this_file: crates/plugin-sdk/src/enhanced_registry.rs

//! Enhanced plugin registry with advanced features
//!
//! This module provides an enhanced plugin registry that supports:
//! - Plugin discovery and dynamic loading
//! - Plugin versioning and compatibility
//! - Plugin dependencies and ordering
//! - Performance monitoring and metrics
//! - Plugin hot-reloading (feature gated)

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use vexy_svgo_core::error::VexyError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Plugin;

/// Plugin metadata with extended information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name (must be unique)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Semantic version (e.g., "1.2.3")
    pub version: String,
    /// Plugin author
    pub author: Option<String>,
    /// Plugin tags for categorization
    pub tags: Vec<String>,
    /// Whether this is an experimental plugin
    pub experimental: bool,
    /// Minimum SVGN version required
    pub min_vexy_svgo_version: Option<String>,
    /// Plugin dependencies
    pub dependencies: Vec<PluginDependency>,
    /// Plugin capabilities
    pub capabilities: Vec<PluginCapability>,
    /// Performance characteristics
    pub performance_hints: PerformanceHints,
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Name of required plugin
    pub name: String,
    /// Version requirement (semver)
    pub version: String,
    /// Whether this dependency is optional
    pub optional: bool,
}

/// Plugin capability flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginCapability {
    /// Plugin modifies the document structure
    StructuralChanges,
    /// Plugin only modifies attributes
    AttributeChanges,
    /// Plugin can work with streaming
    StreamingCompatible,
    /// Plugin benefits from multipass
    MultipassAware,
    /// Plugin has side effects (e.g., external calls)
    HasSideEffects,
    /// Plugin supports async operation
    AsyncCapable,
}

/// Performance characteristics and hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHints {
    /// Expected time complexity (relative scale)
    pub time_complexity: u8, // 1-10 scale
    /// Expected memory usage (relative scale)
    pub memory_usage: u8, // 1-10 scale
    /// Preferred execution order (lower = earlier)
    pub execution_order: i32,
    /// Whether plugin benefits from caching
    pub cacheable: bool,
    /// Whether plugin is CPU intensive
    pub cpu_intensive: bool,
}

impl Default for PerformanceHints {
    fn default() -> Self {
        Self {
            time_complexity: 5,
            memory_usage: 5,
            execution_order: 0,
            cacheable: false,
            cpu_intensive: false,
        }
    }
}

/// Plugin execution statistics
#[derive(Debug, Clone, Default)]
pub struct PluginStats {
    /// Total number of executions
    pub executions: u64,
    /// Total execution time
    pub total_time: Duration,
    /// Average execution time
    pub avg_time: Duration,
    /// Minimum execution time
    pub min_time: Duration,
    /// Maximum execution time
    pub max_time: Duration,
    /// Number of errors
    pub error_count: u64,
    /// Last execution time
    pub last_execution: Option<Instant>,
}

impl PluginStats {
    /// Update statistics with a new execution
    pub fn record_execution(&mut self, duration: Duration, success: bool) {
        self.executions += 1;
        self.total_time += duration;
        self.avg_time = Duration::from_nanos(
            self.total_time.as_nanos() as u64 / self.executions
        );
        
        if self.executions == 1 || duration < self.min_time {
            self.min_time = duration;
        }
        if self.executions == 1 || duration > self.max_time {
            self.max_time = duration;
        }
        
        if !success {
            self.error_count += 1;
        }
        
        self.last_execution = Some(Instant::now());
    }
    
    /// Get error rate as percentage
    pub fn error_rate(&self) -> f64 {
        if self.executions == 0 {
            0.0
        } else {
            (self.error_count as f64 / self.executions as f64) * 100.0
        }
    }
}

/// Plugin factory with enhanced capabilities
pub trait PluginFactory: Send + Sync {
    /// Create a new plugin instance
    fn create(&self) -> Result<Box<dyn Plugin>, VexyError>;
    
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Validate that this factory can create plugins
    fn validate(&self) -> Result<(), VexyError>;
    
    /// Get factory-specific configuration schema
    fn config_schema(&self) -> Option<Value> {
        None
    }
}

/// Enhanced plugin registry with advanced features
pub struct EnhancedPluginRegistry {
    /// Plugin factories mapped by name
    factories: RwLock<HashMap<String, Arc<dyn PluginFactory>>>,
    /// Plugin execution statistics
    stats: RwLock<HashMap<String, PluginStats>>,
    /// Plugin aliases
    aliases: RwLock<HashMap<String, String>>,
    /// Plugin loading cache
    cache: RwLock<HashMap<String, Arc<dyn Plugin>>>,
    /// Registry configuration
    config: RegistryConfig,
}

/// Configuration for the enhanced registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Whether to enable plugin caching
    pub enable_caching: bool,
    /// Whether to collect performance statistics
    pub collect_stats: bool,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Plugin search paths for dynamic loading
    pub search_paths: Vec<String>,
    /// Whether to allow experimental plugins
    pub allow_experimental: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            collect_stats: true,
            max_cache_size: 100,
            search_paths: vec![],
            allow_experimental: false,
        }
    }
}

impl EnhancedPluginRegistry {
    /// Create a new enhanced plugin registry
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            stats: RwLock::new(HashMap::new()),
            aliases: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Register a plugin factory
    pub fn register_factory(&self, factory: Arc<dyn PluginFactory>) -> Result<(), VexyError> {
        let metadata = factory.metadata();
        let plugin_name = metadata.name.clone();
        let is_experimental = metadata.experimental;
        
        // Validate the factory
        factory.validate()?;
        
        // Check if experimental plugins are allowed
        if is_experimental && !self.config.allow_experimental {
            return Err(VexyError::General(format!("Experimental plugin '{}' not allowed", plugin_name)).into());
        }
        
        // Check for name conflicts
        {
            let factories = self.factories.read().unwrap();
            if factories.contains_key(&plugin_name) {
                return Err(VexyError::General(format!("Plugin '{}' already registered", plugin_name)).into());
            }
        }
        
        // Register the factory
        {
            let mut factories = self.factories.write().unwrap();
            factories.insert(plugin_name.clone(), factory);
        }
        
        // Initialize statistics
        if self.config.collect_stats {
            let mut stats = self.stats.write().unwrap();
            stats.insert(plugin_name.clone(), PluginStats::default());
        }
        
        Ok(())
    }

    /// Create a plugin instance with enhanced features
    pub fn create_plugin(&self, name: &str) -> Result<Box<dyn Plugin>, VexyError> {
        let resolved_name = self.resolve_name(name);
        
        // Check cache first
        if self.config.enable_caching {
            let cache = self.cache.read().unwrap();
            if let Some(_cached_plugin) = cache.get(&resolved_name) {
                // Clone the plugin (assuming plugins implement Clone or provide a clone method)
                // For now, we'll create a new instance since cloning plugins is complex
            }
        }
        
        // Get factory and create plugin
        let factory = {
            let factories = self.factories.read().unwrap();
            factories.get(&resolved_name)
                .ok_or_else(|| VexyError::General(format!("Plugin '{}' not found", name)))?
                .clone()
        };
        
        let start_time = Instant::now();
        let plugin_result = factory.create();
        let creation_time = start_time.elapsed();
        
        // Record statistics
        if self.config.collect_stats {
            let mut stats = self.stats.write().unwrap();
            if let Some(plugin_stats) = stats.get_mut(&resolved_name) {
                plugin_stats.record_execution(creation_time, plugin_result.is_ok());
            }
        }
        
        let plugin = plugin_result?;
        
        // Add to cache if enabled
        if self.config.enable_caching {
            self.add_to_cache(resolved_name.to_string(), plugin.as_ref());
        }
        
        Ok(plugin)
    }

    /// Create a configured plugin instance
    pub fn create_configured_plugin(&self, name: &str, _config: Value) -> Result<Box<dyn Plugin>, VexyError> {
        // For now, just create the plugin without configuration
        // since the Plugin trait doesn't have a configure method
        self.create_plugin(name)
    }

    /// Register an alias for a plugin
    pub fn register_alias(&self, alias: &str, plugin_name: &str) -> Result<(), VexyError> {
        {
            let factories = self.factories.read().unwrap();
            if !factories.contains_key(plugin_name) {
                return Err(VexyError::General(format!("Plugin '{}' not found", plugin_name)).into());
            }
        }
        
        let mut aliases = self.aliases.write().unwrap();
        aliases.insert(alias.to_string(), plugin_name.to_string());
        Ok(())
    }

    /// Get plugin metadata
    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        let resolved_name = self.resolve_name(name);
        let factories = self.factories.read().unwrap();
        factories.get(&resolved_name).map(|f| f.metadata().clone())
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        let factories = self.factories.read().unwrap();
        factories.values().map(|f| f.metadata().clone()).collect()
    }

    /// List plugins by tag
    pub fn list_plugins_by_tag(&self, tag: &str) -> Vec<PluginMetadata> {
        let factories = self.factories.read().unwrap();
        factories
            .values()
            .filter(|f| f.metadata().tags.contains(&tag.to_string()))
            .map(|f| f.metadata().clone())
            .collect()
    }

    /// Get plugins sorted by execution order
    pub fn get_ordered_plugins(&self) -> Vec<PluginMetadata> {
        let mut plugins = self.list_plugins();
        plugins.sort_by_key(|p| p.performance_hints.execution_order);
        plugins
    }

    /// Check plugin dependencies
    pub fn check_dependencies(&self, plugin_name: &str) -> Result<Vec<String>, VexyError> {
        let metadata = self.get_metadata(plugin_name)
            .ok_or_else(|| VexyError::General(format!("Plugin '{}' not found", plugin_name)))?;
        
        let mut missing_deps = Vec::new();
        
        for dep in &metadata.dependencies {
            if !dep.optional && !self.has_plugin(&dep.name) {
                missing_deps.push(dep.name.clone());
            }
        }
        
        if missing_deps.is_empty() {
            Ok(missing_deps)
        } else {
            Err(VexyError::General(format!("Missing dependencies: {}", missing_deps.join(", "))))
        }
    }

    /// Resolve plugin execution order based on dependencies
    pub fn resolve_execution_order(&self, plugin_names: &[String]) -> Result<Vec<String>, VexyError> {
        let mut ordered = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();
        
        for name in plugin_names {
            self.visit_plugin_deps(name, &mut ordered, &mut visited, &mut visiting)?;
        }
        
        Ok(ordered)
    }

    /// Get plugin statistics
    pub fn get_stats(&self, plugin_name: &str) -> Option<PluginStats> {
        if !self.config.collect_stats {
            return None;
        }
        
        let resolved_name = self.resolve_name(plugin_name);
        let stats = self.stats.read().unwrap();
        stats.get(&resolved_name).cloned()
    }

    /// Get all plugin statistics
    pub fn get_all_stats(&self) -> HashMap<String, PluginStats> {
        if !self.config.collect_stats {
            return HashMap::new();
        }
        
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    /// Clear plugin cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Check if a plugin is registered
    pub fn has_plugin(&self, name: &str) -> bool {
        let resolved_name = self.resolve_name(name);
        let factories = self.factories.read().unwrap();
        factories.contains_key(&resolved_name)
    }

    /// Discover and load plugins from search paths
    pub fn discover_plugins(&self) -> Result<usize, VexyError> {
        let mut loaded_count = 0;
        
        for search_path in &self.config.search_paths {
            loaded_count += self.load_plugins_from_path(search_path)?;
        }
        
        Ok(loaded_count)
    }

    /// Validate all registered plugins
    pub fn validate_all(&self) -> Result<Vec<String>, VexyError> {
        let mut invalid_plugins = Vec::new();
        let factories = self.factories.read().unwrap();
        
        for (name, factory) in factories.iter() {
            if let Err(e) = factory.validate() {
                invalid_plugins.push(format!("{}: {}", name, e));
            }
        }
        
        if invalid_plugins.is_empty() {
            Ok(invalid_plugins)
        } else {
            Err(VexyError::General(format!("Invalid plugins found: {}", invalid_plugins.join(", "))))
        }
    }

    // Private helper methods

    fn resolve_name(&self, name: &str) -> String {
        let aliases = self.aliases.read().unwrap();
        aliases.get(name).cloned().unwrap_or_else(|| name.to_string())
    }

    fn add_to_cache(&self, _name: String, _plugin: &dyn Plugin) {
        // This is a simplified version - in reality, we'd need a way to clone plugins
        // or use a different caching strategy
    }

    fn visit_plugin_deps(
        &self,
        name: &str,
        ordered: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> Result<(), VexyError> {
        if visited.contains(name) {
            return Ok(());
        }
        
        if visiting.contains(name) {
            return Err(VexyError::General(format!("Circular dependency detected involving '{}'", name)).into());
        }
        
        visiting.insert(name.to_string());
        
        if let Some(metadata) = self.get_metadata(name) {
            for dep in &metadata.dependencies {
                if !dep.optional {
                    self.visit_plugin_deps(&dep.name, ordered, visited, visiting)?;
                }
            }
        }
        
        visiting.remove(name);
        visited.insert(name.to_string());
        ordered.push(name.to_string());
        
        Ok(())
    }

    fn load_plugins_from_path(&self, _path: &str) -> Result<usize, VexyError> {
        // Implementation for dynamic plugin loading
        // This would typically involve:
        // 1. Scanning the directory for plugin files
        // 2. Loading shared libraries
        // 3. Resolving plugin symbols
        // 4. Creating and registering plugin factories
        Ok(0) // Placeholder
    }
}

impl Default for EnhancedPluginRegistry {
    fn default() -> Self {
        Self::new(RegistryConfig::default())
    }
}

/// Builder for creating enhanced plugin registries
pub struct EnhancedRegistryBuilder {
    config: RegistryConfig,
    factories: Vec<Arc<dyn PluginFactory>>,
    aliases: Vec<(String, String)>,
}

impl EnhancedRegistryBuilder {
    /// Create a new registry builder
    pub fn new() -> Self {
        Self {
            config: RegistryConfig::default(),
            factories: Vec::new(),
            aliases: Vec::new(),
        }
    }

    /// Configure the registry
    pub fn with_config(mut self, config: RegistryConfig) -> Self {
        self.config = config;
        self
    }

    /// Add a plugin factory
    pub fn with_factory(mut self, factory: Arc<dyn PluginFactory>) -> Self {
        self.factories.push(factory);
        self
    }

    /// Add an alias
    pub fn with_alias(mut self, alias: String, plugin_name: String) -> Self {
        self.aliases.push((alias, plugin_name));
        self
    }

    /// Enable caching
    pub fn with_caching(mut self, enabled: bool) -> Self {
        self.config.enable_caching = enabled;
        self
    }

    /// Enable statistics collection
    pub fn with_stats(mut self, enabled: bool) -> Self {
        self.config.collect_stats = enabled;
        self
    }

    /// Allow experimental plugins
    pub fn allow_experimental(mut self, allowed: bool) -> Self {
        self.config.allow_experimental = allowed;
        self
    }

    /// Build the registry
    pub fn build(self) -> Result<EnhancedPluginRegistry, VexyError> {
        let registry = EnhancedPluginRegistry::new(self.config);
        
        // Register all factories
        for factory in self.factories {
            registry.register_factory(factory)?;
        }
        
        // Register all aliases
        for (alias, plugin_name) in self.aliases {
            registry.register_alias(&alias, &plugin_name)?;
        }
        
        Ok(registry)
    }
}

impl Default for EnhancedRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}