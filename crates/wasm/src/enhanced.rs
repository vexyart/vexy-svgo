// this_file: crates/wasm/src/enhanced.rs

//! Enhanced WebAssembly bindings with comprehensive API
//!
//! This module provides advanced features for WASM usage including:
//! - Streaming API for large files
//! - Plugin management
//! - Detailed error information
//! - Performance metrics
//! - Memory management utilities

use std::collections::HashMap;

use vexy_svgo_core::features::{enable_feature, is_feature_enabled, Feature};
use vexy_svgo_core::{create_default_registry, optimize_with_config, parse_svg, Config, PluginConfig};
use wasm_bindgen::prelude::*;

/// Advanced configuration with full control over optimization
#[wasm_bindgen]
pub struct EnhancedConfig {
    inner:            Config,
    plugin_params:    HashMap<String, serde_json::Value>,
    performance_mode: PerformanceMode,
    error_handling:   ErrorHandlingMode,
}

#[derive(Clone, Copy, Debug)]
pub enum PerformanceMode {
    /// Optimize for speed
    Speed,
    /// Optimize for compression ratio
    Compression,
    /// Balance between speed and compression
    Balanced,
}

#[derive(Clone, Copy, Debug)]
pub enum ErrorHandlingMode {
    /// Stop on first error
    Strict,
    /// Continue on recoverable errors
    Lenient,
    /// Try to fix common issues automatically
    AutoFix,
}

#[wasm_bindgen]
impl EnhancedConfig {
    /// Create a new enhanced configuration
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner:            Config::with_default_preset(),
            plugin_params:    HashMap::new(),
            performance_mode: PerformanceMode::Balanced,
            error_handling:   ErrorHandlingMode::Lenient,
        }
    }

    /// Create configuration from JSON
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<EnhancedConfig, JsError> {
        let inner: Config =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("Invalid config JSON: {}", e)))?;

        Ok(Self {
            inner,
            plugin_params: HashMap::new(),
            performance_mode: PerformanceMode::Balanced,
            error_handling: ErrorHandlingMode::Lenient,
        })
    }

    /// Export configuration to JSON
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsError> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| JsError::new(&format!("Failed to serialize config: {}", e)))
    }

    /// Set multipass optimization
    #[wasm_bindgen(setter)]
    pub fn set_multipass(&mut self, enabled: bool) {
        self.inner.multipass = enabled;
    }

    /// Get multipass optimization setting
    #[wasm_bindgen(getter)]
    pub fn multipass(&self) -> bool {
        self.inner.multipass
    }

    /// Set pretty printing
    #[wasm_bindgen(setter)]
    pub fn set_pretty(&mut self, enabled: bool) {
        self.inner.js2svg.pretty = enabled;
    }

    /// Get pretty printing setting
    #[wasm_bindgen(getter)]
    pub fn pretty(&self) -> bool {
        self.inner.js2svg.pretty
    }

    /// Enable or disable a specific plugin
    #[wasm_bindgen(js_name = setPluginEnabled)]
    pub fn set_plugin_enabled(&mut self, name: &str, enabled: bool) {
        self.inner.set_plugin_enabled(name, enabled);
    }

    /// Configure a plugin with parameters
    #[wasm_bindgen(js_name = configurePlugin)]
    pub fn configure_plugin(&mut self, name: &str, params_json: &str) -> Result<(), JsError> {
        let params: serde_json::Value =
            serde_json::from_str(params_json).map_err(|e| JsError::new(&format!("Invalid plugin params: {}", e)))?;

        self.plugin_params.insert(name.to_string(), params.clone());

        // Update the plugin configuration
        self.inner.plugins.retain(|p| p.name() != name);
        self.inner.plugins.push(PluginConfig::WithParams {
            name:   name.to_string(),
            params: params.clone(),
        });

        Ok(())
    }

    /// Set performance mode
    #[wasm_bindgen(js_name = setPerformanceMode)]
    pub fn set_performance_mode(&mut self, mode: &str) {
        self.performance_mode = match mode {
            "speed" => PerformanceMode::Speed,
            "compression" => PerformanceMode::Compression,
            _ => PerformanceMode::Balanced,
        };
    }

    /// Set error handling mode
    #[wasm_bindgen(js_name = setErrorHandling)]
    pub fn set_error_handling(&mut self, mode: &str) {
        self.error_handling = match mode {
            "strict" => ErrorHandlingMode::Strict,
            "autofix" => ErrorHandlingMode::AutoFix,
            _ => ErrorHandlingMode::Lenient,
        };
    }
}

