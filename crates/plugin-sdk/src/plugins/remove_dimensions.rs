// this_file: crates/plugin-sdk/src/plugins/remove_dimensions.rs

//! Removes width and height in presence of viewBox (opposite to removeViewBox)
//!
//! This plugin removes the width and height attributes from the top-level <svg> element
//! and ensures a viewBox attribute is present. If viewBox doesn't exist, it creates one
//! from the width and height values before removing them.
//!
//! Reference: SVGO's removeDimensions plugin

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

use crate::Plugin;

/// Configuration for the removeDimensions plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveDimensionsConfig {}

impl Default for RemoveDimensionsConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Removes width and height in presence of viewBox
pub struct RemoveDimensionsPlugin {
    #[allow(dead_code)]
    config: RemoveDimensionsConfig,
}

impl RemoveDimensionsPlugin {
    pub fn new() -> Self {
        Self {
            #[allow(dead_code)]
            config:                     RemoveDimensionsConfig::default(),
        }
    }

    pub fn with_config(config: RemoveDimensionsConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveDimensionsConfig> {
        if params.is_null() {
            Ok(RemoveDimensionsConfig::default())
        } else {
            serde_json::from_value(params.clone()).map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn process_svg_element(&self, element: &mut Element) {
        if element.name != "svg" {
            return;
        }

        let had_viewbox = element.has_attr("viewBox");

        // If viewBox already exists, just remove width and height
        if had_viewbox {
            element.remove_attr("width");
            element.remove_attr("height");
        } else {
            // Try to create viewBox from width and height if both are present and numeric
            let width_str = element.attr("width");
            let height_str = element.attr("height");

            if let (Some(width_str), Some(height_str)) = (width_str, height_str) {
                // Try to parse width and height as numbers
                if let (Ok(width), Ok(height)) = (width_str.parse::<f64>(), height_str.parse::<f64>()) {
                    // Only proceed if both are valid numbers (not NaN)
                    if !width.is_nan() && !height.is_nan() {
                        // Create viewBox and remove width/height
                        let viewbox = format!("0 0 {} {}", width, height);
                        element.set_attr("viewBox", &viewbox);
                        element.remove_attr("width");
                        element.remove_attr("height");
                    }
                }
            }
        }

        // Note: We do NOT add preserveAspectRatio="xMidYMid meet" here because
        // that is already the SVG spec default value. Adding it explicitly would be
        // redundant and would be stripped by removeUnknownsAndDefaults anyway.
    }

    fn process_element(&self, element: &mut Element) {
        // Process this element if it's an SVG element
        self.process_svg_element(element);

        // Process children recursively
        let mut i = 0;
        while i < element.children.len() {
            if let Node::Element(child) = &mut element.children[i] {
                self.process_element(child);
            }
            i += 1;
        }
    }
}

impl Default for RemoveDimensionsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveDimensionsPlugin {
    fn name(&self) -> &'static str {
        "removeDimensions"
    }

    fn description(&self) -> &'static str {
        "removes width and height in presence of viewBox (opposite to removeViewBox)"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        self.process_element(&mut document.root);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use indexmap::IndexMap;
    use vexy_svgo_core::ast::{Document, Element, Node};

    use super::*;

    fn create_test_document() -> Document<'static> {
        Document {
            root:          Element {
                name:       "svg".into(),
                attributes: IndexMap::new(),
                namespaces: IndexMap::new(),
                children:   vec![],
            },
            prologue:      vec![],
            epilogue:      vec![],
            metadata:      vexy_svgo_core::ast::DocumentMetadata {
                path: None,
                encoding: None,
                version: None,
                ..Default::default()
            },
            memory_budget: None,
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveDimensionsPlugin::new();
        assert_eq!(plugin.name(), "removeDimensions");
        assert_eq!(plugin.description(), "removes width and height in presence of viewBox (opposite to removeViewBox)");
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveDimensionsPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test empty object params
        assert!(plugin.validate_params(&serde_json::json!({})).is_ok());

        // Test invalid params
        assert!(plugin
            .validate_params(&serde_json::json!({
                "invalidParam": true
            }))
            .is_err());
    }

    #[test]
    fn test_remove_dimensions_with_existing_viewbox() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Set up SVG with width, height, and viewBox
        doc.root.set_attr("width", "100");
        doc.root.set_attr("height", "50");
        doc.root.set_attr("viewBox", "0 0 200 100");

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Width and height should be removed, viewBox should remain unchanged
        assert!(!doc.root.has_attr("width"));
        assert!(!doc.root.has_attr("height"));
        assert_eq!(doc.root.attr("viewBox"), Some("0 0 200 100"));
        // preserveAspectRatio is NOT added because "xMidYMid meet" is the SVG default
        assert!(!doc.root.has_attr("preserveAspectRatio"));
    }

    #[test]
    fn test_create_viewbox_from_dimensions() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Set up SVG with only width and height
        doc.root.set_attr("width", "100");
        doc.root.set_attr("height", "50");

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Width and height should be removed, viewBox should be created
        assert!(!doc.root.has_attr("width"));
        assert!(!doc.root.has_attr("height"));
        assert_eq!(doc.root.attr("viewBox"), Some("0 0 100 50"));
        assert!(!doc.root.has_attr("preserveAspectRatio"));
    }

