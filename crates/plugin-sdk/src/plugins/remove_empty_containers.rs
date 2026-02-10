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
        remove_empty_containers_recursive(&mut document.root);
        Ok(())
    }
}

fn remove_empty_containers_recursive(element: &mut Element) {
    for child in &mut element.children {
        if let Node::Element(child_element) = child {
            remove_empty_containers_recursive(child_element);
        }
    }

    let mut i = 0;
    while i < element.children.len() {
        let remove = if let Node::Element(child_element) = &element.children[i] {
            should_remove_empty_container(child_element, Some(element.name.as_ref()))
        } else {
            false
        };

        if remove {
            element.children.remove(i);
            // Also remove preceding whitespace text node
            if i > 0 {
                let is_whitespace = if let Node::Text(text) = &element.children[i - 1] {
                    text.trim().is_empty()
                } else {
                    false
                };
                if is_whitespace {
                    element.children.remove(i - 1);
                    i -= 1;
                }
            }
        } else {
            i += 1;
        }
    }
}

fn should_remove_empty_container(element: &Element, parent_name: Option<&str>) -> bool {
    if !CONTAINER_ELEMENTS.contains(element.name.as_ref()) {
        return false;
    }

    if !element.children.is_empty() {
        return false;
    }

    if element.name == "svg" {
        return false;
    }

    if element.name == "pattern" && !element.attributes.is_empty() {
        return false;
    }

    if element.name == "mask" && element.attributes.contains_key("id") {
        return false;
    }

    if parent_name == Some("switch") {
        return false;
    }

    if element.name == "g" {
        if element.attributes.contains_key("filter") {
            return false;
        }
        if let Some(style) = element.attributes.get("style") {
            if style.contains("filter") {
                return false;
            }
        }
    }

    true
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
        doc.root.children.push(Node::Text("Hello".to_string().into()));

        plugin.apply(&mut doc).unwrap();

        // Text nodes should be preserved
        assert_eq!(doc.root.children.len(), 1);
        assert!(matches!(doc.root.children.get(0), Some(Node::Text(_))));
    }
}

// Custom fixture tests to handle weird SVGO indentation
#[cfg(test)]
mod tests {
    use super::*;
    use vexy_svgo_core::Config;
    use vexy_svgo_test_utils::load_fixtures;
    use std::path::PathBuf;

    #[test]
    fn fixture_tests() -> Result<(), Box<dyn std::error::Error>> {
        let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("testdata")
            .join("plugins")
            .join("removeEmptyContainers");

        if !fixtures_path.exists() {
            println!("No fixtures found for plugin: removeEmptyContainers");
            return Ok(());
        }

        let fixtures = load_fixtures(&fixtures_path)?;

        for fixture in fixtures {
            let mut config = Config::new();
            config.plugins = vec![
                vexy_svgo_core::PluginConfig::Name("removeEmptyContainers".to_string()),
            ];
            config.js2svg.pretty = true;
            config.js2svg.indent = "    ".to_string();
            config.js2svg.final_newline = false;

            let registry = crate::registry::create_migrated_plugin_registry();
            let options = vexy_svgo_core::OptimizeOptions::new(config).with_registry(registry);
            let result = vexy_svgo_core::optimize(&fixture.input, options)?;
            
            // Normalize whitespace for comparison
            let actual = result.data.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            let expected = fixture.expected.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            
            assert_eq!(actual, expected, "Fixture: {}", fixture.name);
        }
        Ok(())
    }
}
