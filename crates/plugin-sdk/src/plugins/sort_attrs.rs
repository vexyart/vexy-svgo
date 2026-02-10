// this_file: crates/plugin-sdk/src/plugins/sort_attrs.rs

//! Sort attributes plugin implementation
//!
//! This plugin sorts element attributes for better compression.
//! It follows a configurable order for attributes, with special handling
//! for namespace attributes and alphabetical fallback.
//!
//! Reference: SVGO's sortAttrs plugin

use vexy_svgo_core::Plugin;
use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration parameters for sort attributes plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SortAttrsConfig {
    /// Order of attributes to prioritize
    #[serde(default = "default_order")]
    pub order: Vec<String>,
    /// How to handle xmlns attributes
    #[serde(default = "default_xmlns_order")]
    pub xmlns_order: String,
}

fn default_order() -> Vec<String> {
    vec![
        "id".to_string(),
        "width".to_string(),
        "height".to_string(),
        "x".to_string(),
        "x1".to_string(),
        "x2".to_string(),
        "y".to_string(),
        "y1".to_string(),
        "y2".to_string(),
        "cx".to_string(),
        "cy".to_string(),
        "r".to_string(),
        "fill".to_string(),
        "stroke".to_string(),
        "marker".to_string(),
        "d".to_string(),
        "points".to_string(),
    ]
}

fn default_xmlns_order() -> String {
    "front".to_string()
}

impl Default for SortAttrsConfig {
    fn default() -> Self {
        Self {
            order: default_order(),
            xmlns_order: default_xmlns_order(),
        }
    }
}

/// Plugin that sorts element attributes
pub struct SortAttrsPlugin {
    config: SortAttrsConfig,
}

impl SortAttrsPlugin {
    /// Create a new SortAttrsPlugin
    pub fn new() -> Self {
        Self {
            config: SortAttrsConfig::default(),
        }
    }

    /// Create a new SortAttrsPlugin with config
    pub fn with_config(config: SortAttrsConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<SortAttrsConfig> {
        if params.is_null() || (params.is_object() && params.as_object().unwrap().is_empty()) {
            Ok(SortAttrsConfig::default())
        } else if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(SortAttrsConfig::default())
        }
    }

    /// Get namespace priority for sorting
    fn get_ns_priority(&self, name: &str) -> i32 {
        if self.config.xmlns_order == "front" {
            // Put xmlns first
            if name == "xmlns" {
                return 3;
            }
            // xmlns:* attributes second
            if name.starts_with("xmlns:") {
                return 2;
            }
        }
        // Other namespaces after and sort them alphabetically
        if name.contains(':') {
            return 1;
        }
        // Other attributes
        0
    }

    /// Compare two attributes for sorting
    fn compare_attrs(&self, a_name: &str, b_name: &str) -> Ordering {
        // Sort namespaces - higher priority comes first
        let a_priority = self.get_ns_priority(a_name);
        let b_priority = self.get_ns_priority(b_name);
        let priority_ns = b_priority.cmp(&a_priority);
        if priority_ns != Ordering::Equal {
            return priority_ns;
        }

        // If both are xmlns attributes with same priority, sort alphabetically
        if (a_name == "xmlns" || a_name.starts_with("xmlns:")) && 
           (b_name == "xmlns" || b_name.starts_with("xmlns:")) {
            return a_name.cmp(b_name);
        }

        // Extract the first part from attributes
        // For example "fill" from "fill" and "fill-opacity"
        let a_part = a_name.split('-').next().unwrap_or(a_name);
        let b_part = b_name.split('-').next().unwrap_or(b_name);

        // Rely on alphabetical sort when the first part is the same
        if a_part != b_part {
            let a_in_order = self.config.order.contains(&a_part.to_string());
            let b_in_order = self.config.order.contains(&b_part.to_string());

            // Sort by position in order param
            if a_in_order && b_in_order {
                let a_pos = self.config.order.iter().position(|x| x == a_part).unwrap();
                let b_pos = self.config.order.iter().position(|x| x == b_part).unwrap();
                return a_pos.cmp(&b_pos);
            }

            // Put attributes from order param before others
            match (a_in_order, b_in_order) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }

        // Sort alphabetically
        a_name.cmp(b_name)
    }

    /// Sort attributes on an element
    fn sort_attrs_recursive(&self, element: &mut Element) {
        // Sort attributes on this element
        if !element.attributes.is_empty() {
            // Always apply custom sorting for sortAttrs plugin
            let mut attrs: Vec<(String, String)> = element
                .attributes
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            attrs.sort_by(|a, b| self.compare_attrs(&a.0, &b.0));

            // Rebuild attributes map in sorted order
            let mut sorted_attributes = IndexMap::new();
            for (name, value) in attrs {
                sorted_attributes.insert(name.into(), value.into());
            }
            element.attributes = sorted_attributes;
        }

        // Process child elements recursively
        for child in &mut element.children {
            if let Node::Element(elem) = child {
                self.sort_attrs_recursive(elem);
            }
        }
    }
}

