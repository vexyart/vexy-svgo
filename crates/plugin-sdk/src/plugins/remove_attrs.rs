// this_file: crates/plugin-sdk/src/plugins/remove_attrs.rs

//! Remove specified attributes based on patterns
//!
//! This plugin removes attributes from elements based on flexible pattern matching.
//! Patterns can specify element names, attribute names, and attribute values using regex.
//!
//! Reference: SVGO's removeAttrs plugin

use crate::Plugin;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration for the removeAttrs plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveAttrsConfig {
    /// Attribute patterns to remove (element:attribute:value format)
    pub attrs: Vec<String>,

    /// Element separator for patterns (default ":")
    #[serde(default = "default_elem_separator")]
    pub elem_separator: String,

    /// Whether to preserve currentColor values in fill/stroke attributes
    #[serde(default)]
    pub preserve_current_color: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemoveAttrsConfigInput {
    attrs: AttrPatterns,
    #[serde(default)]
    elem_separator: Option<String>,
    #[serde(default)]
    preserve_current_color: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AttrPatterns {
    One(String),
    Many(Vec<String>),
}

impl AttrPatterns {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        }
    }
}

fn default_elem_separator() -> String {
    ":".to_string()
}

impl Default for RemoveAttrsConfig {
    fn default() -> Self {
        Self {
            attrs: Vec::new(),
            elem_separator: default_elem_separator(),
            preserve_current_color: false,
        }
    }
}

/// Compiled pattern for attribute removal
#[derive(Debug)]
struct CompiledPattern {
    element_regex: Regex,
    attribute_regex: Regex,
    value_regex: Regex,
}

impl CompiledPattern {
    /// Compile a pattern string into regex components
    fn compile(pattern: &str, separator: &str) -> Result<Self> {
        let mut parts: Vec<String> = pattern.split(separator).map(|s| s.to_string()).collect();

        // Expand pattern based on number of parts
        match parts.len() {
            1 => {
                // Just attribute name - apply to all elements with any value
                parts.insert(0, ".*".to_string());
                parts.push(".*".to_string());
            }
            2 => {
                // Element and attribute - apply with any value
                parts.push(".*".to_string());
            }
            3 => {
                // Full pattern - use as is
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid pattern format: {}. Pattern should have at most 3 parts separated by '{}'",
                    pattern, separator
                ));
            }
        }

        // Convert single * to .*
        for part in &mut parts {
            if part == "*" {
                *part = ".*".to_string();
            }
        }

        // Compile regexes
        let element_regex = Regex::new(&format!("^{}$", parts[0]))
            .map_err(|e| anyhow::anyhow!("Invalid element regex '{}': {}", parts[0], e))?;
        let attribute_regex = Regex::new(&format!("^{}$", parts[1]))
            .map_err(|e| anyhow::anyhow!("Invalid attribute regex '{}': {}", parts[1], e))?;
        let value_regex = Regex::new(&format!("^{}$", parts[2]))
            .map_err(|e| anyhow::anyhow!("Invalid value regex '{}': {}", parts[2], e))?;

        Ok(Self {
            element_regex,
            attribute_regex,
            value_regex,
        })
    }

    /// Check if this pattern matches the given element, attribute, and value
    fn matches(&self, element_name: &str, attr_name: &str, attr_value: &str) -> bool {
        self.element_regex.is_match(element_name)
            && self.attribute_regex.is_match(attr_name)
            && self.value_regex.is_match(attr_value)
    }
}

/// Main plugin struct for removing attributes
pub struct RemoveAttrsPlugin {
    config: RemoveAttrsConfig,
}

impl RemoveAttrsPlugin {
    pub fn new() -> Self {
        Self {
            config: RemoveAttrsConfig::default(),
        }
    }

