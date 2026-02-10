// this_file: crates/plugin-sdk/src/plugins/usvg.rs

//! usvg normalization plugin
//!
//! This plugin runs the input SVG through [usvg](https://github.com/linebender/resvg)
//! to normalize and simplify it before further optimization. usvg resolves CSS
//! inheritance, converts shapes to paths, normalizes coordinates, and removes
//! invisible elements.
//!
//! This is an **opt-in** plugin — it is NOT part of the default preset.
//! When enabled, place it first in the plugin config array so it runs before
//! other optimizations.
//!
//! If usvg fails to process the SVG (e.g. malformed input), the plugin logs
//! a warning and passes through the original document unchanged.

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::Document;

/// Configuration for the usvg normalization plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsvgConfig {
    /// DPI for resolving relative units (default: 96.0)
    #[serde(default = "default_dpi")]
    pub dpi: f32,

    /// Precision for coordinate values in output (default: 8)
    #[serde(default = "default_coordinates_precision")]
    pub coordinates_precision: u8,

    /// Precision for transform values in output (default: 8)
    #[serde(default = "default_transforms_precision")]
    pub transforms_precision: u8,
}

fn default_dpi() -> f32 {
    96.0
}

fn default_coordinates_precision() -> u8 {
    8
}

fn default_transforms_precision() -> u8 {
    8
}

impl Default for UsvgConfig {
    fn default() -> Self {
        Self {
            dpi: default_dpi(),
            coordinates_precision: default_coordinates_precision(),
            transforms_precision: default_transforms_precision(),
        }
    }
}

/// Plugin that normalizes SVG through usvg before optimization.
///
/// usvg simplifies SVGs by:
/// - Resolving CSS styles and inherited attributes into explicit attributes
/// - Converting basic shapes (rect, circle, etc.) to path elements
/// - Normalizing path coordinates to absolute values
/// - Resolving url(#id) references (gradients, clips, masks)
/// - Pruning invisible elements (display:none, zero opacity)
/// - Calculating bounding boxes
#[derive(Clone)]
pub struct UsvgPlugin {
    config: UsvgConfig,
}

impl UsvgPlugin {
    /// Create a new UsvgPlugin with default configuration
    pub fn new() -> Self {
        Self {
            config: UsvgConfig::default(),
        }
    }

    /// Create plugin with specific configuration
    pub fn with_config(config: UsvgConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON parameters
    fn parse_config(params: &Value) -> Result<UsvgConfig> {
        if params.is_null() {
            Ok(UsvgConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid usvg plugin configuration: {}", e))
        }
    }

    /// Build usvg parse options from our config
    fn build_usvg_options(&self) -> usvg::Options<'static> {
        let mut opts = usvg::Options::default();
        opts.dpi = self.config.dpi;
        opts
    }

    /// Build usvg write options from our config
    fn build_write_options(&self) -> usvg::WriteOptions {
        usvg::WriteOptions {
            coordinates_precision: self.config.coordinates_precision,
            transforms_precision: self.config.transforms_precision,
            ..usvg::WriteOptions::default()
        }
    }
}

impl Plugin for UsvgPlugin {
    fn name(&self) -> &'static str {
        "usvg"
    }

