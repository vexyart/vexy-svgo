// this_file: crates/core/src/features.rs

//! Feature flag management for Vexy SVGO
//!
//! This module provides a centralized way to manage feature flags across the project,
//! allowing for conditional compilation and runtime feature detection.

use std::collections::HashMap;
use std::fmt;

use once_cell::sync::Lazy;

/// Feature flags available in Vexy SVGO
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// Enable parallel processing for large files
    ParallelProcessing,
    /// Enable streaming parser for memory efficiency
    StreamingParser,
    /// Enable advanced geometric optimizations (requires lyon)
    GeometricOptimizations,
    /// Enable SIMD optimizations (requires target support)
    SimdOptimizations,
    /// Enable experimental plugins
    ExperimentalPlugins,
    /// Enable debug assertions and logging
    DebugMode,
    /// Enable WebAssembly-specific optimizations
    WasmOptimizations,
    /// Enable memory profiling
    MemoryProfiling,
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Feature::ParallelProcessing => write!(f, "ParallelProcessing"),
            Feature::StreamingParser => write!(f, "StreamingParser"),
            Feature::GeometricOptimizations => write!(f, "GeometricOptimizations"),
            Feature::SimdOptimizations => write!(f, "SimdOptimizations"),
            Feature::ExperimentalPlugins => write!(f, "ExperimentalPlugins"),
            Feature::DebugMode => write!(f, "DebugMode"),
            Feature::WasmOptimizations => write!(f, "WasmOptimizations"),
            Feature::MemoryProfiling => write!(f, "MemoryProfiling"),
        }
    }
}

/// Feature flag configuration
pub struct FeatureFlags {
    enabled: HashMap<Feature, bool>,
}

impl FeatureFlags {
    /// Create a new feature flag configuration with defaults
    pub fn new() -> Self {
        let mut enabled = HashMap::new();

        // Default feature states
        enabled.insert(Feature::ParallelProcessing, cfg!(feature = "parallel"));
        enabled.insert(Feature::StreamingParser, true);
        enabled.insert(Feature::GeometricOptimizations, cfg!(feature = "lyon"));
        enabled.insert(Feature::SimdOptimizations, cfg!(target_feature = "simd128"));
        enabled.insert(Feature::ExperimentalPlugins, cfg!(feature = "experimental"));
        enabled.insert(Feature::DebugMode, cfg!(debug_assertions));
        enabled.insert(Feature::WasmOptimizations, cfg!(target_arch = "wasm32"));
        enabled.insert(Feature::MemoryProfiling, cfg!(feature = "memory-profiling"));

        Self { enabled }
    }

    /// Check if a feature is enabled
    pub fn is_enabled(&self, feature: Feature) -> bool {
        self.enabled.get(&feature).copied().unwrap_or(false)
    }

    /// Enable a feature at runtime (if supported)
    pub fn enable(&mut self, feature: Feature) -> Result<(), FeatureError> {
        // Check if feature requires compile-time support
        match feature {
            Feature::GeometricOptimizations | Feature::SimdOptimizations => {
                // For these features, we check if they were enabled at compile time
                // by checking the cfg! macro directly, as they cannot be enabled at runtime
                if !match feature {
                    Feature::GeometricOptimizations => cfg!(feature = "lyon"),
                    Feature::SimdOptimizations => cfg!(target_feature = "simd128"),
                    _ => false, // Should not happen due to match arm
                } {
                    return Err(FeatureError::RequiresCompileTimeSupport(feature));
                }
            },
            _ => {},
        }

        self.enabled.insert(feature, true);
        Ok(())
    }

    /// Disable a feature at runtime
    pub fn disable(&mut self, feature: Feature) {
        self.enabled.insert(feature, false);
    }

    /// Get all enabled features
    pub fn enabled_features(&self) -> Vec<Feature> {
        self.enabled.iter().filter_map(|(f, &enabled)| if enabled { Some(*f) } else { None }).collect()
    }
}

impl fmt::Display for FeatureFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let features: Vec<String> = self.enabled_features().iter().map(|f| format!("{f:?}")).collect();
        write!(f, "Features: [{}]", features.join(", "))
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Feature flag errors
#[derive(Debug, thiserror::Error)]
pub enum FeatureError {
    #[error("Feature {0} requires compile-time support")]
    RequiresCompileTimeSupport(Feature),

    #[error("Feature {0} is not available on this platform")]
    NotAvailableOnPlatform(Feature),
}

/// Global feature flags instance
pub static FEATURES: Lazy<parking_lot::RwLock<FeatureFlags>> =
    Lazy::new(|| parking_lot::RwLock::new(FeatureFlags::new()));

/// Check if a feature is enabled globally
pub fn is_feature_enabled(feature: Feature) -> bool {
    FEATURES.read().is_enabled(feature)
}

/// Enable a feature globally
pub fn enable_feature(feature: Feature) -> Result<(), FeatureError> {
    FEATURES.write().enable(feature)
}

/// Disable a feature globally
pub fn disable_feature(feature: Feature) {
    FEATURES.write().disable(feature)
}

/// Get all enabled features
pub fn enabled_features() -> Vec<Feature> {
    FEATURES.read().enabled_features()
}

/// Macro for conditional compilation based on features
#[macro_export]
macro_rules! if_feature {
    ($feature:expr, $then:expr, $else:expr) => {
        if $crate::features::is_feature_enabled($feature) {
            $then
        } else {
            $else
        }
    };
    ($feature:expr, $then:expr) => {
        if $crate::features::is_feature_enabled($feature) {
            $then
        }
    };
}

/// Macro for debug logging that's compiled out in release
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if $crate::features::is_feature_enabled($crate::features::Feature::DebugMode) {
            eprintln!("[DEBUG] {}", format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flags_default() {
        let flags = FeatureFlags::new();

        // Streaming parser should be enabled by default
        assert!(flags.is_enabled(Feature::StreamingParser));

        // Debug mode depends on compilation mode
        assert_eq!(flags.is_enabled(Feature::DebugMode), cfg!(debug_assertions));
    }

    #[test]
    fn test_enable_disable_features() {
        let mut flags = FeatureFlags::new();

        // Enable experimental plugins
        flags.enable(Feature::ExperimentalPlugins).ok();
        assert!(flags.is_enabled(Feature::ExperimentalPlugins));

        // Disable it
        flags.disable(Feature::ExperimentalPlugins);
        assert!(!flags.is_enabled(Feature::ExperimentalPlugins));
    }

    #[test]
    fn test_compile_time_features() {
        let mut flags = FeatureFlags::new();

        // Try to enable a compile-time feature when not compiled with it
        if !cfg!(feature = "lyon") {
            let result = flags.enable(Feature::GeometricOptimizations);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_enabled_features_list() {
        let flags = FeatureFlags::new();
        let enabled = flags.enabled_features();

        // Should have at least streaming parser
        assert!(enabled.contains(&Feature::StreamingParser));
    }
}
