// this_file: crates/plugin-sdk/src/plugins/remove_empty_containers.rs

//! Remove empty containers plugin implementation
//!
//! This plugin removes empty container elements that have no children, with special
//! handling for certain cases where empty containers have semantic meaning.
//!
//! SVGO parameters supported:
//! - None (this plugin has no configuration parameters)

use crate::Plugin;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::LazyLock;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Set of SVG container elements that can be removed when empty
/// Based on https://www.w3.org/TR/SVG11/intro.html#TermContainerElement
static CONTAINER_ELEMENTS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "a",
        "defs",
        "foreignObject",
        "g",
        "marker",
        "mask",
        "missing-glyph",
        "pattern",
        "svg",
        "switch",
        "symbol",
    ]
    .into_iter()
    .collect()
});

/// Plugin that removes empty container elements
pub struct RemoveEmptyContainersPlugin;

impl RemoveEmptyContainersPlugin {
    /// Create a new RemoveEmptyContainersPlugin
    pub fn new() -> Self {
        Self
    }
}

impl Default for RemoveEmptyContainersPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveEmptyContainersPlugin {
    fn name(&self) -> &'static str {
        "removeEmptyContainers"
    }

    fn description(&self) -> &'static str {
        "Remove empty container elements"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        // This plugin has no parameters
        if let Some(obj) = params.as_object() {
            if !obj.is_empty() {
                return Err(anyhow::anyhow!(
                    "removeEmptyContainers plugin does not accept any parameters"
                ));
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Use a post-order visitor to process children before parents
        let mut visitor = EmptyContainerRemovalVisitor::new();
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that removes empty containers
struct EmptyContainerRemovalVisitor {
    parent_stack: Vec<String>,
}

impl EmptyContainerRemovalVisitor {
    fn new() -> Self {
        Self {
            parent_stack: Vec::new(),
        }
    }

    /// Determine if an empty container element should be removed
    fn should_remove_empty_container(&self, element: &Element) -> bool {
        // Only consider container elements
        if !CONTAINER_ELEMENTS.contains(element.name.as_ref()) {
            return false;
        }

        // Must be empty (no children)
        if !element.children.is_empty() {
            return false;
        }

        // Don't remove root SVG elements
        if element.name == "svg" {
            return false;
        }

        // Empty patterns may contain reusable configuration
        if element.name == "pattern" && !element.attributes.is_empty() {
            return false;
        }

        // Empty <mask> with ID hides masked element
        if element.name == "mask" && element.attributes.contains_key("id") {
            return false;
        }

        // Don't remove elements that are direct children of <switch>
        if let Some(parent) = self.parent_stack.last() {
            if parent == "switch" {
                return false;
            }
        }

        // The <g> may not have content, but the filter may cause a rectangle
        // to be created and filled with pattern
        if element.name == "g" && element.attributes.contains_key("filter") {
            return false;
        }

        // If we get here, it's safe to remove this empty container
        true
    }
}

impl Visitor<'_> for EmptyContainerRemovalVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Track parent hierarchy
        self.parent_stack.push(element.name.to_string());
        Ok(())
    }

    fn visit_element_exit(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Remove empty container children
        element.children.retain(|child| {
            if let Node::Element(child_element) = child {
                !self.should_remove_empty_container(child_element)
            } else {
                true // Keep non-element nodes (text, comments, etc.)
            }
        });

        // Pop from parent stack after processing children
        self.parent_stack.pop();

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
        let plugin = RemoveEmptyContainersPlugin::new();
        assert_eq!(plugin.name(), "removeEmptyContainers");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveEmptyContainersPlugin::new();

        // Valid: no parameters
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Invalid: any parameters
        assert!(plugin.validate_params(&json!({"something": true})).is_err());
    }

    #[test]
    fn test_removes_empty_defs() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add empty defs element
        let empty_defs = create_element("defs");
        doc.root.children.push(Node::Element(empty_defs));

        plugin.apply(&mut doc).unwrap();

        // Empty defs should be removed
        assert!(doc.root.children.is_empty());
    }

    #[test]
    fn test_removes_empty_g() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add empty g element
        let empty_g = create_element("g");
        doc.root.children.push(Node::Element(empty_g));

        plugin.apply(&mut doc).unwrap();

        // Empty g should be removed
        assert!(doc.root.children.is_empty());
    }

    #[test]
    fn test_preserves_svg_root() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Empty SVG root
        plugin.apply(&mut doc).unwrap();

        // SVG root should never be removed even if empty
        assert_eq!(doc.root.name, "svg");
    }

    #[test]
    fn test_preserves_pattern_with_attributes() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add empty pattern with attributes
        let mut pattern = create_element("pattern");
        pattern.set_attr("id", "mypattern");
        doc.root.children.push(Node::Element(pattern));

        plugin.apply(&mut doc).unwrap();

        // Pattern with attributes should be preserved
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(child)) = doc.root.children.get(0) {
            assert_eq!(child.name, "pattern");
        }
    }

    #[test]
    fn test_preserves_mask_with_id() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add empty mask with ID
        let mut mask = create_element("mask");
        mask.set_attr("id", "mymask");
        doc.root.children.push(Node::Element(mask));

        plugin.apply(&mut doc).unwrap();

        // Mask with ID should be preserved
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(child)) = doc.root.children.get(0) {
            assert_eq!(child.name, "mask");
        }
    }

    #[test]
    fn test_preserves_g_with_filter() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add empty g with filter
        let mut g = create_element("g");
        g.set_attr("filter", "url(#myfilter)");
        doc.root.children.push(Node::Element(g));

        plugin.apply(&mut doc).unwrap();

        // Group with filter should be preserved
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(child)) = doc.root.children.get(0) {
            assert_eq!(child.name, "g");
        }
    }

    #[test]
    fn test_preserves_elements_in_switch() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add switch with empty g child
        let mut switch = create_element("switch");
        let empty_g = create_element("g");
        switch.children.push(Node::Element(empty_g));
        doc.root.children.push(Node::Element(switch));

        plugin.apply(&mut doc).unwrap();

        // Elements inside switch should be preserved
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(switch_elem)) = doc.root.children.get(0) {
            assert_eq!(switch_elem.name, "switch");
            assert_eq!(switch_elem.children.len(), 1);
        }
    }

    #[test]
    fn test_preserves_non_container_elements() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add empty rect (not a container)
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Non-container elements should not be removed
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(child)) = doc.root.children.get(0) {
            assert_eq!(child.name, "rect");
        }
    }

    #[test]
    fn test_preserves_containers_with_children() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add g with child element
        let mut g = create_element("g");
        let rect = create_element("rect");
        g.children.push(Node::Element(rect));
        doc.root.children.push(Node::Element(g));

        plugin.apply(&mut doc).unwrap();

        // Container with children should be preserved
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(child)) = doc.root.children.get(0) {
            assert_eq!(child.name, "g");
            assert_eq!(child.children.len(), 1);
        }
    }

    #[test]
    fn test_nested_empty_containers() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Create nested empty containers
        let mut outer_g = create_element("g");
        let inner_g = create_element("g");
        outer_g.children.push(Node::Element(inner_g));
        doc.root.children.push(Node::Element(outer_g));

        plugin.apply(&mut doc).unwrap();

        // Both empty containers should be removed
        assert!(doc.root.children.is_empty());
    }

    #[test]
    fn test_preserves_text_nodes() {
        let plugin = RemoveEmptyContainersPlugin::new();
        let mut doc = Document::new();

        // Add text node
        doc.root.children.push(Node::Text("Hello".to_string()));

        plugin.apply(&mut doc).unwrap();

        // Text nodes should be preserved
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(doc.root.children.get(0), Some(Node::Text(_))));
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveEmptyContainersPlugin, "removeEmptyContainers");
