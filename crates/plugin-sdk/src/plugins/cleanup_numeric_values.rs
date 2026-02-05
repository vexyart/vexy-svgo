// this_file: crates/plugin-sdk/src/plugins/cleanup_numeric_values.rs

//! Cleanup numeric values plugin implementation
//!
//! This plugin rounds numeric values to a fixed precision and removes default "px" units.
//! It follows the same pattern as svgo's cleanupNumericValues plugin.

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use vexy_svgo_core::ast::{Document, Element};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;
use regex::Regex;
use once_cell::sync::Lazy;

/// Parameters for the cleanup numeric values plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CleanupNumericValuesParams {
    /// Number of decimal places to round to
    pub float_precision: u8,
    /// Keep leading zeros (0.5 vs .5)
    pub leading_zero: bool,
    /// Remove default "px" units
    pub default_px: bool,
    /// Convert other units to px when beneficial
    pub convert_to_px: bool,
}

impl Default for CleanupNumericValuesParams {
    fn default() -> Self {
        Self {
            float_precision: 3,
            leading_zero: true,
            default_px: true,
            convert_to_px: true,
        }
    }
}

/// Plugin that cleans up numeric values
#[derive(Default)]
pub struct CleanupNumericValuesPlugin {
    params: CleanupNumericValuesParams,
}

impl CleanupNumericValuesPlugin {
    /// Create a new CleanupNumericValuesPlugin with default settings
    pub fn new() -> Self {
        Self {
            params: CleanupNumericValuesParams::default(),
        }
    }

    /// Create plugin with specific parameters
    pub fn with_params(params: CleanupNumericValuesParams) -> Self {
        Self { params }
    }
}

impl Plugin for CleanupNumericValuesPlugin {
    fn name(&self) -> &'static str {
        "cleanupNumericValues"
    }

    fn description(&self) -> &'static str {
        "Round numeric values to the fixed precision, remove default px units"
    }

    fn validate_params(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        // Try to deserialize the params to validate their structure
        serde_json::from_value::<CleanupNumericValuesParams>(params.clone())
            .map_err(|e| anyhow::anyhow!("Invalid parameters: {}", e))?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        // Use the default params for now
        let params = self.params.clone();

        let mut visitor = CleanupNumericValuesVisitor::new(params);
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

// Regex patterns for numeric value detection
static NUMERIC_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(-?\d*\.?\d+(?:[eE][+-]?\d+)?)\s*(px|pt|pc|mm|cm|in|em|ex|%)?").unwrap()
});

static TRANSFORM_NUMERIC: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-?\d*\.?\d+(?:[eE][+-]?\d+)?").unwrap()
});

/// Visitor implementation that cleans up numeric values
struct CleanupNumericValuesVisitor {
    params: CleanupNumericValuesParams,
}

impl CleanupNumericValuesVisitor {
    fn new(params: CleanupNumericValuesParams) -> Self {
        Self { params }
    }

    fn round_numeric_value(&self, value: f64) -> String {
        if value.fract() == 0.0 {
            // Integer value
            format!("{:.0}", value)
        } else {
            let multiplier = 10f64.powi(self.params.float_precision as i32);
            let rounded = (value * multiplier).round() / multiplier;
            
            let mut result = format!("{:.prec$}", rounded, prec = self.params.float_precision as usize);
            
            // Remove trailing zeros after decimal point
            if result.contains('.') {
                result = result.trim_end_matches('0').trim_end_matches('.').to_string();
            }
            
            // Handle leading zero
            if !self.params.leading_zero && result.starts_with("0.") {
                result = result[1..].to_string();
            } else if !self.params.leading_zero && result.starts_with("-0.") {
                result = format!("-{}", &result[2..]);
            }
            
            result
        }
    }

