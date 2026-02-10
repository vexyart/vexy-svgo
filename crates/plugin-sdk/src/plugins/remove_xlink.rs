// this_file: crates/plugin-sdk/src/plugins/remove_xlink.rs

//! Plugin to remove xlink namespace and replace attributes with SVG 2 equivalents
//!
//! This plugin removes the deprecated XLink namespace and converts XLink attributes
//! to their SVG 2 equivalents where applicable. XLink was deprecated in SVG 2.
//!
//! Reference: SVGO's removeXlink plugin

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// XLink namespace URI
const XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";

/// Elements that use xlink:href but were deprecated in SVG 2
const LEGACY_ELEMENTS: &[&str] = &["cursor", "filter", "font-face-uri", "glyphRef", "tref"];

/// Configuration for the removeXlink plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveXlinkConfig {
    /// Include legacy elements that don't support SVG 2 href
    #[serde(default)]
    pub include_legacy: bool,
}

impl Default for RemoveXlinkConfig {
    fn default() -> Self {
        Self {
            include_legacy: false,
        }
    }
}

/// Plugin to remove xlink namespace and convert to SVG 2 equivalents
pub struct RemoveXlinkPlugin {
    config: RemoveXlinkConfig,
}

impl RemoveXlinkPlugin {
    pub fn new() -> Self {
        Self {
            config: RemoveXlinkConfig::default(),
        }
    }

