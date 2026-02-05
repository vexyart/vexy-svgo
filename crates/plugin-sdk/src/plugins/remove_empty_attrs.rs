// this_file: crates/plugin-sdk/src/plugins/remove_empty_attrs.rs

//! Remove empty attributes plugin implementation
//!
//! This plugin demonstrates the new visitor-based architecture for removing
//! empty attributes from SVG elements, following the same pattern as svgo's
//! removeEmptyAttrs plugin.

use crate::Plugin;
use anyhow::Result;
use vexy_svgo_core::ast::{Document, Element};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Plugin that removes empty attributes from SVG elements
#[derive(Default)]
pub struct RemoveEmptyAttrsPlugin {
    preserve_class: bool,
    preserve_id: bool,
}

impl RemoveEmptyAttrsPlugin {
    /// Create a new RemoveEmptyAttrsPlugin with default settings
    pub fn new() -> Self {
        Self {
            preserve_class: false,
            preserve_id: false,
        }
    }

    /// Create plugin with specific preservation settings
    pub fn with_preserve_settings(preserve_class: bool, preserve_id: bool) -> Self {
        Self {
            preserve_class,
            preserve_id,
        }
    }
}

impl Plugin for RemoveEmptyAttrsPlugin {
    fn name(&self) -> &'static str {
        "removeEmptyAttrs"
    }

    fn description(&self) -> &'static str {
        "Remove empty attributes from SVG elements"
    }

    fn validate_params(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        if let Some(preserve_class) = params.get("preserveClass") {
            if !preserve_class.is_boolean() {
                return Err(anyhow::anyhow!("preserveClass must be a boolean"));
            }
        }
        if let Some(preserve_id) = params.get("preserveId") {
            if !preserve_id.is_boolean() {
                return Err(anyhow::anyhow!("preserveId must be a boolean"));
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let mut visitor = EmptyAttrRemovalVisitor::new(self.preserve_class, self.preserve_id);
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that removes empty attributes
struct EmptyAttrRemovalVisitor {
    preserve_class: bool,
    preserve_id: bool,
}

impl EmptyAttrRemovalVisitor {
    fn new(preserve_class: bool, preserve_id: bool) -> Self {
        Self {
            preserve_class,
            preserve_id,
        }
    }

    fn should_preserve_attribute(&self, name: &str) -> bool {
        match name {
            "class" => self.preserve_class,
            "id" => self.preserve_id,
            // Conditional processing attributes should always be preserved when empty
            // as they have semantic meaning (empty = false, missing = true)
            "requiredExtensions" | "requiredFeatures" | "systemLanguage" => true,
            _ => false,
        }
    }

    fn is_empty_value(&self, value: &str) -> bool {
        value.trim().is_empty()
    }
}

impl Visitor<'_> for EmptyAttrRemovalVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Remove empty attributes from the element
        element.attributes.retain(|name, value| {
            // Keep non-empty attributes
            if !self.is_empty_value(value) {
                return true;
            }

            // Keep empty attributes that should be preserved
            self.should_preserve_attribute(name)
        });

        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use vexy_svgo_core::ast::{Document, Element};

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveEmptyAttrsPlugin::new();
        assert_eq!(plugin.name(), "removeEmptyAttrs");
        assert!(!plugin.preserve_class);
        assert!(!plugin.preserve_id);

        let plugin2 = RemoveEmptyAttrsPlugin::with_preserve_settings(true, true);
        assert!(plugin2.preserve_class);
        assert!(plugin2.preserve_id);
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveEmptyAttrsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({"preserveClass": true}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"preserveId": false}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"preserveClass": true, "preserveId": false}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"preserveClass": "invalid"}))
            .is_err());
        assert!(plugin.validate_params(&json!({"preserveId": 123})).is_err());
    }

    #[test]
    fn test_visitor_attribute_filtering() {
        let visitor = EmptyAttrRemovalVisitor::new(false, false);

        // Test empty value detection
        assert!(visitor.is_empty_value(""));
        assert!(visitor.is_empty_value("   "));
        assert!(visitor.is_empty_value("\t\n"));
        assert!(!visitor.is_empty_value("value"));
        assert!(!visitor.is_empty_value(" value "));

        // Test preservation logic
        assert!(!visitor.should_preserve_attribute("class"));
        assert!(!visitor.should_preserve_attribute("id"));
        assert!(!visitor.should_preserve_attribute("stroke"));

        let visitor2 = EmptyAttrRemovalVisitor::new(true, true);
        assert!(visitor2.should_preserve_attribute("class"));
        assert!(visitor2.should_preserve_attribute("id"));
        assert!(!visitor2.should_preserve_attribute("stroke"));
    }

    #[test]
    fn test_plugin_apply() {
        let plugin = RemoveEmptyAttrsPlugin::new();
        let mut doc = Document::new();

        // Add attributes to root element for testing
        doc.root.set_attr("fill", "red");
        doc.root.set_attr("stroke", "");
        doc.root.set_attr("opacity", "  ");
        doc.root.set_attr("class", "");

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that empty attributes were removed
        assert!(doc.root.attributes.contains_key("fill")); // Non-empty, should remain
        assert!(!doc.root.attributes.contains_key("stroke")); // Empty, should be removed
        assert!(!doc.root.attributes.contains_key("opacity")); // Whitespace only, should be removed
        assert!(!doc.root.attributes.contains_key("class")); // Empty and not preserved, should be removed
    }

    #[test]
    fn test_plugin_apply_with_preservation() {
        let plugin = RemoveEmptyAttrsPlugin::with_preserve_settings(true, true);
        let mut doc = Document::new();

        // Add attributes to root element for testing
        doc.root.set_attr("fill", "red");
        doc.root.set_attr("stroke", "");
        doc.root.set_attr("class", "");
        doc.root.set_attr("id", "");

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check preservation behavior
        assert!(doc.root.attributes.contains_key("fill")); // Non-empty, should remain
        assert!(!doc.root.attributes.contains_key("stroke")); // Empty and not preserved, should be removed
        assert!(doc.root.attributes.contains_key("class")); // Empty but preserved, should remain
        assert!(doc.root.attributes.contains_key("id")); // Empty but preserved, should remain
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveEmptyAttrsPlugin, "removeEmptyAttrs");