    fn optimize_numeric_string(&self, value: &str, attribute_name: &str) -> String {
        // Special handling for transform attributes
        if matches!(attribute_name, "transform" | "gradientTransform" | "patternTransform") {
            return self.optimize_transform_value(value);
        }

        // For other attributes, use the general numeric pattern
        NUMERIC_PATTERN.replace_all(value, |caps: &regex::Captures| {
            let number_str = &caps[1];
            let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            if let Ok(num) = number_str.parse::<f64>() {
                let rounded = self.round_numeric_value(num);
                
                // Handle unit optimization
                if unit == "px" && self.params.default_px && self.is_default_px_context(attribute_name) {
                    // Remove px unit when it's the default
                    rounded
                } else if !unit.is_empty() {
                    // Keep the unit
                    format!("{}{}", rounded, unit)
                } else {
                    rounded
                }
            } else {
                caps[0].to_string()
            }
        }).to_string()
    }

    fn optimize_transform_value(&self, value: &str) -> String {
        TRANSFORM_NUMERIC.replace_all(value, |caps: &regex::Captures| {
            let number_str = &caps[0];
            if let Ok(num) = number_str.parse::<f64>() {
                self.round_numeric_value(num)
            } else {
                caps[0].to_string()
            }
        }).to_string()
    }

    fn is_default_px_context(&self, attribute_name: &str) -> bool {
        // These attributes default to px units in SVG
        matches!(
            attribute_name,
            "x" | "y" | "x1" | "y1" | "x2" | "y2" | 
            "width" | "height" | "rx" | "ry" | "r" |
            "cx" | "cy" | "fx" | "fy" |
            "markerWidth" | "markerHeight" |
            "refX" | "refY" |
            "stroke-width" | "stroke-dasharray" | "stroke-dashoffset" |
            "font-size" | "letter-spacing" | "word-spacing" |
            "baseline-shift"
        )
    }

    fn should_process_attribute(&self, name: &str) -> bool {
        // Process numeric attributes but skip certain ones that need exact values
        !matches!(
            name,
            "version" | "id" | "class" | "preserveAspectRatio" | "xml:space"
        )
    }

    fn process_style_value(&self, style: &str) -> String {
        // Process CSS style values
        let mut result = String::new();
        
        for declaration in style.split(';') {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }
            
            if let Some((property, value)) = declaration.split_once(':') {
                let property = property.trim();
                let value = value.trim();
                
                let optimized = if self.is_numeric_css_property(property) {
                    self.optimize_numeric_string(value, property)
                } else {
                    value.to_string()
                };
                
                if !result.is_empty() {
                    result.push_str("; ");
                }
                result.push_str(&format!("{}: {}", property, optimized));
            } else {
                if !result.is_empty() {
                    result.push_str("; ");
                }
                result.push_str(declaration);
            }
        }
        
        result
    }

    fn is_numeric_css_property(&self, property: &str) -> bool {
        matches!(
            property,
            "font-size" | "letter-spacing" | "word-spacing" |
            "line-height" | "stroke-width" | "stroke-dasharray" |
            "stroke-dashoffset" | "opacity" | "fill-opacity" |
            "stroke-opacity" | "stop-opacity" | "flood-opacity" |
            "baseline-shift" | "kerning" | "margin" | "padding" |
            "border-width" | "border-radius"
        )
    }
}

