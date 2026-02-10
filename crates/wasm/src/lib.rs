// this_file: crates/wasm/src/lib.rs

//! WebAssembly bindings for Vexy SVGO
//!
//! This module provides JavaScript-friendly exports for using Vexy SVGO in web browsers
//! and Node.js environments through WebAssembly.

#[cfg(feature = "size-optimization")]
pub mod minimal;

#[cfg(target_arch = "wasm32")]
pub mod enhanced;

#[cfg(target_arch = "wasm32")]
pub mod wasm_impl {
    use serde::{Deserialize, Serialize};
    use vexy_svgo_core::{optimize_with_config, Config};
    use wasm_bindgen::prelude::*;

    // When the `wasm` feature is enabled, use wee_alloc as the global allocator
    // to reduce WASM bundle size
    #[cfg(feature = "wasm")]
    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

    // Initialize panic hook for better error messages in the browser console
    // Only include in debug builds to save space
    #[cfg(debug_assertions)]
    #[wasm_bindgen(start)]
    pub fn init() {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
    }

    // Minimal init for release builds
    #[cfg(not(debug_assertions))]
    #[wasm_bindgen(start)]
    pub fn init() {
        // No-op in release to save space
    }

    /// JavaScript-friendly configuration object
    #[wasm_bindgen]
    #[derive(Serialize, Deserialize)]
    pub struct JsConfig {
        /// Path to the SVG file (optional, used for error messages)
        path:          Option<String>,
        /// Multipass optimization (run until no changes)
        pub multipass: bool,
        /// Pretty print output
        pub pretty:    bool,
        /// Indentation size for pretty printing
        pub indent:    u8,
        /// Plugins configuration as JSON string
        plugins_json:  String,
    }

    #[wasm_bindgen]
    impl JsConfig {
        #[wasm_bindgen(getter)]
        pub fn path(&self) -> Option<String> {
            self.path.clone()
        }

        #[wasm_bindgen(setter)]
        pub fn set_path(&mut self, path: Option<String>) {
            self.path = path;
        }
    }

    #[wasm_bindgen]
    impl JsConfig {
        /// Create a new configuration with default values
        #[wasm_bindgen(constructor)]
        pub fn new() -> Self {
            Self {
                path:         None,
                multipass:    false,
                pretty:       false,
                indent:       2,
                plugins_json: "{}".to_string(),
            }
        }

        /// Set the plugins configuration from a JSON string
        #[wasm_bindgen(js_name = setPlugins)]
        pub fn set_plugins(&mut self, plugins_json: &str) {
            self.plugins_json = plugins_json.to_string();
        }

        /// Get the plugins configuration as a JSON string
        #[wasm_bindgen(js_name = getPlugins)]
        pub fn get_plugins(&self) -> String {
            self.plugins_json.clone()
        }
    }

    impl Default for JsConfig {
        fn default() -> Self {
            Self::new()
        }
    }

    /// JavaScript-friendly result object
    #[wasm_bindgen]
    pub struct JsOptimizationResult {
        data:               String,
        error:              Option<String>,
        pub original_size:  usize,
        pub optimized_size: usize,
    }

    #[wasm_bindgen]
    impl JsOptimizationResult {
        /// Get the optimized SVG data
        #[wasm_bindgen(getter)]
        pub fn data(&self) -> String {
            self.data.clone()
        }

        /// Get the error message (if any)
        #[wasm_bindgen(getter)]
        pub fn error(&self) -> Option<String> {
            self.error.clone()
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

        /// Get the compression ratio (0.0 to 1.0)
        #[wasm_bindgen(getter, js_name = compressionRatio)]
        pub fn compression_ratio(&self) -> f64 {
            if self.original_size == 0 {
                1.0
            } else {
                self.optimized_size as f64 / self.original_size as f64
            }
        }

        /// Get the size reduction percentage
        #[wasm_bindgen(getter, js_name = sizeReduction)]
        pub fn size_reduction(&self) -> f64 {
            (1.0 - self.compression_ratio()) * 100.0
        }
    }