    #[test]
    fn test_decimal_dimensions() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Set up SVG with decimal dimensions
        doc.root.set_attr("width", "100.5");
        doc.root.set_attr("height", "50.25");

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Width and height should be removed, viewBox should be created with decimals
        assert!(!doc.root.has_attr("width"));
        assert!(!doc.root.has_attr("height"));
        assert_eq!(doc.root.attr("viewBox"), Some("0 0 100.5 50.25"));
        assert!(!doc.root.has_attr("preserveAspectRatio"));
    }

    #[test]
    fn test_invalid_dimensions_ignored() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Set up SVG with invalid dimensions
        doc.root.set_attr("width", "invalid");
        doc.root.set_attr("height", "50");

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Width and height should remain since they're not both valid numbers
        assert_eq!(doc.root.attr("width"), Some("invalid"));
        assert_eq!(doc.root.attr("height"), Some("50"));
        assert!(!doc.root.has_attr("viewBox"));
    }

    #[test]
    fn test_missing_dimension_ignored() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Set up SVG with only width
        doc.root.set_attr("width", "100");

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Width should remain since height is missing
        assert_eq!(doc.root.attr("width"), Some("100"));
        assert!(!doc.root.has_attr("viewBox"));
    }

    #[test]
    fn test_only_processes_svg_elements() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Add a rect element with width and height (should not be processed)
        let mut rect = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        rect.set_attr("width", "100");
        rect.set_attr("height", "50");
        rect.set_attr("x", "10");
        rect.set_attr("y", "10");

        doc.root.children.push(Node::Element(rect));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Rect dimensions should remain unchanged
        if let Node::Element(rect) = &doc.root.children[0] {
            assert_eq!(rect.attr("width"), Some("100"));
            assert_eq!(rect.attr("height"), Some("50"));
            assert!(!rect.has_attr("viewBox"));
        } else {
            panic!("Expected rect element");
        }
    }

    #[test]
    fn test_nested_svg_elements() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Set up root SVG
        doc.root.set_attr("width", "200");
        doc.root.set_attr("height", "100");

        // Add nested SVG element
        let mut nested_svg = Element {
            name:       "svg".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        nested_svg.set_attr("width", "100");
        nested_svg.set_attr("height", "50");

        doc.root.children.push(Node::Element(nested_svg));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Root SVG should have viewBox and no dimensions
        assert!(!doc.root.has_attr("width"));
        assert!(!doc.root.has_attr("height"));
        assert_eq!(doc.root.attr("viewBox"), Some("0 0 200 100"));
        assert!(!doc.root.has_attr("preserveAspectRatio"));

        // Nested SVG should also be processed
        if let Node::Element(nested_svg) = &doc.root.children[0] {
            assert!(!nested_svg.has_attr("width"));
            assert!(!nested_svg.has_attr("height"));
            assert_eq!(nested_svg.attr("viewBox"), Some("0 0 100 50"));
            assert!(!nested_svg.has_attr("preserveAspectRatio"));
        } else {
            panic!("Expected nested SVG element");
        }
    }

    #[test]
    fn test_zero_dimensions() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // Set up SVG with zero dimensions
        doc.root.set_attr("width", "0");
        doc.root.set_attr("height", "0");

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Should still create viewBox even with zero dimensions
        assert!(!doc.root.has_attr("width"));
        assert!(!doc.root.has_attr("height"));
        assert_eq!(doc.root.attr("viewBox"), Some("0 0 0 0"));
        assert!(!doc.root.has_attr("preserveAspectRatio"));
    }

    #[test]
    fn test_existing_preserve_aspect_ratio_is_not_overwritten() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        doc.root.set_attr("width", "100");
        doc.root.set_attr("height", "50");
        doc.root.set_attr("viewBox", "0 0 100 50");
        doc.root.set_attr("preserveAspectRatio", "none");

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        assert_eq!(doc.root.attr("preserveAspectRatio"), Some("none"));
    }

    #[test]
    fn test_no_dimensions_no_change() {
        let plugin = RemoveDimensionsPlugin::new();
        let mut doc = create_test_document();

        // SVG with no width, height, or viewBox
        let original_count = doc.root.attributes.len();

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Should not add any attributes
        assert_eq!(doc.root.attributes.len(), original_count);
        assert!(!doc.root.has_attr("width"));
        assert!(!doc.root.has_attr("height"));
        assert!(!doc.root.has_attr("viewBox"));
    }
}