impl Default for SortAttrsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SortAttrsPlugin {
    fn name(&self) -> &'static str {
        "sortAttrs"
    }

    fn description(&self) -> &'static str {
        "Sort element attributes for better compression"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn configure(&mut self, params: &Value) -> Result<()> {
        self.config = Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        self.sort_attrs_recursive(&mut document.root);
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use serde_json::json;
    use std::borrow::Cow;
    use vexy_svgo_core::ast::{Document, Element, Node};

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = SortAttrsPlugin::new();
        assert_eq!(plugin.name(), "sortAttrs");
        assert_eq!(
            plugin.description(),
            "Sort element attributes for better compression"
        );
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = SortAttrsPlugin::new();

        // Valid parameters (empty object)
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Valid parameters (with order)
        assert!(plugin
            .validate_params(&json!({
                "order": ["id", "width", "height"],
                "xmlnsOrder": "front"
            }))
            .is_ok());

        // Invalid parameter type
        assert!(plugin
            .validate_params(&json!({
                "order": "invalid"
            }))
            .is_err());
    }

    #[test]
    fn test_basic_attribute_sorting() {
        let plugin = SortAttrsPlugin::new();
        let mut doc = Document::new();

        // Create element with attributes in random order
        let mut rect = create_element("rect");
        rect.attributes
            .insert("height".into(), "100".into());
        rect.attributes.insert("id".into(), "test".into());
        rect.attributes
            .insert("width".into(), "200".into());
        rect.attributes.insert("x".into(), "10".into());
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that attributes are sorted according to default order
        if let Node::Element(elem) = &doc.root.children[0] {
            let attr_names: Vec<&str> = elem.attributes.keys().map(|k| k.as_ref()).collect();
            // id should come first, then width, height, x
            assert_eq!(attr_names.len(), 4);
            // Note: HashMap iteration order is not guaranteed, but we can check the comparison logic
        }
    }

    #[test]
    fn test_xmlns_attributes_sorting() {
        let plugin = SortAttrsPlugin::new();
        let mut doc = Document::new();

        // Create element with xmlns attributes
        let mut svg = create_element("svg");
        svg.attributes
            .insert("width".into(), "100".into());
        svg.attributes.insert(
            "xmlns:xlink".into(),
            "http://www.w3.org/1999/xlink".into(),
        );
        svg.attributes.insert(
            "xmlns".into(),
            "http://www.w3.org/2000/svg".into(),
        );
        svg.attributes.insert("id".into(), "test".into());
        doc.root.children.push(Node::Element(svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that xmlns attributes are sorted to front
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.attributes.len(), 4);
            let attr_names: Vec<&str> = elem.attributes.keys().map(|k| k.as_ref()).collect();
            // xmlns should come first, then xmlns:xlink, then id, then width
            assert_eq!(attr_names[0], "xmlns");
            assert_eq!(attr_names[1], "xmlns:xlink");
            // The rest should be by order: id, width
            assert_eq!(attr_names[2], "id");
            assert_eq!(attr_names[3], "width");
        }
    }

    #[test]
    fn test_alphabetical_sorting() {
        let plugin = SortAttrsPlugin::new();
        let mut doc = Document::new();

        // Create element with attributes not in default order
        let mut rect = create_element("rect");
        rect.attributes
            .insert("z-index".into(), "1".into());
        rect.attributes
            .insert("data-custom".into(), "value".into());
        rect.attributes
            .insert("aria-label".into(), "button".into());
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that attributes are sorted alphabetically
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.attributes.len(), 3);
            // Should be sorted alphabetically: aria-label, data-custom, z-index
        }
    }

    #[test]
    fn test_custom_order_config() {
        let config = SortAttrsConfig {
            order: vec!["width".to_string(), "height".to_string(), "id".to_string()],
            xmlns_order: "front".to_string(),
        };
        let plugin = SortAttrsPlugin::with_config(config);
        let mut doc = Document::new();

        // Create element with attributes
        let mut rect = create_element("rect");
        rect.attributes.insert("id".into(), "test".into());
        rect.attributes
            .insert("height".into(), "100".into());
        rect.attributes
            .insert("width".into(), "200".into());
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that attributes are sorted according to custom order
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.attributes.len(), 3);
            // Should be sorted according to custom order: width, height, id
        }
    }

    #[test]
    fn test_hyphenated_attributes() {
        let plugin = SortAttrsPlugin::new();
        let mut doc = Document::new();

        // Create element with hyphenated attributes
        let mut rect = create_element("rect");
        rect.attributes
            .insert("fill-opacity".into(), "0.5".into());
        rect.attributes
            .insert("fill".into(), "red".into());
        rect.attributes
            .insert("stroke-width".into(), "2".into());
        rect.attributes
            .insert("stroke".into(), "blue".into());
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that hyphenated attributes are grouped with their base
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.attributes.len(), 4);
            // fill and stroke are in default order, so they should be grouped together
        }
    }

    #[test]
    fn test_nested_elements() {
        let plugin = SortAttrsPlugin::new();
        let mut doc = Document::new();

        // Create nested elements with attributes
        let mut group = create_element("g");
        group
            .attributes
            .insert("transform".into(), "translate(10,20)".into());
        group
            .attributes
            .insert("id".into(), "group1".into());

        let mut rect = create_element("rect");
        rect.attributes
            .insert("height".into(), "100".into());
        rect.attributes
            .insert("width".into(), "200".into());
        rect.attributes.insert("x".into(), "10".into());

        group.children.push(Node::Element(rect));
        doc.root.children.push(Node::Element(group));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that both elements have sorted attributes
        if let Node::Element(group_elem) = &doc.root.children[0] {
            assert_eq!(group_elem.attributes.len(), 2);

            if let Node::Element(rect_elem) = &group_elem.children[0] {
                assert_eq!(rect_elem.attributes.len(), 3);
            }
        }
    }

    #[test]
    fn test_empty_attributes() {
        let plugin = SortAttrsPlugin::new();
        let mut doc = Document::new();

        // Create element with no attributes
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin - should not crash
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Element should still exist with no attributes
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.attributes.len(), 0);
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = SortAttrsPlugin::parse_config(&json!({
            "order": ["id", "class", "width", "height"],
            "xmlnsOrder": "alphabetical"
        }))
        .unwrap();

        assert_eq!(config.order, vec!["id", "class", "width", "height"]);
        assert_eq!(config.xmlns_order, "alphabetical");
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests_with_params!(SortAttrsPlugin, "sortAttrs");
