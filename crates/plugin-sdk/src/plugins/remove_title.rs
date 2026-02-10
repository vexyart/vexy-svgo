// this_file: crates/plugin-sdk/src/plugins/remove_title.rs

//! Remove title plugin implementation
//!
//! This plugin removes all <title> elements from the SVG document.
//! Title elements provide accessibility information but can add size
//! to the SVG when not needed.
//!
//! Reference: SVGO's removeTitle plugin

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration parameters for remove title plugin (currently empty)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveTitleConfig {
    // No configuration options - matches SVGO behavior
}

impl Default for RemoveTitleConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin that removes title elements
pub struct RemoveTitlePlugin {
    #[allow(dead_code)]
    config: RemoveTitleConfig,
}

impl RemoveTitlePlugin {
    /// Create a new RemoveTitlePlugin
    pub fn new() -> Self {
        Self {
            #[allow(dead_code)]
            config: RemoveTitleConfig::default(),
        }
    }

    /// Create a new RemoveTitlePlugin with config
    pub fn with_config(config: RemoveTitleConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<RemoveTitleConfig> {
        if params.is_null() || (params.is_object() && params.as_object().unwrap().is_empty()) {
            Ok(RemoveTitleConfig::default())
        } else if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(RemoveTitleConfig::default())
        }
    }

    /// Remove title elements recursively
    fn remove_title_recursive(&self, element: &mut Element) {
        // Remove title elements from children
        element.children.retain(|child| {
            if let Node::Element(elem) = child {
                elem.name != "title"
            } else {
                true
            }
        });

        // Process remaining child elements recursively
        for child in &mut element.children {
            if let Node::Element(elem) = child {
                self.remove_title_recursive(elem);
            }
        }
    }
}

impl Default for RemoveTitlePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveTitlePlugin {
    fn name(&self) -> &'static str {
        "removeTitle"
    }

    fn description(&self) -> &'static str {
        "removes <title>"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        self.remove_title_recursive(&mut document.root);
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

    fn create_text(content: &str) -> Node<'static> {
        Node::Text(content.to_string().into())
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveTitlePlugin::new();
        assert_eq!(plugin.name(), "removeTitle");
        assert_eq!(plugin.description(), "removes <title>");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveTitlePlugin::new();

        // Valid parameters (empty object)
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Invalid parameters (non-empty object)
        assert!(plugin.validate_params(&json!({"param": "value"})).is_err());
    }

    #[test]
    fn test_removes_title_element() {
        let plugin = RemoveTitlePlugin::new();
        let mut doc = Document::new();

        // Create SVG with title
        let mut svg = create_element("svg");
        let mut title = create_element("title");
        title.children.push(create_text("My SVG Title"));
        svg.children.push(Node::Element(title));
        svg.children.push(Node::Element(create_element("rect")));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that title was removed
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_removes_multiple_titles() {
        let plugin = RemoveTitlePlugin::new();
        let mut doc = Document::new();

        // Create SVG with multiple titles
        let mut svg = create_element("svg");
        let mut title1 = create_element("title");
        title1.children.push(create_text("Title 1"));
        let mut title2 = create_element("title");
        title2.children.push(create_text("Title 2"));

        svg.children.push(Node::Element(title1));
        svg.children.push(Node::Element(create_element("rect")));
        svg.children.push(Node::Element(title2));
        svg.children.push(Node::Element(create_element("circle")));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all titles were removed
        assert_eq!(doc.root.children.len(), 2);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
        if let Node::Element(elem) = &doc.root.children[1] {
            assert_eq!(elem.name, "circle");
        }
    }

    #[test]
    fn test_removes_nested_titles() {
        let plugin = RemoveTitlePlugin::new();
        let mut doc = Document::new();

        // Create SVG with nested titles
        let mut svg = create_element("svg");
        let mut title1 = create_element("title");
        title1.children.push(create_text("Root Title"));
        svg.children.push(Node::Element(title1));

        let mut group = create_element("g");
        let mut title2 = create_element("title");
        title2.children.push(create_text("Group Title"));
        group.children.push(Node::Element(title2));
        group.children.push(Node::Element(create_element("rect")));

        svg.children.push(Node::Element(group));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all titles were removed at all levels
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(group_elem) = &doc.root.children[0] {
            assert_eq!(group_elem.name, "g");
            assert_eq!(group_elem.children.len(), 1);
            if let Node::Element(rect_elem) = &group_elem.children[0] {
                assert_eq!(rect_elem.name, "rect");
            }
        }
    }

    #[test]
    fn test_preserves_other_elements() {
        let plugin = RemoveTitlePlugin::new();
        let mut doc = Document::new();

        // Create SVG with various elements
        let mut svg = create_element("svg");
        svg.children.push(Node::Element(create_element("defs")));
        svg.children.push(Node::Element(create_element("rect")));
        svg.children.push(Node::Element(create_element("circle")));
        svg.children.push(Node::Element(create_element("path")));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all non-title elements are preserved
        assert_eq!(doc.root.children.len(), 4);
        let element_names: Vec<&str> = doc
            .root
            .children
            .iter()
            .filter_map(|child| {
                if let Node::Element(elem) = child {
                    Some(elem.name.as_ref())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(element_names, vec!["defs", "rect", "circle", "path"]);
    }

    #[test]
    fn test_empty_svg() {
        let plugin = RemoveTitlePlugin::new();
        let mut doc = Document::new();

        // Create empty SVG
        let svg = create_element("svg");
        doc.root = svg;

        // Apply plugin - should not crash
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // SVG should still be empty
        assert_eq!(doc.root.children.len(), 0);
    }

    #[test]
    fn test_title_with_attributes() {
        let plugin = RemoveTitlePlugin::new();
        let mut doc = Document::new();

        // Create SVG with title that has attributes
        let mut svg = create_element("svg");
        let mut title = create_element("title");
        title.set_attr("id", "myTitle");
        title.set_attr("class", "title-class");
        title.children.push(create_text("Title with attributes"));
        svg.children.push(Node::Element(title));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that title was removed regardless of attributes
        assert_eq!(doc.root.children.len(), 0);
    }

    #[test]
    fn test_preserves_text_nodes() {
        let plugin = RemoveTitlePlugin::new();
        let mut doc = Document::new();

        // Create SVG with mixed content
        let mut svg = create_element("svg");
        svg.children.push(create_text("Before title"));
        let mut title = create_element("title");
        title.children.push(create_text("Title"));
        svg.children.push(Node::Element(title));
        svg.children.push(create_text("After title"));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that text nodes are preserved
        assert_eq!(doc.root.children.len(), 2);
        if let Node::Text(text) = &doc.root.children[0] {
            assert_eq!(text.as_ref(), "Before title");
        }
        if let Node::Text(text) = &doc.root.children[1] {
            assert_eq!(text.as_ref(), "After title");
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = RemoveTitlePlugin::parse_config(&json!({})).unwrap();
        // No fields to check since config is empty
        let _ = config;
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveTitlePlugin, "removeTitle");
