// this_file: crates/plugin-sdk/src/plugins/remove_xml_proc_inst.rs

//! Remove XML Processing Instruction plugin implementation
//!
//! This plugin removes XML processing instructions from SVG documents.
//! Processing instructions like <?xml version="1.0" encoding="utf-8"?> are
//! not needed in SVG documents and can be safely removed.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Node};

use crate::Plugin;

/// Configuration parameters for remove XML processing instruction plugin (currently empty)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveXMLProcInstConfig {
    // No configuration options - matches SVGO behavior
}

impl Default for RemoveXMLProcInstConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin that removes XML processing instructions
pub struct RemoveXMLProcInstPlugin {
    #[allow(dead_code)]
    config: RemoveXMLProcInstConfig,
}

impl RemoveXMLProcInstPlugin {
    fn is_formatting_whitespace(text: &str) -> bool {
        text.trim().is_empty() && text.chars().any(|c| matches!(c, '\n' | '\r' | '\t'))
    }

    fn cleanup_formatting_whitespace(nodes: &mut Vec<Node>) {
        let has_non_whitespace_content = nodes.iter().any(|node| match node {
            Node::Element(_) => true,
            Node::Text(text) => !text.trim().is_empty(),
            Node::Comment(_) | Node::ProcessingInstruction { .. } | Node::DocType(_) | Node::CData(_) => true,
        });

        if has_non_whitespace_content {
            nodes.retain(|node| !matches!(node, Node::Text(text) if Self::is_formatting_whitespace(text)));
        }
    }

    fn cleanup_formatting_whitespace_recursive(element: &mut vexy_svgo_core::ast::Element<'_>) {
        let preserve = matches!(element.name.as_ref(), "text" | "tspan" | "tref" | "textPath" | "altGlyph");
        for child in &mut element.children {
            if let Node::Text(text) = child {
                if !preserve && (text.contains('\n') || text.contains('\r') || text.contains('\t')) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        *text = trimmed.to_string().into();
                    }
                }
            }
        }
        Self::cleanup_formatting_whitespace(&mut element.children);
        for child in &mut element.children {
            if let Node::Element(child_element) = child {
                Self::cleanup_formatting_whitespace_recursive(child_element);
            }
        }
    }

    /// Create a new RemoveXMLProcInstPlugin
    pub fn new() -> Self {
        Self {
            #[allow(dead_code)]
            config:                     RemoveXMLProcInstConfig::default(),
        }
    }

    /// Create a new RemoveXMLProcInstPlugin with config
    pub fn with_config(config: RemoveXMLProcInstConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn _parse_config(params: &Value) -> Result<RemoveXMLProcInstConfig> {
        if params.is_object() {
            serde_json::from_value(params.clone()).map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(RemoveXMLProcInstConfig::default())
        }
    }
}

