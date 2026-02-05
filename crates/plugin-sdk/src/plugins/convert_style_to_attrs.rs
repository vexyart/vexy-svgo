// this_file: crates/plugin-sdk/src/plugins/convert_style_to_attrs.rs

//! Convert style to attributes plugin implementation
//!
//! This plugin converts inline styles to SVG presentation attributes
//! where possible. It parses the style attribute and extracts any
//! properties that are valid presentation attributes.
//!
//! SVGO parameters supported:
//! - `keepImportant` (default: false) - Keep !important declarations in style

use crate::Plugin;
use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;
use vexy_svgo_core::ast::{Document, Element};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Configuration parameters for convert style to attrs plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertStyleToAttrsConfig {
    /// Keep !important declarations in style
    #[serde(default)]
    pub keep_important: bool,
}

impl Default for ConvertStyleToAttrsConfig {
    fn default() -> Self {
        Self {
            keep_important: false,
        }
    }
}

// List of SVG presentation attributes that can be converted from style
const PRESENTATION_ATTRS: &[&str] = &[
    "alignment-baseline",
    "baseline-shift",
    "clip",
    "clip-path",
    "clip-rule",
    "color",
    "color-interpolation",
    "color-interpolation-filters",
    "color-profile",
    "color-rendering",
    "cursor",
    "direction",
    "display",
    "dominant-baseline",
    "enable-background",
    "fill",
    "fill-opacity",
    "fill-rule",
    "filter",
    "flood-color",
    "flood-opacity",
    "font-family",
    "font-size",
    "font-size-adjust",
    "font-stretch",
    "font-style",
    "font-variant",
    "font-weight",
    "glyph-orientation-horizontal",
    "glyph-orientation-vertical",
    "image-rendering",
    "kerning",
    "letter-spacing",
    "lighting-color",
    "marker-end",
    "marker-mid",
    "marker-start",
    "mask",
    "opacity",
    "overflow",
    "pointer-events",
    "shape-rendering",
    "stop-color",
    "stop-opacity",
    "stroke",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "stroke-opacity",
    "stroke-width",
    "text-anchor",
    "text-decoration",
    "text-rendering",
    "unicode-bidi",
    "visibility",
    "word-spacing",
    "writing-mode",
];

// Regex for parsing CSS declarations
static CSS_DECLARATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        (?:
            /\*[\s\S]*?\*/  # CSS comments
            |
            (?:
                ([-\w]+)  # property name
                \s*:\s*
                (
                    (?:
                        /\*[\s\S]*?\*/  # inline comments
                        |
                        '(?:[^'\\]|\\.)*'  # single-quoted strings
                        |
                        "(?:[^"\\]|\\.)*"  # double-quoted strings
                        |
                        [^;'"/!]  # any other character except !
                    )+?
                )
                \s*
                (!important)?  # optional !important
            )
            \s*(?:;|$)  # declaration end
        )
        "#,
    )
    .unwrap()
});

/// Plugin that converts inline styles to SVG presentation attributes
pub struct ConvertStyleToAttrsPlugin {
    config: ConvertStyleToAttrsConfig,
}

impl ConvertStyleToAttrsPlugin {
    /// Create a new ConvertStyleToAttrsPlugin
    pub fn new() -> Self {
        Self {
            config: ConvertStyleToAttrsConfig::default(),
        }
    }

    /// Create a new ConvertStyleToAttrsPlugin with config
    pub fn with_config(config: ConvertStyleToAttrsConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    #[allow(dead_code)]
    fn parse_config(params: &Value) -> Result<ConvertStyleToAttrsConfig> {
        if let Some(_obj) = params.as_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(ConvertStyleToAttrsConfig::default())
        }
    }
}

