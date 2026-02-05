// this_file: crates/plugin-sdk/src/plugins/remove_useless_defs.rs

//! Remove useless definitions plugin implementation
//!
//! This plugin removes unused definitions from SVG documents, specifically targeting
//! <defs> elements and non-rendering elements that don't have IDs. It follows the
//! same logic as SVGO's removeUselessDefs plugin.

use crate::Plugin;
use anyhow::Result;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Plugin that removes useless definitions from SVG documents
#[derive(Default)]
pub struct RemoveUselessDefsPlugin;

impl RemoveUselessDefsPlugin {
    /// Create a new RemoveUselessDefsPlugin
    pub fn new() -> Self {
        Self
    }

    /// List of non-rendering elements that can be filtered
    fn non_rendering_elements() -> &'static [&'static str] {
        &[
            "clipPath",
            "filter",
            "linearGradient",
            "marker",
            "mask",
            "pattern",
            "radialGradient",
            "solidColor",
            "symbol",
        ]
    }

    /// Check if an element is a non-rendering element
    fn is_non_rendering_element(name: &str) -> bool {
        Self::non_rendering_elements().contains(&name)
    }

    /// Check if a node should be preserved
    /// Elements are preserved if they have an id attribute or are style elements
    fn should_preserve_node(node: &Node) -> bool {
        match node {
            Node::Element(element) => {
                // Always preserve style elements
                if element.name.as_ref() == "style" {
                    return true;
                }

                // Preserve elements with IDs
                element.attributes.contains_key("id")
            }
            _ => true, // Preserve non-element nodes (text, comments, etc.)
        }
    }

    /// Check if an element should be processed for filtering
    fn should_process_element(element: &Element) -> bool {
        element.name.as_ref() == "defs"
            || (Self::is_non_rendering_element(&element.name)
                && !element.attributes.contains_key("id"))
    }

    /// Collect useful nodes from children, flattening structure where possible
    fn collect_useful_nodes<'a>(children: &[Node<'a>]) -> Vec<Node<'a>> {
        let mut useful_nodes = Vec::new();

        for child in children {
            match child {
                Node::Element(element) => {
                    if Self::should_preserve_node(child) {
                        // Element should be preserved - add it directly
                        useful_nodes.push(child.clone());
                    } else if Self::should_process_element(element) {
                        // Element can be processed - flatten its children
                        let flattened = Self::collect_useful_nodes(&element.children);
                        useful_nodes.extend(flattened);
                    } else {
                        // Element doesn't have ID - check if it has useful children to flatten
                        let flattened = Self::collect_useful_nodes(&element.children);
                        useful_nodes.extend(flattened);
                    }
                }
                _ => {
                    // Preserve non-element nodes (text, comments, etc.)
                    useful_nodes.push(child.clone());
                }
            }
        }

        useful_nodes
    }
}