/// Detailed optimization result with metrics
#[wasm_bindgen]
pub struct EnhancedResult {
    data:           String,
    original_size:  usize,
    optimized_size: usize,
    errors:         Vec<String>,
    warnings:       Vec<String>,
    metrics:        PerformanceMetrics,
}

#[wasm_bindgen]
impl EnhancedResult {
    /// Get the optimized SVG data
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> String {
        self.data.clone()
    }

    /// Get the original size in bytes
    #[wasm_bindgen(getter, js_name = originalSize)]
    pub fn original_size(&self) -> usize {
        self.original_size
    }

    /// Get the optimized size in bytes
    #[wasm_bindgen(getter, js_name = optimizedSize)]
    pub fn optimized_size(&self) -> usize {
        self.optimized_size
    }

    /// Get compression ratio
    #[wasm_bindgen(getter, js_name = compressionRatio)]
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            1.0
        } else {
            self.optimized_size as f64 / self.original_size as f64
        }
    }

    /// Get size reduction percentage
    #[wasm_bindgen(getter, js_name = sizeReduction)]
    pub fn size_reduction(&self) -> f64 {
        (1.0 - self.compression_ratio()) * 100.0
    }

    /// Get errors as JSON array
    #[wasm_bindgen(js_name = getErrors)]
    pub fn get_errors(&self) -> String {
        serde_json::to_string(&self.errors).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get warnings as JSON array
    #[wasm_bindgen(js_name = getWarnings)]
    pub fn get_warnings(&self) -> String {
        serde_json::to_string(&self.warnings).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get performance metrics
    #[wasm_bindgen(js_name = getMetrics)]
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.clone()
    }

    /// Check if optimization was successful
    #[wasm_bindgen(getter, js_name = isSuccess)]
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Performance metrics for optimization
#[wasm_bindgen]
#[derive(Clone)]
pub struct PerformanceMetrics {
    pub parse_time_ms:       f64,
    pub optimize_time_ms:    f64,
    pub stringify_time_ms:   f64,
    pub total_time_ms:       f64,
    pub plugins_applied:     u32,
    pub optimization_passes: u32,
    pub elements_processed:  u32,
    pub memory_peak_kb:      f64,
}

/// Plugin information
#[wasm_bindgen]
#[derive(Clone)]
pub struct PluginInfo {
    name:         String,
    description:  String,
    enabled:      bool,
    configurable: bool,
}

#[wasm_bindgen]
impl PluginInfo {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn description(&self) -> String {
        self.description.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    #[wasm_bindgen(getter)]
    pub fn configurable(&self) -> bool {
        self.configurable
    }
}

/// Enhanced optimization with detailed metrics
#[wasm_bindgen(js_name = optimizeEnhanced)]
pub fn optimize_enhanced(svg: &str, config: EnhancedConfig) -> Result<EnhancedResult, JsError> {
    let start_time = web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0);

    let original_size = svg.len();
    let mut errors = Vec::new();
    let warnings = Vec::new();

    // Parse timing
    let parse_start = web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0);

    let _document = match parse_svg(svg) {
        Ok(doc) => doc,
        Err(e) => {
            errors.push(format!("Parse error: {}", e));
            return Ok(EnhancedResult {
                data: svg.to_string(),
                original_size,
                optimized_size: original_size,
                errors,
                warnings,
                metrics: PerformanceMetrics {
                    parse_time_ms:       0.0,
                    optimize_time_ms:    0.0,
                    stringify_time_ms:   0.0,
                    total_time_ms:       0.0,
                    plugins_applied:     0,
                    optimization_passes: 0,
                    elements_processed:  0,
                    memory_peak_kb:      0.0,
                },
            });
        },
    };

    let parse_time = web_sys::window().and_then(|w| w.performance()).map(|p| p.now() - parse_start).unwrap_or(0.0);

    // Optimize timing
    let optimize_start = web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0);

    let result = match optimize_with_config(svg, config.inner.clone()) {
        Ok(res) => res,
        Err(e) => {
            errors.push(format!("Optimization error: {}", e));
            return Ok(EnhancedResult {
                data: svg.to_string(),
                original_size,
                optimized_size: original_size,
                errors,
                warnings,
                metrics: PerformanceMetrics {
                    parse_time_ms:       parse_time,
                    optimize_time_ms:    0.0,
                    stringify_time_ms:   0.0,
                    total_time_ms:       parse_time,
                    plugins_applied:     0,
                    optimization_passes: 0,
                    elements_processed:  0,
                    memory_peak_kb:      0.0,
                },
            });
        },
    };

    let optimize_time =
        web_sys::window().and_then(|w| w.performance()).map(|p| p.now() - optimize_start).unwrap_or(0.0);

    let total_time = web_sys::window().and_then(|w| w.performance()).map(|p| p.now() - start_time).unwrap_or(0.0);

    Ok(EnhancedResult {
        data: result.data.clone(),
        original_size,
        optimized_size: result.data.len(),
        errors,
        warnings,
        metrics: PerformanceMetrics {
            parse_time_ms:       parse_time,
            optimize_time_ms:    optimize_time,
            stringify_time_ms:   0.0, // Not separately measured in current implementation
            total_time_ms:       total_time,
            plugins_applied:     config.inner.plugins.len() as u32,
            optimization_passes: if config.inner.multipass { 2 } else { 1 },
            elements_processed:  0,   // Would need to count during optimization
            memory_peak_kb:      0.0, // WebAssembly doesn't provide easy memory profiling
        },
    })
}

