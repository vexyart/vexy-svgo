// this_file: crates/plugin-sdk/src/plugins/remove_unused_ns.rs

//! Plugin to remove unused namespace declarations
//!
//! This plugin removes unused namespace declarations from the root SVG element
//! which are not used in elements or attributes throughout the document.
//!
//! Reference: SVGO's removeUnusedNS plugin

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration for the removeUnusedNS plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveUnusedNSConfig {}

impl Default for RemoveUnusedNSConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin to remove unused namespace declarations
pub struct RemoveUnusedNSPlugin {
    #[allow(dead_code)]
    config: RemoveUnusedNSConfig,
}

impl RemoveUnusedNSPlugin {
    pub fn new() -> Self {
        Self {
            #[allow(dead_code)]
            config: RemoveUnusedNSConfig::default(),
        }
    }

    pub fn with_config(config: RemoveUnusedNSConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveUnusedNSConfig> {
        if params.is_null() {
            Ok(RemoveUnusedNSConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn check_usage(&self, element: &Element, unused_namespaces: &mut HashSet<String>) {
        // Check if element name uses a namespace
        if element.name.contains(':') {
            let parts: Vec<&str> = element.name.split(':').collect();
            if parts.len() >= 2 {
                let ns = parts[0];
                unused_namespaces.remove(ns);
            }
        }

        // Check if any attributes use namespaces
        for attr_name in element.attributes.keys() {
            if attr_name.contains(':') {
                let parts: Vec<&str> = attr_name.split(':').collect();
                if parts.len() >= 2 {
                    let ns = parts[0];
                    unused_namespaces.remove(ns);
                }
            }
        }

        // Recursively check children
        for child in &element.children {
            if let Node::Element(ref elem) = child {
                self.check_usage(elem, unused_namespaces);
            }
        }
    }
}

impl Default for RemoveUnusedNSPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveUnusedNSPlugin {
    fn name(&self) -> &'static str {
        "removeUnusedNS"
    }

    fn description(&self) -> &'static str {
        "removes unused namespaces declaration"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // First, collect all namespace declarations from the root SVG element
        let mut unused_namespaces = HashSet::new();

        // Collect xmlns: attributes from root element
        for attr_name in document.root.attributes.keys() {
            if attr_name.starts_with("xmlns:") {
                let local = attr_name.strip_prefix("xmlns:").unwrap();
                unused_namespaces.insert(local.to_string());
            }
        }

        // Traverse the document and remove used namespaces from the unused set
        self.check_usage(&document.root, &mut unused_namespaces);

        // Remove unused namespace declarations from root element
        for ns in &unused_namespaces {
            let xmlns_attr = format!("xmlns:{}", ns);
            document.root.remove_attr(&xmlns_attr);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use vexy_svgo_core::ast::{Document, DocumentMetadata, Element, Node};

    fn create_test_document() -> Document<'static> {
        Document {
            prologue: vec![],
            root: Element {
                name: "svg".into(),
                attributes: IndexMap::new(),
                children: vec![],
                namespaces: IndexMap::new(),
            },
            epilogue: vec![],
            metadata: DocumentMetadata::default(),
            memory_budget: None,
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveUnusedNSPlugin::new();
        assert_eq!(plugin.name(), "removeUnusedNS");
        assert_eq!(
            plugin.description(),
            "removes unused namespaces declaration"
        );
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveUnusedNSPlugin::new();

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
    fn test_remove_unused_namespace() {
        let mut document = create_test_document();

        // Add unused namespace
        document
            .root
            .set_attr("xmlns:unused", "http://example.com/unused");
        document
            .root
            .set_attr("xmlns:xlink", "http://www.w3.org/1999/xlink");

        // Add an element that uses xlink
        let mut rect_element = Element {
            name: "rect".into(),
            attributes: IndexMap::new(),
            children: vec![],
            namespaces: IndexMap::new(),
        };
        rect_element.set_attr("xlink:href", "#test");

        document.root.children = vec![Node::Element(rect_element)];

        let plugin = RemoveUnusedNSPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // unused namespace should be removed, xlink should remain
        assert!(!document.root.has_attr("xmlns:unused"));
        assert!(document.root.has_attr("xmlns:xlink"));
    }

    #[test]
    fn test_preserve_used_namespace_in_element_name() {
        let mut document = create_test_document();

        // Add namespace
        document
            .root
            .set_attr("xmlns:svg", "http://www.w3.org/2000/svg");

        // Add a child element with namespaced name
        let ns_element = Element {
            name: "svg:g".into(),
            attributes: IndexMap::new(),
            children: vec![],
            namespaces: IndexMap::new(),
        };

        document.root.children = vec![Node::Element(ns_element)];

        let plugin = RemoveUnusedNSPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // svg namespace should be preserved
        assert!(document.root.has_attr("xmlns:svg"));
    }

    #[test]
    fn test_preserve_used_namespace_in_attributes() {
        let mut document = create_test_document();

        // Add namespace
        document
            .root
            .set_attr("xmlns:custom", "http://example.com/custom");

        // Add an element with namespaced attribute
        let mut element = Element {
            name: "rect".into(),
            attributes: IndexMap::new(),
            children: vec![],
            namespaces: IndexMap::new(),
        };
        element.set_attr("custom:data", "value");

        document.root.children = vec![Node::Element(element)];

        let plugin = RemoveUnusedNSPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // custom namespace should be preserved
        assert!(document.root.has_attr("xmlns:custom"));
    }

    #[test]
    fn test_remove_all_unused_namespaces() {
        let mut document = create_test_document();

        // Add multiple unused namespaces
        document
            .root
            .set_attr("xmlns:ns1", "http://example.com/ns1");
        document
            .root
            .set_attr("xmlns:ns2", "http://example.com/ns2");
        document
            .root
            .set_attr("xmlns:ns3", "http://example.com/ns3");

        // Add an element without any namespace usage
        let element = Element {
            name: "rect".into(),
            attributes: IndexMap::new(),
            children: vec![],
            namespaces: IndexMap::new(),
        };

        document.root.children = vec![Node::Element(element)];

        let plugin = RemoveUnusedNSPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // All unused namespaces should be removed
        assert!(!document.root.has_attr("xmlns:ns1"));
        assert!(!document.root.has_attr("xmlns:ns2"));
        assert!(!document.root.has_attr("xmlns:ns3"));
    }

    #[test]
    fn test_no_namespaces_to_remove() {
        let mut document = create_test_document();

        // No xmlns: attributes
        document.root.set_attr("width", "100");
        document.root.set_attr("height", "100");

        let plugin = RemoveUnusedNSPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // Should still have original attributes
        assert_eq!(document.root.attr("width"), Some("100"));
        assert_eq!(document.root.attr("height"), Some("100"));
    }

    #[test]
    fn test_nested_element_namespace_usage() {
        let mut document = create_test_document();

        // Add namespace
        document
            .root
            .set_attr("xmlns:deep", "http://example.com/deep");

        // Create nested structure where namespace is used deep in the tree
        let mut deep_element = Element {
            name: "text".into(),
            attributes: IndexMap::new(),
            children: vec![],
            namespaces: IndexMap::new(),
        };
        deep_element.set_attr("deep:attr", "value");

        let middle_element = Element {
            name: "g".into(),
            attributes: IndexMap::new(),
            children: vec![Node::Element(deep_element)],
            namespaces: IndexMap::new(),
        };

        let container_element = Element {
            name: "g".into(),
            attributes: IndexMap::new(),
            children: vec![Node::Element(middle_element)],
            namespaces: IndexMap::new(),
        };

        document.root.children = vec![Node::Element(container_element)];

        let plugin = RemoveUnusedNSPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // deep namespace should be preserved (used in nested element)
        assert!(document.root.has_attr("xmlns:deep"));
    }

    #[test]
    fn test_mixed_used_and_unused_namespaces() {
        let mut document = create_test_document();

        // Add multiple namespaces
        document
            .root
            .set_attr("xmlns:used", "http://example.com/used");
        document
            .root
            .set_attr("xmlns:unused", "http://example.com/unused");
        document
            .root
            .set_attr("xmlns:alsounused", "http://example.com/alsounused");

        // Add an element that uses only one namespace
        let mut element = Element {
            name: "rect".into(),
            attributes: IndexMap::new(),
            children: vec![],
            namespaces: IndexMap::new(),
        };
        element.set_attr("used:data", "value");

        document.root.children = vec![Node::Element(element)];

        let plugin = RemoveUnusedNSPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // Only used namespace should remain
        assert!(document.root.has_attr("xmlns:used"));
        assert!(!document.root.has_attr("xmlns:unused"));
        assert!(!document.root.has_attr("xmlns:alsounused"));
    }
}
