// this_file: crates/plugin-sdk/src/plugins/remove_scripts.rs

//! Remove scripts plugin implementation
//!
//! This plugin removes all script elements and event attributes from SVG documents
//! to improve security. It removes:
//! - All <script> elements
//! - All event handler attributes (onclick, onload, etc.)
//! - JavaScript URLs in href attributes
//!
//! Reference: SVGO's removeScripts plugin

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration parameters for remove scripts plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveScriptsConfig {
    // No configuration options for this plugin
}

impl Default for RemoveScriptsConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin that removes scripts and event attributes
pub struct RemoveScriptsPlugin {
    config: RemoveScriptsConfig,
    event_attrs: HashSet<&'static str>,
}

impl RemoveScriptsPlugin {
    /// Create a new RemoveScriptsPlugin
    pub fn new() -> Self {
        // Initialize with all event attributes from SVGO's collections
        let mut event_attrs = HashSet::new();

        // animationEvent
        event_attrs.extend(&["onbegin", "onend", "onrepeat", "onload"]);

        // documentEvent
        event_attrs.extend(&[
            "onabort", "onerror", "onresize", "onscroll", "onunload", "onzoom",
        ]);

        // documentElementEvent
        event_attrs.extend(&["oncopy", "oncut", "onpaste"]);

        // globalEvent
        event_attrs.extend(&[
            "oncancel",
            "oncanplay",
            "oncanplaythrough",
            "onchange",
            "onclick",
            "onclose",
            "oncuechange",
            "ondblclick",
            "ondrag",
            "ondragend",
            "ondragenter",
            "ondragleave",
            "ondragover",
            "ondragstart",
            "ondrop",
            "ondurationchange",
            "onemptied",
            "onended",
            "onerror",
            "onfocus",
            "oninput",
            "oninvalid",
            "onkeydown",
            "onkeypress",
            "onkeyup",
            "onload",
            "onloadeddata",
            "onloadedmetadata",
            "onloadstart",
            "onmousedown",
            "onmouseenter",
            "onmouseleave",
            "onmousemove",
            "onmouseout",
            "onmouseover",
            "onmouseup",
            "onmousewheel",
            "onpause",
            "onplay",
            "onplaying",
            "onprogress",
            "onratechange",
            "onreset",
            "onresize",
            "onscroll",
            "onseeked",
            "onseeking",
            "onselect",
            "onshow",
            "onstalled",
            "onsubmit",
            "onsuspend",
            "ontimeupdate",
            "ontoggle",
            "onvolumechange",
            "onwaiting",
        ]);

        // graphicalEvent
        event_attrs.extend(&[
            "onactivate",
            "onclick",
            "onfocusin",
            "onfocusout",
            "onload",
            "onmousedown",
            "onmousemove",
            "onmouseout",
            "onmouseover",
            "onmouseup",
        ]);

        Self {
            config: RemoveScriptsConfig::default(),
            event_attrs,
        }
    }

    /// Create a new RemoveScriptsPlugin with config
    pub fn with_config(config: RemoveScriptsConfig) -> Self {
        let mut plugin = Self::new();
        plugin.config = config;
        plugin
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<RemoveScriptsConfig> {
        if params.is_null() {
            Ok(RemoveScriptsConfig::default())
        } else if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Err(anyhow::anyhow!("Configuration must be an object"))
        }
    }

    fn has_javascript_href(element: &Element) -> bool {
        element.attributes.iter().any(|(name, value)| {
            (name == "href" || name.ends_with(":href"))
                && value.trim_start().starts_with("javascript:")
        })
    }

    #[allow(dead_code)]
    fn trim_text_nodes_recursive(element: &mut Element) {
        for child in &mut element.children {
            match child {
                Node::Element(elem) => Self::trim_text_nodes_recursive(elem),
                Node::Text(text) => {
                    *text = text.trim().to_string().into();
                }
                _ => {}
            }
        }
    }

    /// Mirrors SVGO's two-phase visitor: enter strips event attrs and
    /// `<script>` elements; exit unwraps `<a>` elements whose href starts
    /// with `javascript:`, splicing their non-text children into the parent.
    fn process_element(&self, element: &mut Element) {
        element
            .attributes
            .retain(|name, _| !self.event_attrs.contains(name.as_ref()));

        let mut processed_children = Vec::with_capacity(element.children.len());
        for child in std::mem::take(&mut element.children) {
            match child {
                Node::Element(mut elem) => {
                    self.process_element(&mut elem);

                    if elem.name == "script" {
                        continue;
                    }

                    if elem.name == "a" && Self::has_javascript_href(&elem) {
                        for grandchild in elem.children {
                            if !matches!(grandchild, Node::Text(_)) {
                                processed_children.push(grandchild);
                            }
                        }
                        continue;
                    }

                    processed_children.push(Node::Element(elem));
                }
                other => processed_children.push(other),
            }
        }
        element.children = processed_children;
    }
}

