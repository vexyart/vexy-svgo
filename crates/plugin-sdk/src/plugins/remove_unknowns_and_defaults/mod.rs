// this_file: crates/plugin-sdk/src/plugins/remove_unknowns_and_defaults/mod.rs

pub mod default_attrs;
pub mod unknown_elements;

use crate::Plugin;
use anyhow::Result;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

use self::default_attrs::should_remove_attribute;
use self::unknown_elements::should_remove_unknown_element;

/// Configuration parameters for RemoveUnknownsAndDefaults plugin
#[derive(Debug, Clone)]
pub struct RemoveUnknownsAndDefaultsConfig {
    pub unknown_content: bool,
    pub unknown_attrs: bool,
    pub default_attrs: bool,
    pub default_markup_declarations: bool,
    pub useless_overrides: bool,
    pub keep_data_attrs: bool,
    pub keep_aria_attrs: bool,
    pub keep_role_attr: bool,
}

impl Default for RemoveUnknownsAndDefaultsConfig {
    fn default() -> Self {
        Self {
            unknown_content: true,
            unknown_attrs: true,
            default_attrs: true,
            default_markup_declarations: true,
            useless_overrides: true,
            keep_data_attrs: true,
            keep_aria_attrs: true,
            keep_role_attr: false,
        }
    }
}

/// Plugin that removes unknown elements, attributes, and default values
pub struct RemoveUnknownsAndDefaultsPlugin {
    config: RemoveUnknownsAndDefaultsConfig,
}

impl RemoveUnknownsAndDefaultsPlugin {
    /// Create a new RemoveUnknownsAndDefaultsPlugin with default configuration
    pub fn new() -> Self {
        Self {
            config: RemoveUnknownsAndDefaultsConfig::default(),
        }
    }

    /// Create a new RemoveUnknownsAndDefaultsPlugin with custom configuration
    pub fn with_config(config: RemoveUnknownsAndDefaultsConfig) -> Self {
        Self { config }
    }
}

impl Default for RemoveUnknownsAndDefaultsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveUnknownsAndDefaultsPlugin {
    fn name(&self) -> &'static str {
        "removeUnknownsAndDefaults"
    }

    fn description(&self) -> &'static str {
        "Remove unknown elements, attributes, and default values"
    }

    fn validate_params(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        if let Some(obj) = params.as_object() {
            for (key, value) in obj {
                match key.as_str() {
                    "unknownContent"
                    | "unknownAttrs"
                    | "defaultAttrs"
                    | "defaultMarkupDeclarations"
                    | "uselessOverrides"
                    | "keepDataAttrs"
                    | "keepAriaAttrs"
                    | "keepRoleAttr" => {
                        if !value.is_boolean() {
                            return Err(anyhow::anyhow!("{} must be a boolean", key));
                        }
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown parameter: {}", key));
                    }
                }
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let mut visitor = UnknownsAndDefaultsRemovalVisitor::new(self.config.clone());
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that removes unknowns and defaults
struct UnknownsAndDefaultsRemovalVisitor {
    config: RemoveUnknownsAndDefaultsConfig,
    element_stack: Vec<String>, // Track parent elements for inheritance checking
}

impl UnknownsAndDefaultsRemovalVisitor {
    fn new(config: RemoveUnknownsAndDefaultsConfig) -> Self {
        Self {
            config,
            element_stack: Vec::new(),
        }
    }

    fn get_parent_element_name(&self) -> Option<&str> {
        self.element_stack
            .get(self.element_stack.len().saturating_sub(2))
            .map(|s| s.as_str())
    }
}

impl Visitor<'_> for UnknownsAndDefaultsRemovalVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Push current element to stack
        self.element_stack.push(element.name.to_string());

        // Get parent element for inheritance checking
        let _parent_element_name = self.get_parent_element_name();

        // Remove unknown and default attributes
        let _plugin = RemoveUnknownsAndDefaultsPlugin::with_config(self.config.clone());

        // Collect attributes to remove to avoid borrowing issues
        let mut attrs_to_remove = Vec::new();
        for (attr_name, attr_value) in &element.attributes {
            if should_remove_attribute(attr_name, attr_value, element, None, &self.config) {
                attrs_to_remove.push(attr_name.clone());
            }
        }

        // Remove the identified attributes
        for attr_name in attrs_to_remove {
            element.attributes.shift_remove(&attr_name);
        }

        Ok(())
    }

    fn visit_element_exit(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Pop current element from stack
        self.element_stack.pop();

        // Remove unknown child elements
        let _plugin = RemoveUnknownsAndDefaultsPlugin::with_config(self.config.clone());

        element.children.retain(|child| {
            match child {
                Node::Element(child_element) => {
                    !should_remove_unknown_element(child_element, self.config.unknown_content)
                }
                _ => true, // Keep non-element nodes
            }
        });

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

    fn create_element_with_attrs(name: &'static str, attrs: &[(&str, &str)]) -> Element<'static> {
        let mut element = create_element(name);
        for (key, value) in attrs {
            element.set_attr(*key, *value);
        }
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        assert_eq!(plugin.name(), "removeUnknownsAndDefaults");
    }

    #[test]
    fn test_configuration_defaults() {
        let config = RemoveUnknownsAndDefaultsConfig::default();
        assert!(config.unknown_content);
        assert!(config.unknown_attrs);
        assert!(config.default_attrs);
        assert!(config.default_markup_declarations);
        assert!(config.useless_overrides);
        assert!(config.keep_data_attrs);
        assert!(config.keep_aria_attrs);
        assert!(!config.keep_role_attr);
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({"unknownContent": true}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"keepDataAttrs": false}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"unknownContent": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"invalidParam": true}))
            .is_err());
    }

    #[test]
    fn test_known_elements() {
        let known = unknown_elements::known_elements();
        assert!(known.contains("svg"));
        assert!(known.contains("rect"));
        assert!(known.contains("circle"));
        assert!(known.contains("path"));
        assert!(!known.contains("unknown-element"));
    }

    #[test]
    fn test_known_attributes() {
        let known = default_attrs::known_attributes();
        assert!(known.contains("id"));
        assert!(known.contains("class"));
        assert!(known.contains("fill"));
        assert!(known.contains("stroke"));
        assert!(!known.contains("unknown-attr"));
    }

    #[test]
    fn test_default_values() {
        let defaults = default_attrs::default_attribute_values();
        assert_eq!(defaults.get("x"), Some(&"0"));
        assert_eq!(defaults.get("y"), Some(&"0"));
        assert_eq!(defaults.get("fill"), Some(&"black"));
        assert_eq!(defaults.get("stroke"), Some(&"none"));
    }

    #[test]
    fn test_should_remove_unknown_element() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();

        // Known elements should not be removed
        let rect = create_element("rect");
        assert!(!should_remove_unknown_element(&rect, plugin.config.unknown_content));

        // Unknown elements should be removed
        let unknown = create_element("unknown-element");
        assert!(should_remove_unknown_element(&unknown, plugin.config.unknown_content));

        // Namespaced elements should not be removed
        let mut namespaced = create_element("custom:element");
        namespaced.name = Cow::Borrowed("custom:element");
        assert!(!should_remove_unknown_element(&namespaced, plugin.config.unknown_content));
    }

    #[test]
    fn test_should_remove_attribute_unknown() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        let element = create_element("rect");

        // Known attributes should not be removed
        assert!(!should_remove_attribute("fill", "red", &element, None, &plugin.config));

        // Unknown attributes should be removed
        assert!(should_remove_attribute("unknown-attr", "value", &element, None, &plugin.config));

        // Data attributes should be preserved by default
        assert!(!should_remove_attribute("data-test", "value", &element, None, &plugin.config));

        // ARIA attributes should be preserved by default
        assert!(!should_remove_attribute("aria-label", "test", &element, None, &plugin.config));

        // Role attribute should be removed by default (keepRoleAttr is false)
        assert!(should_remove_attribute("role", "button", &element, None, &plugin.config));

        // Role attribute should be kept when keepRoleAttr is true
        let mut config = RemoveUnknownsAndDefaultsConfig::default();
        config.keep_role_attr = true;
        let plugin_keep_role = RemoveUnknownsAndDefaultsPlugin::with_config(config);
        assert!(!should_remove_attribute("role", "button", &element, None, &plugin_keep_role.config));
    }

    #[test]
    fn test_should_remove_attribute_defaults() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        let element = create_element("rect");

        // Default values should be removed
        assert!(should_remove_attribute("x", "0", &element, None, &plugin.config));
        assert!(should_remove_attribute("fill", "black", &element, None, &plugin.config));

        // Non-default values should not be removed
        assert!(!should_remove_attribute("x", "10", &element, None, &plugin.config));
        assert!(!should_remove_attribute("fill", "red", &element, None, &plugin.config));

        // Elements with id should keep their default values
        let element_with_id = create_element_with_attrs("rect", &[("id", "test")]);
        assert!(!should_remove_attribute("x", "0", &element_with_id, None, &plugin.config));
    }

    #[test]
    fn test_should_remove_attribute_namespaced() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        let element = create_element("rect");

        // xmlns attributes should be preserved
        assert!(!should_remove_attribute(
            "xmlns",
            "http://www.w3.org/2000/svg",
            &element,
            None,
            &plugin.config
        ));
        assert!(!should_remove_attribute(
            "xmlns:xlink",
            "http://www.w3.org/1999/xlink",
            &element,
            None,
            &plugin.config
        ));

        // xml: and xlink: attributes should be preserved
        assert!(!should_remove_attribute("xml:space", "preserve", &element, None, &plugin.config));
        assert!(!should_remove_attribute("xlink:href", "#test", &element, None, &plugin.config));

        // Other namespaced attributes should be removed if unknown
        assert!(should_remove_attribute("custom:attr", "value", &element, None, &plugin.config));
    }

    #[test]
    fn test_plugin_apply_remove_unknown_elements() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        let mut doc = Document::new();

        // Add known and unknown elements
        doc.root
            .children
            .push(Node::Element(create_element("rect")));
        doc.root
            .children
            .push(Node::Element(create_element("unknown-element")));
        doc.root
            .children
            .push(Node::Element(create_element("circle")));

        plugin.apply(&mut doc).unwrap();

        // Unknown element should be removed
        assert_eq!(doc.root.children.len(), 2);

        // Check that known elements remain
        let element_names: Vec<&str> = doc
            .root
            .children
            .iter()
            .filter_map(|child| match child {
                Node::Element(elem) => Some(elem.name.as_ref()),
                _ => None,
            })
            .collect();

        assert!(element_names.contains(&"rect"));
        assert!(element_names.contains(&"circle"));
        assert!(!element_names.contains(&"unknown-element"));
    }

    #[test]
    fn test_plugin_apply_remove_unknown_attributes() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        let mut doc = Document::new();

        // Create element with known and unknown attributes
        let element = create_element_with_attrs(
            "rect",
            &[
                ("width", "100"),
                ("height", "100"),
                ("unknown-attr", "value"),
                ("data-test", "keep"),
            ],
        );

        doc.root.children.push(Node::Element(element));

        plugin.apply(&mut doc).unwrap();

        if let Some(Node::Element(rect)) = doc.root.children.first() {
            assert!(rect.attributes.contains_key("width"));
            assert!(rect.attributes.contains_key("height"));
            assert!(rect.attributes.contains_key("data-test"));
            assert!(!rect.attributes.contains_key("unknown-attr"));
        }
    }

    #[test]
    fn test_plugin_apply_remove_default_values() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        let mut doc = Document::new();

        // Create element with default and non-default values
        let element = create_element_with_attrs(
            "rect",
            &[
                ("x", "0"),        // default
                ("y", "10"),       // non-default
                ("fill", "black"), // default
                ("stroke", "red"), // non-default
            ],
        );

        doc.root.children.push(Node::Element(element));

        plugin.apply(&mut doc).unwrap();

        if let Some(Node::Element(rect)) = doc.root.children.first() {
            assert!(!rect.attributes.contains_key("x"));
            assert!(rect.attributes.contains_key("y"));
            assert!(!rect.attributes.contains_key("fill"));
            assert!(rect.attributes.contains_key("stroke"));
        }
    }

    #[test]
    fn test_plugin_apply_preserve_elements_with_id() {
        let plugin = RemoveUnknownsAndDefaultsPlugin::new();
        let mut doc = Document::new();

        // Create element with id and default values
        let element = create_element_with_attrs(
            "rect",
            &[
                ("id", "test"),
                ("x", "0"),        // default, but should be kept
                ("fill", "black"), // default, but should be kept
            ],
        );

        doc.root.children.push(Node::Element(element));

        plugin.apply(&mut doc).unwrap();

        if let Some(Node::Element(rect)) = doc.root.children.first() {
            assert!(rect.attributes.contains_key("id"));
            assert!(rect.attributes.contains_key("x"));
            assert!(rect.attributes.contains_key("fill"));
        }
    }
}