impl Plugin for RemoveUselessDefsPlugin {
    fn name(&self) -> &'static str {
        "removeUselessDefs"
    }

    fn description(&self) -> &'static str {
        "Remove useless definitions from SVG document"
    }

    fn validate_params(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        // This plugin doesn't accept any parameters in the original SVGO implementation
        if !params.is_null() && !params.as_object().map_or(false, |obj| obj.is_empty()) {
            return Err(anyhow::anyhow!(
                "removeUselessDefs plugin does not accept any parameters"
            ));
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let mut visitor = UselessDefsRemovalVisitor::new();
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that removes useless definitions
struct UselessDefsRemovalVisitor;

impl UselessDefsRemovalVisitor {
    fn new() -> Self {
        Self
    }
}

impl Visitor<'_> for UselessDefsRemovalVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Process elements that should be filtered
        if RemoveUselessDefsPlugin::should_process_element(element) {
            // Collect useful nodes from children, applying flattening
            element.children = RemoveUselessDefsPlugin::collect_useful_nodes(&element.children);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::borrow::Cow;
    use vexy_svgo_core::ast::{Document, Element, Node};

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    fn create_element_with_id(name: &'static str, id: &str) -> Element<'static> {
        let mut element = create_element(name);
        element.attributes.insert(Cow::Borrowed("id"), Cow::Owned(id.to_string()));
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveUselessDefsPlugin::new();
        assert_eq!(plugin.name(), "removeUselessDefs");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveUselessDefsPlugin::new();

        // Valid: null parameters
        assert!(plugin.validate_params(&json!(null)).is_ok());

        // Valid: empty object
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Invalid: non-empty parameters
        assert!(plugin.validate_params(&json!({"someParam": true})).is_err());
    }

    #[test]
    fn test_non_rendering_elements() {
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element(
            "clipPath"
        ));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element("filter"));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element(
            "linearGradient"
        ));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element("marker"));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element("mask"));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element("pattern"));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element(
            "radialGradient"
        ));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element(
            "solidColor"
        ));
        assert!(RemoveUselessDefsPlugin::is_non_rendering_element("symbol"));

        // Not non-rendering elements
        assert!(!RemoveUselessDefsPlugin::is_non_rendering_element("rect"));
        assert!(!RemoveUselessDefsPlugin::is_non_rendering_element("path"));
        assert!(!RemoveUselessDefsPlugin::is_non_rendering_element("defs"));
    }

    #[test]
    fn test_should_preserve_node() {
        // Elements with IDs should be preserved
        let element_with_id = create_element_with_id("path", "mypath");
        assert!(RemoveUselessDefsPlugin::should_preserve_node(
            &Node::Element(element_with_id)
        ));

        // Style elements should always be preserved
        let style_element = create_element("style");
        assert!(RemoveUselessDefsPlugin::should_preserve_node(
            &Node::Element(style_element)
        ));

        // Elements without IDs should not be preserved (unless they're style)
        let element_without_id = create_element("path");
        assert!(!RemoveUselessDefsPlugin::should_preserve_node(
            &Node::Element(element_without_id)
        ));

        // Non-element nodes should be preserved
        assert!(RemoveUselessDefsPlugin::should_preserve_node(
            &Node::Comment("comment".to_string())
        ));
    }

    #[test]
    fn test_should_process_element() {
        // defs elements should be processed
        let defs_element = create_element("defs");
        assert!(RemoveUselessDefsPlugin::should_process_element(
            &defs_element
        ));

        // Non-rendering elements without IDs should be processed
        let clippath_no_id = create_element("clipPath");
        assert!(RemoveUselessDefsPlugin::should_process_element(
            &clippath_no_id
        ));

        // Non-rendering elements with IDs should not be processed
        let clippath_with_id = create_element_with_id("clipPath", "clip1");
        assert!(!RemoveUselessDefsPlugin::should_process_element(
            &clippath_with_id
        ));

        // Regular elements should not be processed
        let rect_element = create_element("rect");
        assert!(!RemoveUselessDefsPlugin::should_process_element(
            &rect_element
        ));
    }

    #[test]
    fn test_collect_useful_nodes_basic() {
        // Create test children: one with ID, one without
        let path_with_id = create_element_with_id("path", "path1");
        let path_without_id = create_element("path");

        let children = vec![
            Node::Element(path_with_id.clone()),
            Node::Element(path_without_id),
        ];

        let useful = RemoveUselessDefsPlugin::collect_useful_nodes(&children);

        // Only the element with ID should be preserved
        assert_eq!(useful.len(), 1);
        if let Node::Element(element) = &useful[0] {
            assert!(element.attributes.contains_key("id"));
        }
    }

    #[test]
    fn test_collect_useful_nodes_style_preservation() {
        let style_element = create_element("style");
        let path_without_id = create_element("path");

        let children = vec![
            Node::Element(style_element.clone()),
            Node::Element(path_without_id),
        ];

        let useful = RemoveUselessDefsPlugin::collect_useful_nodes(&children);

        // Style element should be preserved even without ID
        assert_eq!(useful.len(), 1);
        if let Node::Element(element) = &useful[0] {
            assert_eq!(element.name.as_ref(), "style");
        }
    }

    #[test]
    fn test_collect_useful_nodes_flattening() {
        // Test flattening within a defs context via the full plugin
        let plugin = RemoveUselessDefsPlugin::new();
        let mut doc = Document::new();

        // Create defs with nested structure: defs > g > path (with ID)
        let path_with_id = create_element_with_id("path", "path1");
        let mut group_no_id = create_element("g");
        group_no_id
            .children
            .push(Node::Element(path_with_id.clone()));

        let mut defs = create_element("defs");
        defs.children.push(Node::Element(group_no_id));

        doc.root.children.push(Node::Element(defs));

        // Apply the plugin
        plugin.apply(&mut doc).unwrap();

        // Check that flattening occurred within defs
        if let Some(Node::Element(defs_element)) = doc.root.children.first() {
            assert_eq!(defs_element.name.as_ref(), "defs");
            // Group should be removed, path should be flattened up
            assert_eq!(defs_element.children.len(), 1);

            if let Node::Element(child) = &defs_element.children[0] {
                assert_eq!(child.name.as_ref(), "path");
                assert!(child.attributes.contains_key("id"));
            }
        }
    }

    #[test]
    fn test_plugin_apply() {
        let plugin = RemoveUselessDefsPlugin::new();
        let mut doc = Document::new();

        // Create defs with mixed content
        let mut defs = create_element("defs");
        defs.children.push(Node::Element(create_element("path"))); // Should be removed
        defs.children
            .push(Node::Element(create_element_with_id("path", "keep"))); // Should be kept
        defs.children.push(Node::Element(create_element("style"))); // Should be kept

        doc.root.children.push(Node::Element(defs));

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that defs was processed
        if let Some(Node::Element(defs_element)) = doc.root.children.first() {
            assert_eq!(defs_element.name.as_ref(), "defs");
            // Should have 2 children now (path with ID and style)
            assert_eq!(defs_element.children.len(), 2);

            // Verify preserved elements
            let has_path_with_id = defs_element.children.iter().any(|child| {
                if let Node::Element(elem) = child {
                    elem.name.as_ref() == "path" && elem.attributes.contains_key("id")
                } else {
                    false
                }
            });

            let has_style = defs_element.children.iter().any(|child| {
                if let Node::Element(elem) = child {
                    elem.name.as_ref() == "style"
                } else {
                    false
                }
            });

            assert!(has_path_with_id);
            assert!(has_style);
        } else {
            panic!("Expected defs element not found");
        }
    }

    #[test]
    fn test_non_rendering_element_without_id() {
        let plugin = RemoveUselessDefsPlugin::new();
        let mut doc = Document::new();

        // Create clipPath without ID containing useful content
        let mut clippath = create_element("clipPath");
        clippath
            .children
            .push(Node::Element(create_element("path"))); // Should be removed
        clippath
            .children
            .push(Node::Element(create_element_with_id("circle", "keep"))); // Should be kept

        doc.root.children.push(Node::Element(clippath));

        plugin.apply(&mut doc).unwrap();

        // clipPath without ID should be processed, keeping only useful children
        if let Some(Node::Element(clippath_element)) = doc.root.children.first() {
            assert_eq!(clippath_element.name.as_ref(), "clipPath");
            assert_eq!(clippath_element.children.len(), 1);

            if let Node::Element(child) = &clippath_element.children[0] {
                assert_eq!(child.name.as_ref(), "circle");
                assert!(child.attributes.contains_key("id"));
            }
        }
    }
}
