// this_file: crates/plugin-sdk/src/plugins/remove_comments.rs

//! Remove comments plugin implementation
//!
//! This plugin removes comments from SVG documents while optionally preserving
//! legal comments (those starting with '!') if configured.
//!
//! Reference: SVGO's removeComments plugin

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration for the remove comments plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveCommentsConfig {
    /// Whether to preserve legal comments (those starting with '!')
    #[serde(default = "default_preserve_patterns")]
    pub preserve_patterns: bool,
}

fn default_preserve_patterns() -> bool {
    true
}

impl Default for RemoveCommentsConfig {
    fn default() -> Self {
        Self {
            preserve_patterns: default_preserve_patterns(),
        }
    }
}

/// Plugin that removes comments from SVG documents
#[derive(Clone)]
pub struct RemoveCommentsPlugin {
    config: RemoveCommentsConfig,
}

impl RemoveCommentsPlugin {
    fn is_formatting_whitespace(text: &str) -> bool {
        text.trim().is_empty() && text.chars().any(|c| matches!(c, '\n' | '\r' | '\t'))
    }

    fn cleanup_formatting_whitespace(nodes: &mut Vec<Node>) {
        let has_non_whitespace_content = nodes.iter().any(|node| match node {
            Node::Element(_) => true,
            Node::Text(text) => !text.trim().is_empty(),
            _ => false,
        });

        if has_non_whitespace_content {
            nodes.retain(|node| {
                !matches!(node, Node::Text(text) if Self::is_formatting_whitespace(text))
            });
        }
    }

    fn should_preserve_text_whitespace(element_name: &str) -> bool {
        matches!(
            element_name,
            "text" | "tspan" | "tref" | "textPath" | "altGlyph"
        )
    }

    fn normalize_text_children(element: &mut Element<'_>) {
        let preserve = Self::should_preserve_text_whitespace(element.name.as_ref());
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
    }

    /// Create a new RemoveCommentsPlugin
    pub fn new() -> Self {
        Self {
            config: RemoveCommentsConfig::default(),
        }
    }

    /// Create plugin with specific configuration
    pub fn with_config(config: RemoveCommentsConfig) -> Self {
        Self { config }
    }