/// Get list of available plugins with metadata
#[wasm_bindgen(js_name = getPluginsWithMetadata)]
pub fn get_plugins() -> Result<Vec<JsValue>, JsError> {
    let registry = create_default_registry();
    let plugins: Vec<JsValue> = registry
        .plugin_names()
        .into_iter()
        .filter_map(|name| registry.create_plugin(name))
        .map(|plugin| {
            let info = PluginInfo {
                name:        plugin.name().to_string(),
                description: plugin.description().to_string(),

                enabled:      true, // Default enabled status
                configurable: true, // Most plugins are configurable
            };
            JsValue::from(info)
        })
        .collect();

    Ok(plugins)
}

/// Streaming optimization for large files
#[wasm_bindgen]
pub struct StreamingOptimizer {
    config: EnhancedConfig,
    buffer: String,
    state:  StreamingState,
}

#[derive(Clone)]
enum StreamingState {
    Ready,
    Processing,
    Complete,
    Error(String),
}

#[wasm_bindgen]
impl StreamingOptimizer {
    /// Create a new streaming optimizer
    #[wasm_bindgen(constructor)]
    pub fn new(config: EnhancedConfig) -> Self {
        Self {
            config,
            buffer: String::new(),
            state: StreamingState::Ready,
        }
    }

    /// Add a chunk of SVG data
    #[wasm_bindgen(js_name = addChunk)]
    pub fn add_chunk(&mut self, chunk: &str) -> Result<(), JsError> {
        match &self.state {
            StreamingState::Ready | StreamingState::Processing => {
                self.buffer.push_str(chunk);
                self.state = StreamingState::Processing;
                Ok(())
            },
            StreamingState::Complete => Err(JsError::new("Optimization already complete")),
            StreamingState::Error(e) => Err(JsError::new(&format!("Optimizer in error state: {}", e))),
        }
    }

    /// Finalize and get the optimized result
    #[wasm_bindgen(js_name = finalize)]
    pub fn finalize(&mut self) -> Result<EnhancedResult, JsError> {
        match &self.state {
            StreamingState::Processing => {
                match optimize_enhanced(&self.buffer, self.config.clone()) {
                    Ok(result) => {
                        self.state = StreamingState::Complete;
                        self.buffer.clear(); // Free memory
                        Ok(result)
                    },
                    Err(e) => {
                        self.state = StreamingState::Error(format!("{:?}", e));
                        self.buffer.clear(); // Free memory
                        Err(e)
                    },
                }
            },
            StreamingState::Ready => Err(JsError::new("No data to optimize")),
            StreamingState::Complete => Err(JsError::new("Already finalized")),
            StreamingState::Error(e) => Err(JsError::new(&format!("Optimizer in error state: {}", e))),
        }
    }

    /// Reset the optimizer for reuse
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.state = StreamingState::Ready;
    }

    /// Get current buffer size
    #[wasm_bindgen(js_name = getBufferSize)]
    pub fn get_buffer_size(&self) -> usize {
        self.buffer.len()
    }
}

/// Feature management
#[wasm_bindgen(js_name = enableFeature)]
pub fn enable_wasm_feature(feature_name: &str) -> Result<(), JsError> {
    let feature = match feature_name {
        "parallel" => Feature::ParallelProcessing,
        "streaming" => Feature::StreamingParser,
        "geometric" => Feature::GeometricOptimizations,
        "simd" => Feature::SimdOptimizations,
        "experimental" => Feature::ExperimentalPlugins,
        "debug" => Feature::DebugMode,
        "wasm" => Feature::WasmOptimizations,
        "memory_profiling" => Feature::MemoryProfiling,
        _ => return Err(JsError::new(&format!("Unknown feature: {}", feature_name))),
    };

    enable_feature(feature).map_err(|e| JsError::new(&format!("Failed to enable feature: {}", e)))
}

