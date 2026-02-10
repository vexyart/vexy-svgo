// this_file: crates/plugin-sdk/src/plugins/remove_metadata.rs

//! Remove metadata plugin implementation
//!
//! This plugin removes `<metadata>` elements from SVG documents.
//! Metadata elements are used to store document metadata but are not
//! needed for rendering and can be safely removed for optimization.
//!
//! Reference: https://www.w3.org/TR/SVG11/metadata.html

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration parameters for remove metadata plugin (currently empty)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMetadataConfig {
    // No configuration options - matches SVGO behavior
}

impl Default for RemoveMetadataConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin that removes metadata elements
pub struct RemoveMetadataPlugin {
    #[allow(dead_code)]
    config: RemoveMetadataConfig,
}

impl RemoveMetadataPlugin {
    /// Create a new RemoveMetadataPlugin
    pub fn new() -> Self {
        Self {
            #[allow(dead_code)]
            config: RemoveMetadataConfig::default(),
        }
    }

    /// Create a new RemoveMetadataPlugin with config
    pub fn with_config(config: RemoveMetadataConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn _parse_config(params: &Value) -> Result<RemoveMetadataConfig> {
        if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(RemoveMetadataConfig::default())
        }
    }

    /// Recursively remove metadata elements from an element and its children
    fn remove_metadata_recursive(&self, element: &mut Element) {
        // Remove metadata elements from children
        element.children.retain(|child| {
            if let Node::Element(elem) = child {
                elem.name != "metadata"
            } else {
                true // Keep non-element nodes
            }
        });

        // Process child elements recursively
        for child in &mut element.children {
            if let Node::Element(elem) = child {
                self.remove_metadata_recursive(elem);
            }
        }
    }
}

impl Default for RemoveMetadataPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveMetadataPlugin {
    fn name(&self) -> &'static str {
        "removeMetadata"
    }

    fn description(&self) -> &'static str {
        "removes <metadata>"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            if !obj.is_empty() {
                return Err(anyhow::anyhow!(
                    "removeMetadata plugin does not accept any parameters"
                ));
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Remove metadata elements from the document
        self.remove_metadata_recursive(&mut document.root);
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

    fn count_metadata_elements(element: &Element) -> usize {
        let mut count = 0;
        for child in &element.children {
            if let Node::Element(elem) = child {
                if elem.name == "metadata" {
                    count += 1;
                }
                count += count_metadata_elements(elem);
            }
        }
        count
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveMetadataPlugin::new();
        assert_eq!(plugin.name(), "removeMetadata");
        assert_eq!(plugin.description(), "removes <metadata>");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveMetadataPlugin::new();

        // Valid parameters (empty object)
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Invalid parameters (non-empty object)
        assert!(plugin.validate_params(&json!({"param": "value"})).is_err());
    }

    #[test]
    fn test_remove_metadata() {
        let plugin = RemoveMetadataPlugin::new();
        let mut doc = Document::new();

        // Add a metadata element
        let mut metadata = create_element("metadata");
        metadata
            .children
            .push(Node::Text("Some metadata content".to_string().into()));
        doc.root.children.push(Node::Element(metadata));

        // Add a regular element
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the metadata element
        assert_eq!(count_metadata_elements(&doc.root), 0);
        assert_eq!(doc.root.children.len(), 1);

        // The remaining element should be the rect
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_remove_multiple_metadata() {
        let plugin = RemoveMetadataPlugin::new();
        let mut doc = Document::new();

        // Add multiple metadata elements
        let metadata1 = create_element("metadata");
        let metadata2 = create_element("metadata");
        doc.root.children.push(Node::Element(metadata1));
        doc.root.children.push(Node::Element(metadata2));

        // Add a regular element
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed all metadata elements
        assert_eq!(count_metadata_elements(&doc.root), 0);
        assert_eq!(doc.root.children.len(), 1);

        // The remaining element should be the rect
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_remove_nested_metadata() {
        let plugin = RemoveMetadataPlugin::new();
        let mut doc = Document::new();

        // Create a group with nested metadata
        let mut group = create_element("g");
        let metadata = create_element("metadata");
        group.children.push(Node::Element(metadata));

        // Add regular elements
        let rect = create_element("rect");
        group.children.push(Node::Element(rect));

        doc.root.children.push(Node::Element(group));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the nested metadata element
        assert_eq!(count_metadata_elements(&doc.root), 0);
        assert_eq!(doc.root.children.len(), 1);

        // The group should still exist with only the rect
        if let Node::Element(group_elem) = &doc.root.children[0] {
            assert_eq!(group_elem.name, "g");
            assert_eq!(group_elem.children.len(), 1);

            if let Node::Element(rect_elem) = &group_elem.children[0] {
                assert_eq!(rect_elem.name, "rect");
            }
        }
    }

    #[test]
    fn test_no_metadata() {
        let plugin = RemoveMetadataPlugin::new();
        let mut doc = Document::new();

        // Add only regular elements
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have no changes
        assert_eq!(count_metadata_elements(&doc.root), 0);
        assert_eq!(doc.root.children.len(), 1);

        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_metadata_with_text() {
        let plugin = RemoveMetadataPlugin::new();
        let mut doc = Document::new();

        // Add metadata, text, and element
        let metadata = create_element("metadata");
        doc.root.children.push(Node::Element(metadata));
        doc.root.children.push(Node::Text("Some text".to_string().into()));
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed only the metadata element
        assert_eq!(count_metadata_elements(&doc.root), 0);
        assert_eq!(doc.root.children.len(), 2);
        assert!(matches!(doc.root.children[0], Node::Text(_)));
        assert!(matches!(doc.root.children[1], Node::Element(_)));
    }

    #[test]
    fn test_metadata_with_content() {
        let plugin = RemoveMetadataPlugin::new();
        let mut doc = Document::new();

        // Add metadata with content
        let mut metadata = create_element("metadata");
        metadata
            .children
            .push(Node::Text("Title: Test SVG".to_string().into()));
        let mut desc = create_element("desc");
        desc.children
            .push(Node::Text("Description content".to_string().into()));
        metadata.children.push(Node::Element(desc));

        doc.root.children.push(Node::Element(metadata));

        // Add a regular element
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the metadata element and all its content
        assert_eq!(count_metadata_elements(&doc.root), 0);
        assert_eq!(doc.root.children.len(), 1);

        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = RemoveMetadataPlugin::_parse_config(&json!({})).unwrap();
        // No fields to check since config is empty
        let _ = config;
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveMetadataPlugin, "removeMetadata");
