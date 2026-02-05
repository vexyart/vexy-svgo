// this_file: crates/plugin-sdk/src/plugins/cleanup_attrs.rs

//! Cleanup attributes plugin implementation
//!
//! This plugin cleans up attribute values by removing newlines, trimming whitespace,
//! and collapsing multiple spaces into single spaces. This follows the same pattern
//! as svgo's cleanupAttrs plugin.

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use vexy_svgo_core::ast::{Document, Element};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Parameters for the cleanup attrs plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CleanupAttrsParams {
    /// Remove newlines from attribute values
    pub newlines: bool,
    /// Trim leading and trailing whitespace from attribute values
    pub trim: bool,
    /// Collapse multiple spaces into single space
    pub spaces: bool,
}

impl Default for CleanupAttrsParams {
    fn default() -> Self {
        Self {
            newlines: true,
            trim: true,
            spaces: true,
        }
    }
}

/// Plugin that cleans up attribute values
#[derive(Default)]
pub struct CleanupAttrsPlugin {
    params: CleanupAttrsParams,
}

impl CleanupAttrsPlugin {
    /// Create a new CleanupAttrsPlugin with default settings
    pub fn new() -> Self {
        Self {
            params: CleanupAttrsParams::default(),
        }
    }

    /// Create plugin with specific parameters
    pub fn with_params(params: CleanupAttrsParams) -> Self {
        Self { params }
    }
}

impl Plugin for CleanupAttrsPlugin {
    fn name(&self) -> &'static str {
        "cleanupAttrs"
    }

    fn description(&self) -> &'static str {
        "Cleanup attributes from newlines, trailing and repeating spaces"
    }

    fn validate_params(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        // Accept null or empty object as valid (use defaults)
        if params.is_null() || params.as_object().map_or(false, |obj| obj.is_empty()) {
            return Ok(());
        }
        // Try to deserialize the params to validate their structure
        serde_json::from_value::<CleanupAttrsParams>(params.clone())
            .map_err(|e| anyhow::anyhow!("Invalid parameters: {}", e))?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        // Use the default params for now
        let params = self.params.clone();

        let mut visitor = CleanupAttrsVisitor::new(params);
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that cleans up attribute values
struct CleanupAttrsVisitor {
    params: CleanupAttrsParams,
}

impl CleanupAttrsVisitor {
    fn new(params: CleanupAttrsParams) -> Self {
        Self { params }
    }

    fn clean_attribute_value(&self, value: &str) -> String {
        let mut result = value.to_string();

        // Remove newlines if requested
        if self.params.newlines {
            result = result.replace('\n', " ").replace('\r', " ");
        }

        // Trim whitespace if requested
        if self.params.trim {
            result = result.trim().to_string();
        }

        // Collapse multiple spaces if requested
        if self.params.spaces {
            let mut cleaned = String::with_capacity(result.len());
            let mut prev_space = false;
            
            for ch in result.chars() {
                if ch.is_whitespace() {
                    if !prev_space {
                        cleaned.push(' ');
                        prev_space = true;
                    }
                } else {
                    cleaned.push(ch);
                    prev_space = false;
                }
            }

            result = cleaned;
        }

        // Final trim if we've done any processing
        if self.params.newlines || self.params.spaces {
            result = result.trim().to_string();
        }

        result
    }

    fn should_cleanup_attribute(&self, name: &str) -> bool {
        // Skip certain attributes that should preserve their exact formatting
        !matches!(
            name,
            "xml:space" | "preserveAspectRatio" | "viewBox" | "points" | "d"
        )
    }
}

impl Visitor<'_> for CleanupAttrsVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Clean up attribute values
        for (name, value) in element.attributes.iter_mut() {
            if self.should_cleanup_attribute(name) && !value.is_empty() {
                let cleaned = self.clean_attribute_value(value);
                if cleaned != value.as_ref() {
                    *value = cleaned.into();
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
        let plugin = CleanupAttrsPlugin::new();
        assert_eq!(plugin.name(), "cleanupAttrs");
        assert!(plugin.params.newlines);
        assert!(plugin.params.trim);
        assert!(plugin.params.spaces);
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = CleanupAttrsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({"newlines": true}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"trim": false}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"spaces": true}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"newlines": false, "trim": true, "spaces": false}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"newlines": "invalid"}))
            .is_err());
        assert!(plugin.validate_params(&json!({"trim": 123})).is_err());
    }

    #[test]
    fn test_attribute_value_cleaning() {
        let params = CleanupAttrsParams::default();
        let visitor = CleanupAttrsVisitor::new(params);

        // Test newline removal
        assert_eq!(
            visitor.clean_attribute_value("value\nwith\nnewlines"),
            "value with newlines"
        );

        // Test trimming
        assert_eq!(
            visitor.clean_attribute_value("  value  "),
            "value"
        );

        // Test space collapsing
        assert_eq!(
            visitor.clean_attribute_value("value    with    spaces"),
            "value with spaces"
        );

        // Test combined
        assert_eq!(
            visitor.clean_attribute_value("  value\n  with  \n  all   issues  "),
            "value with all issues"
        );

        // Test empty and whitespace-only
        assert_eq!(visitor.clean_attribute_value(""), "");
        assert_eq!(visitor.clean_attribute_value("   "), "");
    }

    #[test]
    fn test_selective_params() {
        // Test with only newlines enabled
        let params = CleanupAttrsParams {
            newlines: true,
            trim: false,
            spaces: false,
        };
        let visitor = CleanupAttrsVisitor::new(params);
        assert_eq!(
            visitor.clean_attribute_value("  value\nwith\nnewlines  "),
            "value with newlines"
        );

        // Test with only trim enabled
        let params = CleanupAttrsParams {
            newlines: false,
            trim: true,
            spaces: false,
        };
        let visitor = CleanupAttrsVisitor::new(params);
        assert_eq!(
            visitor.clean_attribute_value("  value  "),
            "value"
        );

        // Test with only spaces enabled
        let params = CleanupAttrsParams {
            newlines: false,
            trim: false,
            spaces: true,
        };
        let visitor = CleanupAttrsVisitor::new(params);
        assert_eq!(
            visitor.clean_attribute_value("value    with    spaces"),
            "value with spaces"
        );
    }

    #[test]
    fn test_plugin_apply() {
        let plugin = CleanupAttrsPlugin::new();
        let mut doc = Document::new();

        // Add attributes to root element for testing
        doc.root.set_attr("class", "  my-class  ");
        doc.root.set_attr("title", "Title\nwith\nnewlines");
        doc.root.set_attr("style", "color:    red;    font-size:    14px");

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that attributes were cleaned
        assert_eq!(doc.root.attr("class"), Some("my-class"));
        assert_eq!(doc.root.attr("title"), Some("Title with newlines"));
        assert_eq!(doc.root.attr("style"), Some("color: red; font-size: 14px"));
    }

    #[test]
    fn test_skip_certain_attributes() {
        let visitor = CleanupAttrsVisitor::new(CleanupAttrsParams::default());

        // These attributes should not be cleaned up
        assert!(!visitor.should_cleanup_attribute("xml:space"));
        assert!(!visitor.should_cleanup_attribute("preserveAspectRatio"));
        assert!(!visitor.should_cleanup_attribute("viewBox"));
        assert!(!visitor.should_cleanup_attribute("points"));
        assert!(!visitor.should_cleanup_attribute("d"));

        // These should be cleaned up
        assert!(visitor.should_cleanup_attribute("class"));
        assert!(visitor.should_cleanup_attribute("style"));
        assert!(visitor.should_cleanup_attribute("title"));
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(CleanupAttrsPlugin, "cleanupAttrs");