impl Default for ConvertStyleToAttrsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ConvertStyleToAttrsPlugin {
    fn name(&self) -> &'static str {
        "convertStyleToAttrs"
    }

    fn description(&self) -> &'static str {
        "Convert inline styles to SVG presentation attributes"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            // Validate parameters
            for (key, value) in obj {
                match key.as_str() {
                    "keepImportant" => {
                        if !value.is_boolean() {
                            return Err(anyhow!("{} must be a boolean", key));
                        }
                    }
                    _ => return Err(anyhow!("Unknown parameter: {}", key)),
                }
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        let mut visitor = ConvertStyleToAttrsVisitor::new(self.config.clone());
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Strip CSS comments from a value string
fn strip_css_comments(value: &str) -> String {
    // Simple regex to remove CSS comments
    static COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/\*[\s\S]*?\*/").unwrap());
    COMMENT_RE.replace_all(value, "").to_string()
}

/// Visitor implementation that converts styles to attributes
struct ConvertStyleToAttrsVisitor {
    config: ConvertStyleToAttrsConfig,
}

impl ConvertStyleToAttrsVisitor {
    fn new(config: ConvertStyleToAttrsConfig) -> Self {
        Self { config }
    }

    /// Convert style attribute to presentation attributes for an element
    fn convert_element_styles(&self, element: &mut Element) {
        // Check if element has a style attribute
        if let Some(style_value) = element.attributes.get("style").cloned() {
            let mut remaining_styles = Vec::new();
            let mut new_attributes = Vec::new();

            // Parse CSS declarations
            for cap in CSS_DECLARATION_RE.captures_iter(&style_value) {
                if let (Some(prop_match), Some(value_match)) = (cap.get(1), cap.get(2)) {
                    let property = prop_match.as_str().trim();
                    let cleaned_value = strip_css_comments(value_match.as_str());
                    let value = cleaned_value.trim();
                    let has_important = cap.get(3).is_some();

                    // Check if this is a presentation attribute
                    if PRESENTATION_ATTRS.contains(&property) {
                        // Handle !important declarations
                        if has_important {
                            if self.config.keep_important {
                                // Keep in style if it has !important and keepImportant is true
                                remaining_styles.push(format!("{}: {} !important", property, value));
                            }
                            // Otherwise drop it entirely (don't convert to attribute)
                        } else {
                            // No !important, convert to attribute if doesn't already exist
                            if !element.attributes.contains_key(property) {
                                new_attributes.push((property.to_string(), value.to_string()));
                            } else {
                                // Keep in style if attribute already exists
                                remaining_styles.push(format!("{}: {}", property, value));
                            }
                        }
                    } else {
                        // Not a presentation attribute, keep in style
                        if has_important {
                            remaining_styles.push(format!("{}: {} !important", property, value));
                        } else {
                            remaining_styles.push(format!("{}: {}", property, value));
                        }
                    }
                } else if cap.get(0).is_some() {
                    // Capture other content (like comments) to preserve
                    let content = cap.get(0).unwrap().as_str().trim();
                    if !content.is_empty() && !content.starts_with("/*") {
                        remaining_styles.push(content.to_string());
                    }
                }
            }

            // Add new attributes
            for (name, value) in new_attributes {
                element.attributes.insert(name.into(), value.into());
            }

            // Update or remove style attribute
            if remaining_styles.is_empty() {
                element.attributes.shift_remove("style");
            } else {
                element
                    .attributes
                    .insert("style".into(), remaining_styles.join("; ").into());
            }
        }
    }
}

impl Visitor<'_> for ConvertStyleToAttrsVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        self.convert_element_styles(element);
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
        let plugin = ConvertStyleToAttrsPlugin::new();
        assert_eq!(plugin.name(), "convertStyleToAttrs");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = ConvertStyleToAttrsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({"keepImportant": true}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"keepImportant": false}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"keepImportant": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"unknownParam": true}))
            .is_err());
    }

    #[test]
    fn test_strip_css_comments() {
        assert_eq!(strip_css_comments("red /* comment */"), "red ");
        assert_eq!(strip_css_comments("/* start */ blue /* end */"), " blue ");
        assert_eq!(strip_css_comments("green"), "green");
    }
    
    #[test]
    fn test_css_declaration_regex() {
        let style = "fill: red !important; stroke: blue";
        let mut captures = Vec::new();
        for cap in CSS_DECLARATION_RE.captures_iter(style) {
            let prop = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let val = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let imp = cap.get(3).is_some();
            captures.push((prop, val, imp));
        }
        assert_eq!(captures.len(), 2);
        assert_eq!(captures[0], ("fill", "red", true));
        assert_eq!(captures[1], ("stroke", "blue", false));
    }

    #[test]
    fn test_convert_basic_styles() {
        let plugin = ConvertStyleToAttrsPlugin::new();
        let mut doc = Document::new();

        // Add element with style
        let mut rect = create_element("rect");
        rect.set_attr("style", "fill: red; stroke: blue; opacity: 0.5");
        rect.set_attr("width", "100");

        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Check that styles were converted to attributes
        if let Some(Node::Element(rect)) = doc.root.children.get(0) {
            assert_eq!(rect.attr("fill"), Some("red"));
            assert_eq!(rect.attr("stroke"), Some("blue"));
            assert_eq!(rect.attr("opacity"), Some("0.5"));
            assert!(!rect.attributes.contains_key("style"));
        }
    }

    #[test]
    fn test_preserve_existing_attributes() {
        let plugin = ConvertStyleToAttrsPlugin::new();
        let mut doc = Document::new();

        // Add element with style and existing attribute
        let mut rect = create_element("rect");
        rect.set_attr("style", "fill: red; stroke: blue");
        rect.set_attr("fill", "green");

        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Check that existing attribute is preserved
        if let Some(Node::Element(rect)) = doc.root.children.get(0) {
            assert_eq!(rect.attr("fill"), Some("green"));
            assert_eq!(rect.attr("stroke"), Some("blue"));
            assert_eq!(rect.attr("style"), Some("fill: red"));
        }
    }

    #[test]
    fn test_non_presentation_attributes() {
        let plugin = ConvertStyleToAttrsPlugin::new();
        let mut doc = Document::new();

        // Add element with non-presentation CSS properties
        let mut circle = create_element("circle");
        circle.set_attr("style", "fill: green; custom-prop: value; -webkit-something: test");

        doc.root.children.push(Node::Element(circle));

        plugin.apply(&mut doc).unwrap();

        // Check that only presentation attributes were converted
        if let Some(Node::Element(circle)) = doc.root.children.get(0) {
            assert_eq!(circle.attr("fill"), Some("green"));
            assert_eq!(circle.attr("style"), Some("custom-prop: value; -webkit-something: test"));
        }
    }

    #[test]
    fn test_important_declarations() {
        let plugin = ConvertStyleToAttrsPlugin::new();
        let mut doc = Document::new();

        // Add element with !important declarations
        let mut rect = create_element("rect");
        rect.set_attr("style", "fill: red !important; stroke: blue");

        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Check that !important declaration was dropped (default behavior)
        if let Some(Node::Element(rect)) = doc.root.children.get(0) {
            assert!(!rect.attributes.contains_key("fill"));
            assert_eq!(rect.attr("stroke"), Some("blue"));
            assert!(!rect.attributes.contains_key("style"));
        }
    }

    #[test]
    fn test_keep_important() {
        let config = ConvertStyleToAttrsConfig {
            keep_important: true,
        };
        let plugin = ConvertStyleToAttrsPlugin::with_config(config);
        let mut doc = Document::new();

        // Add element with !important declarations
        let mut rect = create_element("rect");
        rect.set_attr("style", "fill: red !important; stroke: blue");

        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Check that !important declaration was kept in style
        if let Some(Node::Element(rect)) = doc.root.children.get(0) {
            assert!(!rect.attributes.contains_key("fill"));
            assert_eq!(rect.attr("stroke"), Some("blue"));
            assert_eq!(rect.attr("style"), Some("fill: red !important"));
        }
    }

    #[test]
    fn test_css_comments() {
        let plugin = ConvertStyleToAttrsPlugin::new();
        let mut doc = Document::new();

        // Add element with CSS comments in style
        let mut rect = create_element("rect");
        rect.set_attr("style", "fill: /* comment */ red; stroke: blue /* another comment */");

        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Check that comments were stripped and styles converted
        if let Some(Node::Element(rect)) = doc.root.children.get(0) {
            assert_eq!(rect.attr("fill"), Some("red"));
            assert_eq!(rect.attr("stroke"), Some("blue"));
            assert!(!rect.attributes.contains_key("style"));
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = ConvertStyleToAttrsPlugin::parse_config(&json!({
            "keepImportant": true
        }))
        .unwrap();

        assert_eq!(config.keep_important, true);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(ConvertStyleToAttrsPlugin, "convertStyleToAttrs");