    pub fn with_config(config: RemoveAttrsConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveAttrsConfig> {
        if params.is_null() {
            return Err(anyhow::anyhow!(
                "removeAttrs plugin requires 'attrs' parameter"
            ));
        }

        // Handle both object format and simple string/array format
        let config = if params.is_object() {
            let parsed: RemoveAttrsConfigInput = serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))?;
            RemoveAttrsConfig {
                attrs: parsed.attrs.into_vec(),
                elem_separator: parsed.elem_separator.unwrap_or_else(default_elem_separator),
                preserve_current_color: parsed.preserve_current_color.unwrap_or(false),
            }
        } else if let Some(attrs_str) = params.as_str() {
            RemoveAttrsConfig {
                attrs: vec![attrs_str.to_string()],
                ..Default::default()
            }
        } else if let Some(attrs_array) = params.as_array() {
            let mut attrs = Vec::new();
            for attr in attrs_array {
                if let Some(s) = attr.as_str() {
                    attrs.push(s.to_string());
                } else {
                    return Err(anyhow::anyhow!("attrs array must contain only strings"));
                }
            }
            RemoveAttrsConfig {
                attrs,
                ..Default::default()
            }
        } else {
            return Err(anyhow::anyhow!(
                "removeAttrs plugin parameters must be an object, string, or array of strings"
            ));
        };

        // Validate that attrs is not empty
        if config.attrs.is_empty() {
            return Err(anyhow::anyhow!(
                "removeAttrs plugin requires non-empty 'attrs' parameter"
            ));
        }

        Ok(config)
    }

    fn process_element(&self, element: &mut Element, patterns: &[CompiledPattern]) {
        // Process children first
        let mut i = 0;
        while i < element.children.len() {
            if let Node::Element(child) = &mut element.children[i] {
                self.process_element(child, patterns);
            }
            i += 1;
        }

        // Remove matching attributes
        let mut attrs_to_remove = Vec::new();

        for (attr_name, attr_value) in &element.attributes {
            for pattern in patterns {
                if pattern.matches(&element.name, attr_name, attr_value) {
                    // Check for currentColor preservation
                    if self.config.preserve_current_color {
                        let is_current_color = attr_value.to_lowercase() == "currentcolor";
                        let is_fill_or_stroke = attr_name == "fill" || attr_name == "stroke";

                        if is_fill_or_stroke && is_current_color {
                            continue; // Skip removal
                        }
                    }

                    attrs_to_remove.push(attr_name.clone());
                    break; // No need to check other patterns for this attribute
                }
            }
        }

        // Remove the attributes
        for attr_name in attrs_to_remove {
            element.remove_attr(&attr_name);
        }
    }
}