impl Default for RemoveScriptsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveScriptsPlugin {
    fn name(&self) -> &'static str {
        "removeScripts"
    }

    fn description(&self) -> &'static str {
        "removes scripts (disabled by default)"
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
        let plugin = RemoveScriptsPlugin::new();
        assert_eq!(plugin.name(), "removeScripts");
        assert_eq!(
            plugin.description(),
            "removes scripts (disabled by default)"
        );
    }

    #[test]
    fn test_removes_script_elements() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add script element
        let mut script = create_element("script");
        script
            .children
            .push(Node::Text("alert('hello');".into()));
        doc.root.children.push(Node::Element(script));

        // Add non-script element
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that script was removed
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        } else {
            panic!("Expected element node");
        }
    }

    #[test]
    fn test_removes_event_attributes() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");
        doc.root.set_attr("onclick", "alert('clicked')");
        doc.root.set_attr("onload", "init()");
        doc.root.set_attr("width", "100");

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that event attributes were removed
        assert!(!doc.root.attributes.contains_key("onclick"));
        assert!(!doc.root.attributes.contains_key("onload"));
        assert_eq!(doc.root.attr("width"), Some("100"));
    }

    #[test]
    fn test_removes_javascript_hrefs() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        let mut anchor = create_element("a");
        anchor.set_attr("href", "javascript:void(0)");
        anchor.children.push(Node::Text("Click me".to_string().into()));
        let rect = create_element("rect");
        anchor.children.push(Node::Element(rect));

        doc.root.children.push(Node::Element(anchor));

        plugin.apply(&mut doc).unwrap();

        // SVGO unwraps <a> with javascript: href — anchor is replaced by
        // its non-text children directly in the parent
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "rect");
        } else {
            panic!("Expected rect element after unwrapping anchor");
        }
    }

    #[test]
    fn test_removes_xlink_javascript_hrefs() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        let mut anchor = create_element("a");
        anchor.set_attr("xlink:href", "  javascript:alert('test')");
        anchor
            .children
            .push(Node::Element(create_element("circle")));

        doc.root.children.push(Node::Element(anchor));

        plugin.apply(&mut doc).unwrap();

        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "circle");
        } else {
            panic!("Expected circle element after unwrapping anchor");
        }
    }

    #[test]
    fn test_preserves_non_javascript_hrefs() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add anchor with normal href
        let mut anchor = create_element("a");
        anchor.set_attr("href", "https://example.com");
        anchor.children.push(Node::Text("Link".to_string().into()));

        doc.root.children.push(Node::Element(anchor));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that normal href was preserved
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.attr("href"), Some("https://example.com"));
            assert_eq!(elem.children.len(), 1); // Text node preserved
        }
    }

    #[test]
    fn test_nested_script_removal() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Create nested structure
        let mut group = create_element("g");
        group.set_attr("onclick", "handleClick()");

        let mut script = create_element("script");
        script
            .children
            .push(Node::Text("console.log('test');".into()));
        group.children.push(Node::Element(script));

        let mut rect = create_element("rect");
        rect.set_attr("onmouseover", "highlight()");
        group.children.push(Node::Element(rect));

        doc.root.children.push(Node::Element(group));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check nested removal
        if let Node::Element(g) = &doc.root.children[0] {
            assert!(!g.attributes.contains_key("onclick"));
            assert_eq!(g.children.len(), 1); // Only rect remains

            if let Node::Element(rect) = &g.children[0] {
                assert!(!rect.attributes.contains_key("onmouseover"));
            }
        }
    }

    #[test]
    fn test_removes_all_event_types() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add various event attributes
        doc.root.set_attr("onbegin", "startAnim()"); // animationEvent
        doc.root.set_attr("onzoom", "handleZoom()"); // documentEvent
        doc.root.set_attr("oncopy", "handleCopy()"); // documentElementEvent
        doc.root.set_attr("ondrag", "handleDrag()"); // globalEvent
        doc.root.set_attr("onfocusin", "handleFocus()"); // graphicalEvent
        doc.root.set_attr("viewBox", "0 0 100 100"); // non-event

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all event attributes were removed
        assert_eq!(doc.root.attributes.len(), 1);
        assert_eq!(doc.root.attr("viewBox"), Some("0 0 100 100"));
    }

    #[test]
    fn test_empty_document() {
        let plugin = RemoveScriptsPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Apply plugin to empty document
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveScriptsPlugin::new();

        // Empty object is valid
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Null is valid
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Non-object is invalid
        assert!(plugin.validate_params(&json!("invalid")).is_err());
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
            .join("removeScripts");

        if !fixtures_path.exists() {
            println!("No fixtures found for plugin: removeScripts");
            return Ok(());
        }

        let fixtures = load_fixtures(&fixtures_path)?;

        for fixture in fixtures {
            let mut config = Config::new();
            config.plugins = vec![
                vexy_svgo_core::PluginConfig::Name("removeScripts".to_string()),
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