impl Default for RemoveXMLProcInstPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveXMLProcInstPlugin {
    fn name(&self) -> &'static str {
        "removeXMLProcInst"
    }

    fn description(&self) -> &'static str {
        "removes XML processing instructions"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            if !obj.is_empty() {
                return Err(anyhow::anyhow!("removeXMLProcInst plugin does not accept any parameters"));
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        document.metadata.version = None;
        document.metadata.encoding = None;

        // Remove all XML processing instruction nodes from the document prologue
        document.prologue.retain(|child| {
            // Remove ProcessingInstruction nodes with target "xml"
            if let Node::ProcessingInstruction { target, .. } = child {
                // Check if it's an XML processing instruction
                if target.eq_ignore_ascii_case("xml") {
                    return false; // Remove this node
                }
            }
            true // Keep all other nodes
        });
        Self::cleanup_formatting_whitespace(&mut document.prologue);

        // Also remove XML processing instructions from root children (in case they were misplaced)
        document.root.children.retain(|child| {
            // Remove ProcessingInstruction nodes with target "xml"
            if let Node::ProcessingInstruction { target, .. } = child {
                // Check if it's an XML processing instruction
                if target.eq_ignore_ascii_case("xml") {
                    return false; // Remove this node
                }
            }
            true // Keep all other nodes
        });
        Self::cleanup_formatting_whitespace_recursive(&mut document.root);
        Self::cleanup_formatting_whitespace(&mut document.epilogue);

        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use std::borrow::Cow;

    use serde_json::json;
    use vexy_svgo_core::ast::{Document, Element, Node};

    use super::*;

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveXMLProcInstPlugin::new();
        assert_eq!(plugin.name(), "removeXMLProcInst");
        assert_eq!(plugin.description(), "removes XML processing instructions");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveXMLProcInstPlugin::new();

        // Valid parameters (empty object)
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Invalid parameters (non-empty object)
        assert!(plugin.validate_params(&json!({"param": "value"})).is_err());
    }

    #[test]
    fn test_remove_xml_proc_inst() {
        let plugin = RemoveXMLProcInstPlugin::new();
        let mut doc = Document::new();

        // Add an XML processing instruction
        doc.root.children.push(Node::ProcessingInstruction {
            target: "xml".into(),
            data:   "version=\"1.0\" encoding=\"UTF-8\"".to_string().into(),
        });

        // Add a regular element
        let svg = create_element("svg");
        doc.root.children.push(Node::Element(svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the processing instruction
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(doc.root.children[0], Node::Element(_)));
    }

    #[test]
    fn test_remove_xml_proc_inst_simple() {
        let plugin = RemoveXMLProcInstPlugin::new();
        let mut doc = Document::new();

        // Add a simple XML processing instruction
        doc.root.children.push(Node::ProcessingInstruction {
            target: "xml".into(),
            data:   "".to_string().into(),
        });

        // Add a regular element
        let svg = create_element("svg");
        doc.root.children.push(Node::Element(svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed the processing instruction
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(doc.root.children[0], Node::Element(_)));
    }

    #[test]
    fn test_keep_other_processing_instructions() {
        let plugin = RemoveXMLProcInstPlugin::new();
        let mut doc = Document::new();

        // Add XML processing instruction (should be removed)
        doc.root.children.push(Node::ProcessingInstruction {
            target: "xml".into(),
            data:   "version=\"1.0\"".to_string().into(),
        });

        // Add other processing instruction (should be kept)
        doc.root.children.push(Node::ProcessingInstruction {
            target: "stylesheet".into(),
            data:   "type=\"text/css\"".to_string().into(),
        });

        // Add a regular element
        let svg = create_element("svg");
        doc.root.children.push(Node::Element(svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed only the XML processing instruction
        assert_eq!(doc.root.children.len(), 2);
        assert!(matches!(doc.root.children[0], Node::ProcessingInstruction { .. }));
        assert!(matches!(doc.root.children[1], Node::Element(_)));

        // Check that the remaining processing instruction is not XML
        if let Node::ProcessingInstruction { target, .. } = &doc.root.children[0] {
            assert_eq!(target.as_ref(), "stylesheet");
        }
    }

    #[test]
    fn test_multiple_xml_proc_inst() {
        let plugin = RemoveXMLProcInstPlugin::new();
        let mut doc = Document::new();

        // Add multiple XML processing instructions
        doc.root.children.push(Node::ProcessingInstruction {
            target: "xml".into(),
            data:   "version=\"1.0\"".to_string().into(),
        });
        doc.root.children.push(Node::ProcessingInstruction {
            target: "xml".into(),
            data:   "encoding=\"UTF-8\"".to_string().into(),
        });

        // Add a regular element
        let svg = create_element("svg");
        doc.root.children.push(Node::Element(svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed all XML processing instructions
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(doc.root.children[0], Node::Element(_)));
    }

    #[test]
    fn test_no_processing_instructions() {
        let plugin = RemoveXMLProcInstPlugin::new();
        let mut doc = Document::new();

        // Add only regular elements
        let svg = create_element("svg");
        doc.root.children.push(Node::Element(svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have no changes
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(doc.root.children[0], Node::Element(_)));
    }

    #[test]
    fn test_proc_inst_with_text() {
        let plugin = RemoveXMLProcInstPlugin::new();
        let mut doc = Document::new();

        // Add XML processing instruction, text, and element
        doc.root.children.push(Node::ProcessingInstruction {
            target: "xml".into(),
            data:   "version=\"1.0\"".to_string().into(),
        });
        doc.root.children.push(Node::Text("Some text".to_string().into()));
        let svg = create_element("svg");
        doc.root.children.push(Node::Element(svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Should have removed only the XML processing instruction
        assert_eq!(doc.root.children.len(), 2);
        assert!(matches!(doc.root.children[0], Node::Text(_)));
        assert!(matches!(doc.root.children[1], Node::Element(_)));
    }

    #[test]
    fn test_config_parsing() {
        let config = RemoveXMLProcInstPlugin::_parse_config(&json!({})).unwrap();
        // No fields to check since config is empty
        let _ = config;
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveXMLProcInstPlugin, "removeXMLProcInst");