impl Default for RemoveAttrsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveAttrsPlugin {
    fn name(&self) -> &'static str {
        "removeAttrs"
    }

    fn description(&self) -> &'static str {
        "removes specified attributes"
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
        // Parse the configuration fresh for this apply call
        let config = if self.config.attrs.is_empty() {
            return Err(anyhow::anyhow!(
                "removeAttrs plugin requires 'attrs' parameter"
            ));
        } else {
            self.config.clone()
        };

        // Compile all patterns
        let mut compiled_patterns = Vec::new();
        for pattern in &config.attrs {
            compiled_patterns.push(CompiledPattern::compile(pattern, &config.elem_separator)?);
        }

        self.process_element(&mut document.root, &compiled_patterns);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveAttrsPlugin::new();
        assert_eq!(plugin.name(), "removeAttrs");
        assert_eq!(plugin.description(), "removes specified attributes");
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveAttrsPlugin::new();

        // Test null params - should fail
        assert!(plugin.validate_params(&Value::Null).is_err());

        // Test empty object - should fail
        assert!(plugin.validate_params(&json!({})).is_err());

        // Test valid string param
        assert!(plugin.validate_params(&json!("fill")).is_ok());

        // Test valid array param
        assert!(plugin.validate_params(&json!(["fill", "stroke"])).is_ok());

        // Test valid object param
        assert!(plugin
            .validate_params(&json!({
                "attrs": ["fill"],
                "preserveCurrentColor": true
            }))
            .is_ok());

        // Test invalid param type in array
        assert!(plugin.validate_params(&json!(["fill", 123])).is_err());

        // Test unknown field
        assert!(plugin
            .validate_params(&json!({
                "attrs": ["fill"],
                "unknownField": true
            }))
            .is_err());
    }

    #[test]
    fn test_simple_attribute_removal() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "red");
        rect.set_attr("stroke", "blue");
        rect.set_attr("width", "100");

        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["fill".to_string()],
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(rect) = &document.root.children[0] {
            assert!(!rect.has_attr("fill"));
            assert!(rect.has_attr("stroke"));
            assert!(rect.has_attr("width"));
        }
    }

    #[test]
    fn test_multiple_attribute_removal() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "red");
        rect.set_attr("stroke", "blue");
        rect.set_attr("width", "100");

        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["fill".to_string(), "stroke".to_string()],
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(rect) = &document.root.children[0] {
            assert!(!rect.has_attr("fill"));
            assert!(!rect.has_attr("stroke"));
            assert!(rect.has_attr("width"));
        }
    }

    #[test]
    fn test_regex_pattern_removal() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "red");
        rect.set_attr("stroke", "blue");
        rect.set_attr("stroke-width", "2");
        rect.set_attr("stroke-opacity", "0.5");

        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["stroke.*".to_string()],
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(rect) = &document.root.children[0] {
            assert!(rect.has_attr("fill"));
            assert!(!rect.has_attr("stroke"));
            assert!(!rect.has_attr("stroke-width"));
            assert!(!rect.has_attr("stroke-opacity"));
        }
    }

    #[test]
    fn test_element_specific_removal() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut circle = Element::new("circle");
        circle.set_attr("fill", "red");

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "blue");

        document.root.children.push(Node::Element(circle));
        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["circle:fill".to_string()],
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        // Circle should have fill removed, rect should keep it
        if let Node::Element(circle_elem) = &document.root.children[0] {
            assert!(!circle_elem.has_attr("fill"));
        }
        if let Node::Element(rect_elem) = &document.root.children[1] {
            assert!(rect_elem.has_attr("fill"));
        }
    }

    #[test]
    fn test_value_specific_removal() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "red");
        rect.set_attr("stroke", "blue");

        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["*:fill:red".to_string()],
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(rect) = &document.root.children[0] {
            assert!(!rect.has_attr("fill"));
            assert!(rect.has_attr("stroke"));
        }
    }

    #[test]
    fn test_preserve_current_color() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "currentColor");
        rect.set_attr("stroke", "red");

        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["(fill|stroke)".to_string()],
            preserve_current_color: true,
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(rect) = &document.root.children[0] {
            assert!(rect.has_attr("fill")); // currentColor preserved
            assert!(!rect.has_attr("stroke")); // red removed
        }
    }

    #[test]
    fn test_preserve_current_color_case_insensitive() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "currentcolor");
        rect.set_attr("stroke", "CURRENTCOLOR");

        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["(fill|stroke)".to_string()],
            preserve_current_color: true,
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(rect) = &document.root.children[0] {
            assert!(rect.has_attr("fill")); // currentcolor preserved
            assert!(rect.has_attr("stroke")); // CURRENTCOLOR preserved
        }
    }

    #[test]
    fn test_custom_separator() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "red");
        rect.set_attr("stroke", "blue");

        document.root.children.push(Node::Element(rect));

        let config = RemoveAttrsConfig {
            attrs: vec!["rect|fill".to_string()],
            elem_separator: "|".to_string(),
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(rect) = &document.root.children[0] {
            assert!(!rect.has_attr("fill"));
            assert!(rect.has_attr("stroke"));
        }
    }

    #[test]
    fn test_nested_elements() {
        let mut document = Document::new();
        document.root.name = "svg".into();

        let mut group = Element::new("g");
        group.set_attr("fill", "red");

        let mut rect = Element::new("rect");
        rect.set_attr("fill", "blue");
        rect.set_attr("stroke", "green");

        group.children.push(Node::Element(rect));
        document.root.children.push(Node::Element(group));

        let config = RemoveAttrsConfig {
            attrs: vec!["fill".to_string()],
            ..Default::default()
        };
        let plugin = RemoveAttrsPlugin::with_config(config);
        plugin.apply(&mut document).unwrap();

        if let Node::Element(group_elem) = &document.root.children[0] {
            assert!(!group_elem.has_attr("fill"));

            if let Node::Element(rect_elem) = &group_elem.children[0] {
                assert!(!rect_elem.has_attr("fill"));
                assert!(rect_elem.has_attr("stroke"));
            }
        }
    }
}