/// Check if a feature is enabled
#[wasm_bindgen(js_name = isFeatureEnabled)]
pub fn is_wasm_feature_enabled(feature_name: &str) -> bool {
    let feature = match feature_name {
        "parallel" => Feature::ParallelProcessing,
        "streaming" => Feature::StreamingParser,
        "geometric" => Feature::GeometricOptimizations,
        "simd" => Feature::SimdOptimizations,
        "experimental" => Feature::ExperimentalPlugins,
        "debug" => Feature::DebugMode,
        "wasm" => Feature::WasmOptimizations,
        "memory_profiling" => Feature::MemoryProfiling,
        _ => return false,
    };

    is_feature_enabled(feature)
}

/// Get list of all available features
#[wasm_bindgen(js_name = getAvailableFeatures)]
pub fn get_available_features() -> Vec<JsValue> {
    vec![
        JsValue::from_str("parallel"),
        JsValue::from_str("streaming"),
        JsValue::from_str("geometric"),
        JsValue::from_str("simd"),
        JsValue::from_str("experimental"),
        JsValue::from_str("debug"),
        JsValue::from_str("wasm"),
        JsValue::from_str("memory_profiling"),
    ]
}

/// Validate SVG without optimization
#[wasm_bindgen(js_name = validateSvg)]
pub fn validate_svg(svg: &str) -> Result<ValidationResult, JsError> {
    match parse_svg(svg) {
        Ok(_doc) => {
            let mut issues = Vec::new();

            // Basic validation checks
            if !svg.trim().starts_with('<') {
                issues.push("SVG should start with '<'".to_string());
            }

            // Count elements (simplified)
            let element_count = svg.matches('<').count();

            Ok(ValidationResult {
                valid: issues.is_empty(),
                issues,
                element_count: element_count as u32,
                has_viewbox: svg.contains("viewBox"),
                has_namespace: svg.contains("xmlns"),
            })
        },
        Err(e) => Ok(ValidationResult {
            valid:         false,
            issues:        vec![format!("Parse error: {}", e)],
            element_count: 0,
            has_viewbox:   false,
            has_namespace: false,
        }),
    }
}

/// SVG validation result
#[wasm_bindgen]
pub struct ValidationResult {
    valid:         bool,
    issues:        Vec<String>,
    element_count: u32,
    has_viewbox:   bool,
    has_namespace: bool,
}

#[wasm_bindgen]
impl ValidationResult {
    #[wasm_bindgen(getter)]
    pub fn valid(&self) -> bool {
        self.valid
    }

    #[wasm_bindgen(getter, js_name = elementCount)]
    pub fn element_count(&self) -> u32 {
        self.element_count
    }

    #[wasm_bindgen(getter, js_name = hasViewBox)]
    pub fn has_viewbox(&self) -> bool {
        self.has_viewbox
    }

    #[wasm_bindgen(getter, js_name = hasNamespace)]
    pub fn has_namespace(&self) -> bool {
        self.has_namespace
    }

    #[wasm_bindgen(js_name = getIssues)]
    pub fn get_issues(&self) -> String {
        serde_json::to_string(&self.issues).unwrap_or_else(|_| "[]".to_string())
    }
}

/// Memory utilities
#[wasm_bindgen(js_name = getMemoryUsage)]
pub fn get_memory_usage() -> MemoryInfo {
    // WebAssembly.Memory provides buffer info but not detailed usage
    // This is a simplified version
    MemoryInfo {
        used_kb:  0.0, // Would need runtime support
        total_kb: 0.0,
        peak_kb:  0.0,
    }
}

#[wasm_bindgen]
pub struct MemoryInfo {
    pub used_kb:  f64,
    pub total_kb: f64,
    pub peak_kb:  f64,
}

// Implement Clone for EnhancedConfig manually since HashMap doesn't auto-derive
impl Clone for EnhancedConfig {
    fn clone(&self) -> Self {
        Self {
            inner:            self.inner.clone(),
            plugin_params:    self.plugin_params.clone(),
            performance_mode: self.performance_mode,
            error_handling:   self.error_handling,
        }
    }
}
