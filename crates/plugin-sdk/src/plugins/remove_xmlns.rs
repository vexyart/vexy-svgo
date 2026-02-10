// this_file: crates/plugin-sdk/src/plugins/remove_xmlns.rs

//! Plugin to remove xmlns attribute from SVG elements
//!
//! This plugin removes the xmlns attribute when present, which is useful for inline SVG
//! where the namespace declaration is not needed. This plugin is disabled by default.
//!
//! Reference: SVGO's removeXMLNS plugin

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration for the removeXMLNS plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveXmlnsConfig {}

impl Default for RemoveXmlnsConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin to remove xmlns attribute from SVG elements
pub struct RemoveXmlnsPlugin {
    #[allow(dead_code)]
    config: RemoveXmlnsConfig,
}

impl RemoveXmlnsPlugin {
    pub fn new() -> Self {
        Self {
            #[allow(dead_code)]
            config: RemoveXmlnsConfig::default(),
        }
    }

    pub fn with_config(config: RemoveXmlnsConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveXmlnsConfig> {
        if params.is_null() {
            Ok(RemoveXmlnsConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn process_element(&self, element: &mut Element) {
        // Process children first
        let mut i = 0;
        while i < element.children.len() {
            if let Node::Element(child) = &mut element.children[i] {
                self.process_element(child);
            }
            i += 1;
        }

        // Remove xmlns attribute from SVG elements
        if element.name == "svg" {
            element.remove_attr("xmlns");
        }
    }
}

impl Default for RemoveXmlnsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveXmlnsPlugin {
    fn name(&self) -> &'static str {
        "removeXMLNS"
    }

    fn description(&self) -> &'static str {
        "removes xmlns attribute (for inline svg, disabled by default)"
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
    use super::*;
    use vexy_svgo_core::ast::Element;
    use std::borrow::Cow;

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveXmlnsPlugin::new();
        assert_eq!(plugin.name(), "removeXMLNS");
        assert_eq!(
            plugin.description(),
            "removes xmlns attribute (for inline svg, disabled by default)"
        );
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveXmlnsPlugin::new();

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
    fn test_remove_xmlns_from_svg() {
        let mut document = Document::default();
        document.root.name = Cow::Borrowed("svg");
        document
            .root
            .set_attr("xmlns", "http://www.w3.org/2000/svg");
        document.root.set_attr("viewBox", "0 0 100 100");

        let plugin = RemoveXmlnsPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // xmlns should be removed, other attributes preserved
        assert!(!document.root.has_attr("xmlns"));
        assert!(document.root.has_attr("viewBox"));
        assert_eq!(document.root.attr("viewBox").unwrap(), "0 0 100 100");
    }

    #[test]
    fn test_remove_xmlns_from_nested_svg() {
        let mut document = Document::default();
        document.root.name = Cow::Borrowed("svg");

        // Add nested SVG element with xmlns
        let mut nested_svg = Element::new("svg");
        nested_svg.set_attr("xmlns", "http://www.w3.org/2000/svg");
        nested_svg.set_attr("width", "50");

        document.root.children.push(Node::Element(nested_svg));

        let plugin = RemoveXmlnsPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // xmlns should be removed from nested SVG
        if let Node::Element(ref nested) = document.root.children[0] {
            assert_eq!(nested.name, "svg");
            assert!(!nested.has_attr("xmlns"));
            assert!(nested.has_attr("width"));
            assert_eq!(nested.attr("width").unwrap(), "50");
        } else {
            panic!("Expected nested svg element");
        }
    }

    #[test]
    fn test_preserve_other_xmlns_attributes() {
        let mut document = Document::default();
        document.root.name = Cow::Borrowed("svg");
        document
            .root
            .set_attr("xmlns", "http://www.w3.org/2000/svg");
        document
            .root
            .set_attr("xmlns:xlink", "http://www.w3.org/1999/xlink");
        document
            .root
            .set_attr("xmlns:custom", "http://example.com/custom");

        let plugin = RemoveXmlnsPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // Only xmlns should be removed, namespaced xmlns attributes preserved
        assert!(!document.root.has_attr("xmlns"));
        assert!(document.root.has_attr("xmlns:xlink"));
        assert!(document.root.has_attr("xmlns:custom"));
    }

    #[test]
    fn test_ignore_non_svg_elements() {
        let mut document = Document::default();
        document.root.name = Cow::Borrowed("svg");

        // Add a non-SVG element with xmlns (shouldn't happen but test anyway)
        let mut rect_element = Element::new("rect");
        rect_element.set_attr("xmlns", "http://www.w3.org/2000/svg");
        rect_element.set_attr("width", "100");

        document.root.children.push(Node::Element(rect_element));

        let plugin = RemoveXmlnsPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // xmlns should be preserved on non-SVG elements
        if let Node::Element(ref rect) = document.root.children[0] {
            assert_eq!(rect.name, "rect");
            assert!(rect.has_attr("xmlns"));
            assert_eq!(rect.attr("xmlns").unwrap(), "http://www.w3.org/2000/svg");
        } else {
            panic!("Expected rect element");
        }
    }

    #[test]
    fn test_no_xmlns_attribute() {
        let mut document = Document::default();
        document.root.name = Cow::Borrowed("svg");
        document.root.set_attr("viewBox", "0 0 100 100");
        document.root.set_attr("width", "100");

        let plugin = RemoveXmlnsPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // Should work fine even without xmlns attribute
        assert!(document.root.has_attr("viewBox"));
        assert!(document.root.has_attr("width"));
    }

    #[test]
    fn test_complex_nested_structure() {
        let mut document = Document::default();
        document.root.name = Cow::Borrowed("svg");
        document
            .root
            .set_attr("xmlns", "http://www.w3.org/2000/svg");

        // Nested structure: svg -> g -> svg
        let mut inner_svg = Element::new("svg");
        inner_svg.set_attr("xmlns", "http://www.w3.org/2000/svg");
        inner_svg.set_attr("x", "10");

        let mut g_element = Element::new("g");
        g_element.children.push(Node::Element(inner_svg));

        document.root.children.push(Node::Element(g_element));

        let plugin = RemoveXmlnsPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // Both root and nested SVG should have xmlns removed
        assert!(!document.root.has_attr("xmlns"));

        if let Node::Element(ref g) = document.root.children[0] {
            if let Node::Element(ref inner_svg) = g.children[0] {
                assert_eq!(inner_svg.name, "svg");
                assert!(!inner_svg.has_attr("xmlns"));
                assert_eq!(inner_svg.attr("x").unwrap(), "10");
            } else {
                panic!("Expected inner svg element");
            }
        } else {
            panic!("Expected g element");
        }
    }
}