    pub fn with_config(config: RemoveXlinkConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveXlinkConfig> {
        if params.is_null() {
            Ok(RemoveXlinkConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn process_element(&self, element: &mut Element, context: &mut XlinkContext) {
        // Collect xlink namespace prefixes
        let mut current_xlink_prefixes = Vec::new();

        for (key, value) in &element.attributes {
            if key.starts_with("xmlns:") && value == XLINK_NAMESPACE {
                let prefix = key.strip_prefix("xmlns:").unwrap();
                current_xlink_prefixes.push(prefix.to_string());
                context.xlink_prefixes.push(prefix.to_string());
            }
        }

        // Handle xlink:href conversion
        let is_legacy = LEGACY_ELEMENTS.contains(&element.name.as_ref());

        if is_legacy && !self.config.include_legacy {
            // Mark prefixes as used in legacy elements - check for any xlink attributes
            let has_xlink_attrs = element.attributes.keys().any(|key| {
                context
                    .xlink_prefixes
                    .iter()
                    .any(|prefix| key.starts_with(&format!("{}:", prefix)))
            });

            if has_xlink_attrs {
                for prefix in &context.xlink_prefixes {
                    if !context.used_in_legacy.contains(prefix) {
                        context.used_in_legacy.push(prefix.clone());
                    }
                }
            }
        } else {
            // Convert xlink:href to href if no href exists
            self.convert_href_attributes(element, &context.xlink_prefixes);
        }

        // Only convert other xlink attributes if not a legacy element or include_legacy is true
        if !is_legacy || self.config.include_legacy {
            // Handle xlink:show conversion to target
            self.convert_show_attributes(element, &context.xlink_prefixes);

            // Handle xlink:title conversion to <title> element
            self.convert_title_attributes(element, &context.xlink_prefixes);

            // Remove unused xlink attributes
            self.remove_unused_xlink_attributes(
                element,
                &context.xlink_prefixes,
                &context.used_in_legacy,
            );
        }

        // Process children recursively
        for child in &mut element.children {
            if let Node::Element(ref mut elem) = child {
                self.process_element(elem, context);
            }
        }

        // Remove xlink namespace declarations if not used in legacy elements
        for prefix in &current_xlink_prefixes {
            if !context.used_in_legacy.contains(prefix) {
                let xmlns_key = format!("xmlns:{}", prefix);
                element.remove_attr(&xmlns_key);
            }
        }

        // Remove processed prefixes from context
        for prefix in &current_xlink_prefixes {
            context.xlink_prefixes.retain(|p| p != prefix);
        }
    }

    fn convert_href_attributes(&self, element: &mut Element, xlink_prefixes: &[String]) {
        // Find xlink:href attributes
        let href_attrs: Vec<String> = element
            .attributes
            .keys()
            .filter(|key| {
                xlink_prefixes
                    .iter()
                    .any(|prefix| *key == &format!("{}:href", prefix))
            })
            .map(|k| k.to_string())
            .collect();

        for href_attr in href_attrs {
            if let Some(href_value) = element.attr(&href_attr).map(|s| s.to_string()) {
                // Only convert if no href attribute exists
                if !element.has_attr("href") {
                    element.set_attr("href", &href_value);
                }
                element.remove_attr(&href_attr);
            }
        }
    }

    fn convert_show_attributes(&self, element: &mut Element, xlink_prefixes: &[String]) {
        // Find xlink:show attributes
        let show_attrs: Vec<String> = element
            .attributes
            .keys()
            .filter(|key| {
                xlink_prefixes
                    .iter()
                    .any(|prefix| *key == &format!("{}:show", prefix))
            })
            .map(|k| k.to_string())
            .collect();

        for show_attr in show_attrs {
            if let Some(show_value) = element.attr(&show_attr) {
                // Convert to target attribute if no target exists
                if !element.has_attr("target") {
                    let target_value = match show_value.as_ref() {
                        "new" => "_blank",
                        "replace" => "_self",
                        _ => {
                            // Remove unknown values
                            element.remove_attr(&show_attr);
                            continue;
                        }
                    };
                    element.set_attr("target", target_value);
                }
                element.remove_attr(&show_attr);
            }
        }
    }

    fn convert_title_attributes(&self, element: &mut Element, xlink_prefixes: &[String]) {
        // Find xlink:title attributes
        let title_attrs: Vec<String> = element
            .attributes
            .keys()
            .filter(|key| {
                xlink_prefixes
                    .iter()
                    .any(|prefix| *key == &format!("{}:title", prefix))
            })
            .map(|k| k.to_string())
            .collect();

        for title_attr in title_attrs {
            if let Some(title_value) = element.attr(&title_attr) {
                // Check if element already has a title child
                let has_title_child = element
                    .children
                    .iter()
                    .any(|child| matches!(child, Node::Element(elem) if elem.name == "title"));

                if !has_title_child {
                    // Create title element
                    let title_element = Element {
                        name: "title".to_string().into(),
                        attributes: indexmap::IndexMap::new(),
                        namespaces: indexmap::IndexMap::new(),
                        children: vec![Node::Text(title_value.to_string().into())],
                    };
                    element.children.insert(0, Node::Element(title_element));
                }
                element.remove_attr(&title_attr);
            }
        }
    }

    fn remove_unused_xlink_attributes(
        &self,
        element: &mut Element,
        xlink_prefixes: &[String],
        used_in_legacy: &[String],
    ) {
        // Remove any remaining xlink attributes that weren't converted
        let attrs_to_remove: Vec<String> = element
            .attributes
            .keys()
            .filter(|key| {
                if let Some(colon_pos) = key.find(':') {
                    let prefix = &key[..colon_pos];
                    xlink_prefixes.contains(&prefix.to_string())
                        && !used_in_legacy.contains(&prefix.to_string())
                } else {
                    false
                }
            })
            .map(|k| k.to_string())
            .collect();

        for attr in attrs_to_remove {
            element.remove_attr(&attr);
        }
    }
}

impl Default for RemoveXlinkPlugin {
    fn default() -> Self {
        Self::new()
    }
}

struct XlinkContext {
    xlink_prefixes: Vec<String>,
    used_in_legacy: Vec<String>,
}

impl XlinkContext {
    fn new() -> Self {
        Self {
            xlink_prefixes: Vec::new(),
            used_in_legacy: Vec::new(),
        }
    }
}

impl Plugin for RemoveXlinkPlugin {
    fn name(&self) -> &'static str {
        "removeXlink"
    }

    fn description(&self) -> &'static str {
        "remove xlink namespace and replaces attributes with the SVG 2 equivalent where applicable"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        let mut context = XlinkContext::new();
        self.process_element(&mut document.root, &mut context);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use vexy_svgo_core::ast::{Document, Element, Node};
    use std::borrow::Cow;

    fn create_test_document() -> Document<'static> {
        Document {
            root: Element {
                name: Cow::Borrowed("svg"),
                attributes: IndexMap::new(),
                namespaces: IndexMap::new(),
                children: vec![],
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveXlinkPlugin::new();
        assert_eq!(plugin.name(), "removeXlink");
        assert_eq!(
            plugin.description(),
            "remove xlink namespace and replaces attributes with the SVG 2 equivalent where applicable"
        );
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveXlinkPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test valid params
        assert!(plugin
            .validate_params(&serde_json::json!({
                "includeLegacy": true
            }))
            .is_ok());

        // Test invalid params
        assert!(plugin
            .validate_params(&serde_json::json!({
                "invalidParam": true
            }))
            .is_err());
    }

    #[test]
    fn test_convert_xlink_href_to_href() {
        let mut document = create_test_document();

        // Add xlink namespace
        document.root.set_attr("xmlns:xlink", XLINK_NAMESPACE);

        // Add element with xlink:href
        let mut use_element = Element {
            name: Cow::Borrowed("use"),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };
        use_element.set_attr("xlink:href", "#symbol1");

        document.root.children = vec![Node::Element(use_element)];

        let plugin = RemoveXlinkPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // xlink:href should be converted to href
        if let Node::Element(ref use_elem) = document.root.children[0] {
            assert!(!use_elem.has_attr("xlink:href"));
            assert_eq!(use_elem.attr("href"), Some("#symbol1"));
        } else {
            panic!("Expected use element");
        }

        // xlink namespace should be removed
        assert!(!document.root.has_attr("xmlns:xlink"));
    }

    #[test]
    fn test_preserve_existing_href() {
        let mut document = create_test_document();

        document.root.set_attr("xmlns:xlink", XLINK_NAMESPACE);

        let mut element = Element {
            name: Cow::Borrowed("a"),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };
        element.set_attr("href", "#existing");
        element.set_attr("xlink:href", "#xlink");

        document.root.children = vec![Node::Element(element)];

        let plugin = RemoveXlinkPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // Should preserve existing href and remove xlink:href
        if let Node::Element(ref elem) = document.root.children[0] {
            assert!(!elem.has_attr("xlink:href"));
            assert_eq!(elem.attr("href"), Some("#existing"));
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_convert_xlink_show_to_target() {
        let mut document = create_test_document();

        document.root.set_attr("xmlns:xlink", XLINK_NAMESPACE);

        let mut element = Element {
            name: Cow::Borrowed("a"),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };
        element.set_attr("xlink:show", "new");

        document.root.children = vec![Node::Element(element)];

        let plugin = RemoveXlinkPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // xlink:show="new" should become target="_blank"
        if let Node::Element(ref elem) = document.root.children[0] {
            assert!(!elem.has_attr("xlink:show"));
            assert_eq!(elem.attr("target"), Some("_blank"));
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_convert_xlink_title_to_title_element() {
        let mut document = create_test_document();

        document.root.set_attr("xmlns:xlink", XLINK_NAMESPACE);

        let mut element = Element {
            name: Cow::Borrowed("rect"),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };
        element.set_attr("xlink:title", "Element title");

        document.root.children = vec![Node::Element(element)];

        let plugin = RemoveXlinkPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // xlink:title should be converted to title element
        if let Node::Element(ref elem) = document.root.children[0] {
            assert!(!elem.has_attr("xlink:title"));
            assert_eq!(elem.children.len(), 1);

            if let Node::Element(ref title) = elem.children[0] {
                assert_eq!(title.name, "title");
                assert_eq!(title.children.len(), 1);
                if let Node::Text(ref text) = title.children[0] {
                    assert_eq!(text.as_ref(), "Element title");
                } else {
                    panic!("Expected text in title");
                }
            } else {
                panic!("Expected title element");
            }
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_preserve_legacy_elements() {
        let mut document = create_test_document();

        document.root.set_attr("xmlns:xlink", XLINK_NAMESPACE);

        let mut filter_element = Element {
            name: Cow::Borrowed("filter"),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };
        filter_element.set_attr("xlink:href", "#filter1");

        document.root.children = vec![Node::Element(filter_element)];

        let plugin = RemoveXlinkPlugin::new();
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // Legacy elements should preserve xlink attributes by default
        if let Node::Element(ref filter) = document.root.children[0] {
            assert!(filter.has_attr("xlink:href"));
            assert!(!filter.has_attr("href"));
        } else {
            panic!("Expected filter element");
        }

        // xlink namespace should be preserved for legacy usage
        assert!(document.root.has_attr("xmlns:xlink"));
    }

    #[test]
    fn test_include_legacy_option() {
        let mut document = create_test_document();

        document.root.set_attr("xmlns:xlink", XLINK_NAMESPACE);

        let mut filter_element = Element {
            name: Cow::Borrowed("filter"),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };
        filter_element.set_attr("xlink:href", "#filter1");

        document.root.children = vec![Node::Element(filter_element)];

        let params = serde_json::json!({"includeLegacy": true});
        let plugin = RemoveXlinkPlugin::new();
        plugin.validate_params(&params).unwrap();
        let plugin = RemoveXlinkPlugin::with_config(RemoveXlinkConfig {
            include_legacy: true,
        });
        let result = plugin.apply(&mut document);
        assert!(result.is_ok());

        // With includeLegacy=true, should convert even legacy elements
        if let Node::Element(ref filter) = document.root.children[0] {
            assert!(!filter.has_attr("xlink:href"));
            assert_eq!(filter.attr("href"), Some("#filter1"));
        } else {
            panic!("Expected filter element");
        }
    }
}