    /// Main optimization function exposed to JavaScript
    #[wasm_bindgen]
    pub fn optimize(svg: &str, config: Option<JsConfig>) -> Result<JsOptimizationResult, JsError> {
        let config = config.unwrap_or_default();
        let original_size = svg.len();

        // Convert JavaScript config to native config
        let mut native_config = Config::with_default_preset();
        native_config.multipass = config.multipass;
        native_config.js2svg.pretty = config.pretty;
        native_config.js2svg.indent = config.indent.to_string();

        // Parse plugins configuration
        if !config.plugins_json.is_empty() && config.plugins_json != "{}" {
            match serde_json::from_str::<serde_json::Value>(&config.plugins_json) {
                Ok(plugins) => {
                    if let Some(plugins_obj) = plugins.as_object() {
                        for (name, enabled) in plugins_obj {
                            if let Some(enabled) = enabled.as_bool() {
                                native_config.set_plugin_enabled(name, enabled);
                            }
                        }
                    }
                },
                Err(e) => {
                    return Ok(JsOptimizationResult {
                        data: svg.to_string(),
                        error: Some(format!("Invalid plugins configuration: {}", e)),
                        original_size,
                        optimized_size: original_size,
                    });
                },
            }
        }

        // Optimize the SVG
        match optimize_with_config(svg, native_config) {
            Ok(result) => Ok(JsOptimizationResult {
                data: result.data.clone(),
                error: None,
                original_size,
                optimized_size: result.data.len(),
            }),
            Err(e) => Ok(JsOptimizationResult {
                data: svg.to_string(),
                error: Some(e.to_string()),
                original_size,
                optimized_size: original_size,
            }),
        }
    }

    /// Optimize SVG with default configuration
    #[wasm_bindgen(js_name = optimizeDefault)]
    pub fn optimize_default(svg: &str) -> Result<JsOptimizationResult, JsError> {
        optimize(svg, None)
    }

    /// Get the version of the Vexy SVGO library
    #[wasm_bindgen(js_name = getVersion)]
    pub fn get_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Get list of available plugins
    #[wasm_bindgen(js_name = getPlugins)]
    pub fn get_plugins() -> Result<String, JsError> {
        let config = Config::with_default_preset();
        let plugins: Vec<String> = config.plugins.iter().map(|p| p.name().to_string()).collect();
        serde_json::to_string(&plugins).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Get default preset configuration
    #[wasm_bindgen(js_name = getDefaultPreset)]
    pub fn get_default_preset() -> Result<String, JsError> {
        let config = Config::with_default_preset();
        serde_json::to_string(&config).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Optimize SVG in chunks for better memory management with large files
    #[wasm_bindgen(js_name = optimizeChunked)]
    pub fn optimize_chunked(
        svg: &str,
        config: Option<JsConfig>,
        chunk_size: usize,
    ) -> Result<JsOptimizationResult, JsError> {
        // For now, just call the regular optimize function
        // In the future, this could be implemented to process large SVGs in chunks
        let _ = chunk_size; // Suppress unused warning
        optimize(svg, config)
    }

    /// Free memory hint for JavaScript - can be called after processing large SVGs
    #[wasm_bindgen(js_name = freeMemory)]
    pub fn free_memory() {
        // This is a hint for JavaScript garbage collection
        // In Rust/WASM, memory is managed automatically
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_js_config_default() {
            let config = JsConfig::new();
            assert_eq!(config.multipass, false);
            assert_eq!(config.pretty, false);
            assert_eq!(config.indent, 2);
            assert_eq!(config.plugins_json, "{}");
        }

        #[test]
        fn test_js_config_plugins() {
            let mut config = JsConfig::new();
            config.set_plugins("{\"removeComments\": true}");
            assert_eq!(config.get_plugins(), "{\"removeComments\": true}");
        }

        #[test]
        fn test_optimization_result() {
            let result = JsOptimizationResult {
                data:           "optimized".to_string(),
                error:          None,
                original_size:  100,
                optimized_size: 50,
            };

            assert_eq!(result.compression_ratio(), 0.5);
            assert_eq!(result.size_reduction(), 50.0);
        }
    }
}

// Re-export the WASM functionality when targeting WASM
#[cfg(target_arch = "wasm32")]
#[cfg(target_arch = "wasm32")]
pub use enhanced::*;

// Provide stub implementations for non-WASM targets
#[cfg(not(target_arch = "wasm32"))]
pub mod stub {
    //! Stub implementations for non-WASM targets
    //!
    //! This module provides empty implementations of WASM-specific functionality
    //! when not targeting WebAssembly, allowing the code to compile on all platforms.
}
