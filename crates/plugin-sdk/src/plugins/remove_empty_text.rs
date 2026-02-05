// this_file: crates/plugin-sdk/src/plugins/remove_empty_text.rs

//! Remove empty text plugin implementation
//!
//! This plugin removes empty text elements from SVG documents.
//! It handles:
//! - Empty `<text>` elements
//! - Empty `<tspan>` elements  
//! - `<tref>` elements with empty xlink:href attributes
//!
//! Reference: https://www.w3.org/TR/SVG11/text.html

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration parameters for remove empty text plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveEmptyTextConfig {
    /// Remove empty text elements (default: true)
    #[serde(default = "default_true")]
    pub text: bool,
    /// Remove empty tspan elements (default: true)
    #[serde(default = "default_true")]
    pub tspan: bool,
    /// Remove tref elements with empty xlink:href (default: true)
    #[serde(default = "default_true")]
    pub tref: bool,
}

fn default_true() -> bool {
    true
}

impl Default for RemoveEmptyTextConfig {
    fn default() -> Self {
        Self {
            text: true,
            tspan: true,
            tref: true,
        }
    }
}

/// Plugin that removes empty text elements
pub struct RemoveEmptyTextPlugin {
    config: RemoveEmptyTextConfig,
}

impl RemoveEmptyTextPlugin {
    /// Create a new RemoveEmptyTextPlugin
    pub fn new() -> Self {
        Self {
            config: RemoveEmptyTextConfig::default(),
        }
    }

    /// Create a new RemoveEmptyTextPlugin with config
    pub fn with_config(config: RemoveEmptyTextConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<RemoveEmptyTextConfig> {
        if params.is_null() || (params.is_object() && params.as_object().unwrap().is_empty()) {
            Ok(RemoveEmptyTextConfig::default())
        } else if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(RemoveEmptyTextConfig::default())
        }
    }

    /// Recursively remove empty text elements from an element and its children
    fn remove_empty_text_recursive(&self, element: &mut Element) {
        // Remove empty text elements from children
        element.children.retain(|child| {
            if let Node::Element(elem) = child {
                // Remove empty text element
                if self.config.text && elem.name == "text" && elem.children.is_empty() {
                    return false;
                }

                // Remove empty tspan element
                if self.config.tspan && elem.name == "tspan" && elem.children.is_empty() {
                    return false;
                }

                // Remove tref with empty xlink:href attribute
                if self.config.tref && elem.name == "tref" {
                    if let Some(href) = elem.attributes.get("xlink:href") {
                        if href.is_empty() {
                            return false;
                        }
                    } else {
                        // No xlink:href attribute at all
                        return false;
                    }
                }
            }
            true // Keep all other nodes
        });

        // Process child elements recursively
        for child in &mut element.children {
            if let Node::Element(elem) = child {
                self.remove_empty_text_recursive(elem);
            }
        }
    }
}