impl Visitor<'_> for CleanupNumericValuesVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Process regular attributes
        for (name, value) in element.attributes.iter_mut() {
            if self.should_process_attribute(name) && !value.is_empty() {
                if name == "style" {
                    // Special handling for style attribute
                    let optimized = self.process_style_value(value);
                    if optimized != value.as_ref() {
                        *value = optimized.into();
                    }
                } else {
                    let optimized = self.optimize_numeric_string(value, name);
                    if optimized != value.as_ref() {
                        *value = optimized.into();
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use serde_json::json;
    use vexy_svgo_core::ast::{Document, Element};

    #[test]
    fn test_plugin_creation() {
        let plugin = CleanupNumericValuesPlugin::new();
        assert_eq!(plugin.name(), "cleanupNumericValues");
        assert_eq!(plugin.params.float_precision, 3);
        assert!(plugin.params.leading_zero);
        assert!(plugin.params.default_px);
        assert!(plugin.params.convert_to_px);
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = CleanupNumericValuesPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({"floatPrecision": 2}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"leadingZero": false}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"defaultPx": true, "convertToPx": false}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"floatPrecision": "invalid"}))
            .is_err());
        assert!(plugin.validate_params(&json!({"leadingZero": 123})).is_err());
    }

    #[test]
    fn test_numeric_rounding() {
        let params = CleanupNumericValuesParams::default();
        let visitor = CleanupNumericValuesVisitor::new(params);

        // Test basic rounding
        assert_eq!(visitor.round_numeric_value(1.23456), "1.235");
        assert_eq!(visitor.round_numeric_value(1.0), "1");
        assert_eq!(visitor.round_numeric_value(0.5), "0.5");
        assert_eq!(visitor.round_numeric_value(-1.23456), "-1.235");

        // Test without leading zero
        let params = CleanupNumericValuesParams {
            leading_zero: false,
            ..Default::default()
        };
        let visitor = CleanupNumericValuesVisitor::new(params);
        assert_eq!(visitor.round_numeric_value(0.5), ".5");
        assert_eq!(visitor.round_numeric_value(-0.5), "-.5");
    }

    #[test]
    fn test_unit_handling() {
        let params = CleanupNumericValuesParams::default();
        let visitor = CleanupNumericValuesVisitor::new(params);

        // Test px removal
        assert_eq!(visitor.optimize_numeric_string("10px", "width"), "10");
        assert_eq!(visitor.optimize_numeric_string("10px", "height"), "10");
        
        // Test keeping other units
        assert_eq!(visitor.optimize_numeric_string("10em", "width"), "10em");
        assert_eq!(visitor.optimize_numeric_string("50%", "width"), "50%");
        
        // Test non-default-px contexts
        assert_eq!(visitor.optimize_numeric_string("10px", "stroke"), "10px");
    }

    #[test]
    fn test_transform_optimization() {
        let params = CleanupNumericValuesParams::default();
        let visitor = CleanupNumericValuesVisitor::new(params);

        assert_eq!(
            visitor.optimize_numeric_string("translate(10.12345, 20.98765)", "transform"),
            "translate(10.123, 20.988)"
        );
        
        assert_eq!(
            visitor.optimize_numeric_string("scale(1.00000)", "transform"),
            "scale(1)"
        );
    }

    #[test]
    fn test_style_processing() {
        let params = CleanupNumericValuesParams::default();
        let visitor = CleanupNumericValuesVisitor::new(params);

        assert_eq!(
            visitor.process_style_value("font-size: 14.5678px; stroke-width: 2.0000"),
            "font-size: 14.568; stroke-width: 2"
        );
        
        assert_eq!(
            visitor.process_style_value("opacity: 0.50000; fill: red"),
            "opacity: 0.5; fill: red"
        );
    }

    #[test]
    fn test_plugin_apply() {
        let plugin = CleanupNumericValuesPlugin::new();
        let mut doc = Document::new();

        // Add attributes to root element for testing
        doc.root.set_attr("width", "100.12345px");
        doc.root.set_attr("height", "50.00000");
        doc.root.set_attr("transform", "scale(1.23456789)");
        doc.root.set_attr("style", "stroke-width: 2.50000px");

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that values were cleaned
        assert_eq!(doc.root.attr("width"), Some("100.123"));
        assert_eq!(doc.root.attr("height"), Some("50"));
        assert_eq!(doc.root.attr("transform"), Some("scale(1.235)"));
        assert_eq!(doc.root.attr("style"), Some("stroke-width: 2.5"));
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(CleanupNumericValuesPlugin, "cleanupNumericValues");