    /// Create plugin with preserve_patterns setting
    pub fn with_preserve_patterns(preserve_patterns: bool) -> Self {
        Self {
            config: RemoveCommentsConfig { preserve_patterns },
        }
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<RemoveCommentsConfig> {
        if params.is_null() {
            Ok(RemoveCommentsConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    /// Check if a comment should be preserved (legal comments start with '!')
    fn should_keep_comment(&self, comment: &str) -> bool {
        self.config.preserve_patterns && comment.trim_start().starts_with('!')
    }

    /// Remove comments from a list of nodes
    fn remove_comments_from_nodes(&self, nodes: &mut Vec<Node>) {
        nodes.retain(|node| match node {
            Node::Comment(comment) => self.should_keep_comment(comment),
            _ => true,
        });
        Self::cleanup_formatting_whitespace(nodes);
    }

    /// Recursively remove comments from an element and its children
    fn remove_comments_recursive(&self, element: &mut Element) {
        // Remove comment nodes from children
        self.remove_comments_from_nodes(&mut element.children);
        Self::normalize_text_children(element);

        // Recursively process child elements
        for child in &mut element.children {
            if let Node::Element(elem) = child {
                self.remove_comments_recursive(elem);
            }
        }
    }
}

impl Default for RemoveCommentsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveCommentsPlugin {
    fn name(&self) -> &'static str {
        "removeComments"
    }

    fn description(&self) -> &'static str {
        "removes comments"
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
        // Remove comments from prologue
        self.remove_comments_from_nodes(&mut document.prologue);

        // Remove comments from the main document tree
        self.remove_comments_recursive(&mut document.root);

        // Remove comments from epilogue
        self.remove_comments_from_nodes(&mut document.epilogue);

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
    fn test_plugin_info() {
        let plugin = RemoveCommentsPlugin::new();
        assert_eq!(plugin.name(), "removeComments");
        assert_eq!(plugin.description(), "removes comments");
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveCommentsPlugin::new();
        assert!(plugin.config.preserve_patterns);

        let config = RemoveCommentsConfig {
            preserve_patterns: false,
        };
        let plugin2 = RemoveCommentsPlugin::with_config(config);
        assert!(!plugin2.config.preserve_patterns);
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveCommentsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!(null)).is_ok());
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({"preservePatterns": true}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"preservePatterns": false}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"preservePatterns": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"preservePatterns": 123}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"unknownParam": true}))
            .is_err());
    }

    #[test]
    fn test_comment_preservation_logic() {
        let plugin = RemoveCommentsPlugin::new();

        // By default, legal comments are preserved
        assert!(plugin.should_keep_comment("! Legal comment"));
        assert!(plugin.should_keep_comment("!Important notice"));
        assert!(!plugin.should_keep_comment(" Regular comment"));
        assert!(!plugin.should_keep_comment("Just a comment"));

        // Test with preserve_patterns disabled
        let config = RemoveCommentsConfig {
            preserve_patterns: false,
        };
        let plugin2 = RemoveCommentsPlugin::with_config(config);
        assert!(!plugin2.should_keep_comment("! Legal comment"));
        assert!(!plugin2.should_keep_comment("Regular comment"));
    }

    #[test]
    fn test_removes_regular_comments() {
        let config = RemoveCommentsConfig {
            preserve_patterns: true,
        };
        let plugin = RemoveCommentsPlugin::with_config(config);
        let mut doc = Document::new();

        // Create SVG with comments
        let mut svg = create_element("svg");
        svg.children
            .push(Node::Comment("Regular comment".to_string().into()));
        svg.children.push(Node::Element(create_element("rect")));
        svg.children
            .push(Node::Comment("Another comment".to_string().into()));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that regular comments were removed
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_preserves_legal_comments() {
        let config = RemoveCommentsConfig {
            preserve_patterns: true,
        };
        let plugin = RemoveCommentsPlugin::with_config(config);
        let mut doc = Document::new();

        // Create SVG with legal and regular comments
        let mut svg = create_element("svg");
        svg.children
            .push(Node::Comment("! Legal comment".to_string().into()));
        svg.children
            .push(Node::Comment("Regular comment".to_string().into()));
        svg.children.push(Node::Element(create_element("rect")));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that legal comment was preserved
        assert_eq!(doc.root.children.len(), 2);
        if let Node::Comment(comment) = &doc.root.children[0] {
            assert_eq!(comment.as_ref(), "! Legal comment");
        }
        if let Node::Element(elem) = &doc.root.children[1] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_removes_all_comments_when_disabled() {
        let config = RemoveCommentsConfig {
            preserve_patterns: false,
        };
        let plugin = RemoveCommentsPlugin::with_config(config);
        let mut doc = Document::new();

        // Create SVG with legal and regular comments
        let mut svg = create_element("svg");
        svg.children
            .push(Node::Comment("! Legal comment".to_string().into()));
        svg.children
            .push(Node::Comment("Regular comment".to_string().into()));
        svg.children.push(Node::Element(create_element("rect")));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all comments were removed
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_removes_nested_comments() {
        let plugin = RemoveCommentsPlugin::new();
        let mut doc = Document::new();

        // Create SVG with nested comments
        let mut svg = create_element("svg");
        svg.children.push(Node::Comment("Root comment".to_string().into()));

        let mut group = create_element("g");
        group
            .children
            .push(Node::Comment("Group comment".to_string().into()));
        group.children.push(Node::Element(create_element("rect")));

        svg.children.push(Node::Element(group));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that nested comments were removed
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
    fn test_removes_prologue_and_epilogue_comments() {
        let plugin = RemoveCommentsPlugin::new();
        let mut doc = Document::new();

        // Add comments to prologue and epilogue
        doc.prologue
            .push(Node::Comment("Prologue comment".to_string().into()));
        doc.prologue
            .push(Node::Comment("! Legal prologue".to_string().into()));

        doc.epilogue
            .push(Node::Comment("Epilogue comment".to_string().into()));
        doc.epilogue
            .push(Node::Comment("! Legal epilogue".to_string().into()));

        let svg = create_element("svg");
        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that only legal comments remain
        assert_eq!(doc.prologue.len(), 1);
        if let Node::Comment(comment) = &doc.prologue[0] {
            assert_eq!(comment.as_ref(), "! Legal prologue");
        }

        assert_eq!(doc.epilogue.len(), 1);
        if let Node::Comment(comment) = &doc.epilogue[0] {
            assert_eq!(comment.as_ref(), "! Legal epilogue");
        }
    }

    #[test]
    fn test_cleans_whitespace_only_text() {
        let plugin = RemoveCommentsPlugin::new();
        let mut doc = Document::new();

        // Create SVG with comments and whitespace
        let mut svg = create_element("svg");
        svg.children.push(Node::Text("\n    ".to_string().into()));
        svg.children.push(Node::Comment("Comment".to_string().into()));
        svg.children.push(Node::Text("\n    ".to_string().into()));
        svg.children.push(Node::Element(create_element("rect")));
        svg.children.push(Node::Text("\n".to_string().into()));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that whitespace-only text nodes were cleaned up
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        }
    }

    #[test]
    fn test_preserves_meaningful_text() {
        let plugin = RemoveCommentsPlugin::new();
        let mut doc = Document::new();

        // Create SVG with meaningful text
        let mut svg = create_element("svg");
        let mut text_elem = create_element("text");
        text_elem.children.push(Node::Text("Hello".to_string().into()));
        text_elem
            .children
            .push(Node::Comment("Comment".to_string().into()));
        text_elem.children.push(Node::Text(" World".to_string().into()));

        svg.children.push(Node::Element(text_elem));

        doc.root = svg;

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that meaningful text was preserved
        if let Node::Element(text_elem) = &doc.root.children[0] {
            assert_eq!(text_elem.name, "text");
            assert_eq!(text_elem.children.len(), 2);
            if let Node::Text(text) = &text_elem.children[0] {
                assert_eq!(text.as_ref(), "Hello");
            }
            if let Node::Text(text) = &text_elem.children[1] {
                assert_eq!(text.as_ref(), " World");
            }
        }
    }

    #[test]
    fn test_config_parsing() {
        // Default config
        let config = RemoveCommentsPlugin::parse_config(&json!(null)).unwrap();
        assert!(config.preserve_patterns);

        // Explicit config
        let config =
            RemoveCommentsPlugin::parse_config(&json!({"preservePatterns": false})).unwrap();
        assert!(!config.preserve_patterns);

        let config =
            RemoveCommentsPlugin::parse_config(&json!({"preservePatterns": true})).unwrap();
        assert!(config.preserve_patterns);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests_with_params!(RemoveCommentsPlugin, "removeComments");