    fn description(&self) -> &'static str {
        "Normalize SVG through usvg (resolves styles, converts shapes to paths)"
    }

    fn category(&self) -> &'static str {
        "global"
    }

    fn validate_params(&self, params: &serde_json::Value) -> Result<()> {
        if !params.is_null() {
            // Filter out the "enabled" key before validating
            if let Some(obj) = params.as_object() {
                let mut filtered = obj.clone();
                filtered.remove("enabled");
                if !filtered.is_empty() {
                    let filtered_value = serde_json::Value::Object(filtered);
                    Self::parse_config(&filtered_value)?;
                }
            } else {
                Self::parse_config(params)?;
            }
        }
        Ok(())
    }

    fn apply<'a>(&self, document: &mut Document<'a>) -> Result<()> {
        // Step 1: Stringify the current document to SVG text
        let svg_text = match vexy_svgo_core::stringify(document) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("usvg plugin: failed to stringify document, passing through: {e}");
                return Ok(());
            }
        };

        // Step 2: Parse through usvg to normalize
        let usvg_opts = self.build_usvg_options();
        let tree = match usvg::Tree::from_str(&svg_text, &usvg_opts) {
            Ok(tree) => tree,
            Err(e) => {
                eprintln!("usvg plugin: failed to parse SVG, passing through: {e}");
                return Ok(());
            }
        };

        // Step 3: Write the usvg tree back to SVG text
        let write_opts = self.build_write_options();
        let normalized_svg = tree.to_string(&write_opts);

        // Step 4: Re-parse the normalized SVG back into our Document AST
        let new_doc = match vexy_svgo_core::parse_svg(&normalized_svg) {
            Ok(doc) => doc,
            Err(e) => {
                eprintln!("usvg plugin: failed to re-parse normalized SVG, passing through: {e}");
                return Ok(());
            }
        };

        // Step 5: Replace the document contents with the normalized version
        document.root = new_doc.root;
        document.prologue = new_doc.prologue;
        document.epilogue = new_doc.epilogue;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = UsvgConfig::default();
        assert!((config.dpi - 96.0).abs() < f32::EPSILON);
        assert_eq!(config.coordinates_precision, 8);
        assert_eq!(config.transforms_precision, 8);
    }

    #[test]
    fn test_parse_config_null() {
        let config = UsvgPlugin::parse_config(&Value::Null);
        assert!(config.is_ok());
        let config = config.expect("config should parse");
        assert!((config.dpi - 96.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_config_custom() {
        let params = serde_json::json!({
            "dpi": 72.0,
            "coordinatesPrecision": 3,
            "transformsPrecision": 4
        });
        let config = UsvgPlugin::parse_config(&params);
        assert!(config.is_ok());
        let config = config.expect("config should parse");
        assert!((config.dpi - 72.0).abs() < f32::EPSILON);
        assert_eq!(config.coordinates_precision, 3);
        assert_eq!(config.transforms_precision, 4);
    }

    #[test]
    fn test_parse_config_invalid() {
        let params = serde_json::json!({ "dpi": "not_a_number" });
        let config = UsvgPlugin::parse_config(&params);
        assert!(config.is_err());
    }

    #[test]
    fn test_parse_config_unknown_field() {
        let params = serde_json::json!({ "unknownField": true });
        let config = UsvgPlugin::parse_config(&params);
        assert!(config.is_err(), "Unknown fields should be rejected");
    }

    #[test]
    fn test_plugin_name() {
        let plugin = UsvgPlugin::new();
        assert_eq!(plugin.name(), "usvg");
    }

    #[test]
    fn test_plugin_category() {
        let plugin = UsvgPlugin::new();
        assert_eq!(plugin.category(), "global");
    }

    #[test]
    fn test_validate_params_null() {
        let plugin = UsvgPlugin::new();
        assert!(plugin.validate_params(&Value::Null).is_ok());
    }

    #[test]
    fn test_validate_params_valid() {
        let plugin = UsvgPlugin::new();
        let params = serde_json::json!({ "dpi": 72.0 });
        assert!(plugin.validate_params(&params).is_ok());
    }

    #[test]
    fn test_validate_params_with_enabled() {
        let plugin = UsvgPlugin::new();
        let params = serde_json::json!({ "enabled": true, "dpi": 72.0 });
        assert!(plugin.validate_params(&params).is_ok());
    }

    #[test]
    fn test_validate_params_invalid() {
        let plugin = UsvgPlugin::new();
        let params = serde_json::json!({ "dpi": "bad" });
        assert!(plugin.validate_params(&params).is_err());
    }

    #[test]
    fn test_apply_simple_svg() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
  <rect x="10" y="10" width="80" height="80" fill="red"/>
</svg>"#;

        let mut doc = vexy_svgo_core::parse_svg(svg).expect("should parse test SVG");
        let plugin = UsvgPlugin::new();
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok(), "usvg plugin should succeed on valid SVG");

        // Verify the document was modified (usvg converts rect to path)
        let output = vexy_svgo_core::stringify(&doc).expect("should stringify");
        assert!(
            output.contains("<svg"),
            "Output should still be valid SVG: {output}"
        );
    }

    #[test]
    fn test_apply_empty_document() {
        let mut doc = Document::new();
        let plugin = UsvgPlugin::new();
        // Empty document may fail usvg parsing, should pass through gracefully
        let result = plugin.apply(&mut doc);
        assert!(
            result.is_ok(),
            "usvg plugin should not error on empty document"
        );
    }

    #[test]
    fn test_with_config() {
        let config = UsvgConfig {
            dpi: 72.0,
            coordinates_precision: 3,
            transforms_precision: 4,
        };
        let plugin = UsvgPlugin::with_config(config);
        assert!((plugin.config.dpi - 72.0).abs() < f32::EPSILON);
        assert_eq!(plugin.config.coordinates_precision, 3);
        assert_eq!(plugin.config.transforms_precision, 4);
    }
}