impl Default for RemoveEmptyTextPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveEmptyTextPlugin {
    fn name(&self) -> &'static str {
        "removeEmptyText"
    }

    fn description(&self) -> &'static str {
        "removes empty <text> elements"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Remove empty text elements from the document
        self.remove_empty_text_recursive(&mut document.root);
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

    fn count_elements_by_name(element: &Element, name: &str) -> usize {
        let mut count = 0;
        for child in &element.children {
            if let Node::Element(elem) = child {
                if elem.name == name {
                    count += 1;
                }
                count += count_elements_by_name(elem, name);
            }
        }
        count
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveEmptyTextPlugin::new();
        assert_eq!(plugin.name(), "removeEmptyText");
        assert_eq!(plugin.description(), "removes empty <text> elements");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveEmptyTextPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin.validate_params(&json!({"text": true})).is_ok());
        assert!(plugin
            .validate_params(&json!({"text": false, "tspan": true}))
            .is_ok());

        // Invalid parameters
        assert!(plugin.validate_params(&json!({"text": "invalid"})).is_err());
        assert!(plugin
            .validate_params(&json!({"unknownParam": true}))
            .is_err());
    }

    #[test]
    fn test_remove_empty_text() {
        let plugin = RemoveEmptyTextPlugin::new();
        let mut doc = Document::new();

        // Add empty text element
        let empty_text = create_element("text");
        doc.root.children.push(Node::Element(empty_text));

        // Add text element with content
        let mut text_with_content = create_element("text");
        text_with_content
            .children
            .push(Node::Text("Hello".to_string()));
        doc.root.children.push(Node::Element(text_with_content));

        // Add a regular element
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the empty text element
        assert_eq!(count_elements_by_name(&doc.root, "text"), 1);
        assert_eq!(doc.root.children.len(), 2);
    }

    #[test]
    fn test_remove_empty_tspan() {
        let plugin = RemoveEmptyTextPlugin::new();
        let mut doc = Document::new();

        // Add empty tspan element
        let empty_tspan = create_element("tspan");
        doc.root.children.push(Node::Element(empty_tspan));

        // Add tspan element with content
        let mut tspan_with_content = create_element("tspan");
        tspan_with_content
            .children
            .push(Node::Text("World".to_string()));
        doc.root.children.push(Node::Element(tspan_with_content));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the empty tspan element
        assert_eq!(count_elements_by_name(&doc.root, "tspan"), 1);
        assert_eq!(doc.root.children.len(), 1);
    }

    #[test]
    fn test_remove_tref_empty_href() {
        let plugin = RemoveEmptyTextPlugin::new();
        let mut doc = Document::new();

        // Add tref with empty xlink:href
        let mut tref_empty = create_element("tref");
        tref_empty.set_attr("xlink:href", "");
        doc.root.children.push(Node::Element(tref_empty));

        // Add tref with no xlink:href
        let tref_no_href = create_element("tref");
        doc.root.children.push(Node::Element(tref_no_href));

        // Add tref with valid xlink:href
        let mut tref_valid = create_element("tref");
        tref_valid.set_attr("xlink:href", "#validref");
        doc.root.children.push(Node::Element(tref_valid));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the tref elements with empty/missing href
        assert_eq!(count_elements_by_name(&doc.root, "tref"), 1);
        assert_eq!(doc.root.children.len(), 1);
    }

    #[test]
    fn test_config_text_disabled() {
        let config = RemoveEmptyTextConfig {
            text: false,
            tspan: true,
            tref: true,
        };
        let plugin = RemoveEmptyTextPlugin::with_config(config);
        let mut doc = Document::new();

        // Add empty text element
        let empty_text = create_element("text");
        doc.root.children.push(Node::Element(empty_text));

        // Add empty tspan element
        let empty_tspan = create_element("tspan");
        doc.root.children.push(Node::Element(empty_tspan));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have kept the text element but removed tspan
        assert_eq!(count_elements_by_name(&doc.root, "text"), 1);
        assert_eq!(count_elements_by_name(&doc.root, "tspan"), 0);
    }

    #[test]
    fn test_config_tspan_disabled() {
        let config = RemoveEmptyTextConfig {
            text: true,
            tspan: false,
            tref: true,
        };
        let plugin = RemoveEmptyTextPlugin::with_config(config);
        let mut doc = Document::new();

        // Add empty text element
        let empty_text = create_element("text");
        doc.root.children.push(Node::Element(empty_text));

        // Add empty tspan element
        let empty_tspan = create_element("tspan");
        doc.root.children.push(Node::Element(empty_tspan));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the text element but kept tspan
        assert_eq!(count_elements_by_name(&doc.root, "text"), 0);
        assert_eq!(count_elements_by_name(&doc.root, "tspan"), 1);
    }

    #[test]
    fn test_config_tref_disabled() {
        let config = RemoveEmptyTextConfig {
            text: true,
            tspan: true,
            tref: false,
        };
        let plugin = RemoveEmptyTextPlugin::with_config(config);
        let mut doc = Document::new();

        // Add tref with empty xlink:href
        let mut tref_empty = create_element("tref");
        tref_empty.set_attr("xlink:href", "");
        doc.root.children.push(Node::Element(tref_empty));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have kept the tref element
        assert_eq!(count_elements_by_name(&doc.root, "tref"), 1);
    }

    #[test]
    fn test_nested_empty_text() {
        let plugin = RemoveEmptyTextPlugin::new();
        let mut doc = Document::new();

        // Create a group with nested empty text elements
        let mut group = create_element("g");
        let empty_text = create_element("text");
        group.children.push(Node::Element(empty_text));

        // Add regular element
        let rect = create_element("rect");
        group.children.push(Node::Element(rect));

        doc.root.children.push(Node::Element(group));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the nested empty text element
        assert_eq!(count_elements_by_name(&doc.root, "text"), 0);
        assert_eq!(doc.root.children.len(), 1);

        // The group should still exist with only the rect
        if let Node::Element(group_elem) = &doc.root.children[0] {
            assert_eq!(group_elem.name, "g");
            assert_eq!(group_elem.children.len(), 1);
        }
    }

    #[test]
    fn test_no_empty_text_elements() {
        let plugin = RemoveEmptyTextPlugin::new();
        let mut doc = Document::new();

        // Add only non-empty text elements
        let mut text_with_content = create_element("text");
        text_with_content
            .children
            .push(Node::Text("Hello".to_string()));
        doc.root.children.push(Node::Element(text_with_content));

        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have no changes
        assert_eq!(count_elements_by_name(&doc.root, "text"), 1);
        assert_eq!(doc.root.children.len(), 2);
    }

    #[test]
    fn test_config_parsing() {
        let config = RemoveEmptyTextPlugin::parse_config(&json!({
            "text": false,
            "tspan": true,
            "tref": false
        }))
        .unwrap();

        assert_eq!(config.text, false);
        assert_eq!(config.tspan, true);
        assert_eq!(config.tref, false);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveEmptyTextPlugin, "removeEmptyText");
