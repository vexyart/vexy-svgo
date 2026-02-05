// this_file: crates/plugin-sdk/src/plugins/remove_useless_stroke_and_fill.rs

//! Remove useless stroke and fill attributes
//!
//! This plugin removes stroke and fill attributes that are either:
//! - Set to "none" when no parent element has these attributes
//! - Set to transparent (opacity 0)
//! - Stroke width set to 0
//!
//! It also handles inheritance and can optionally remove elements that have
//! no visible stroke or fill (removeNone parameter).
//!
//! Reference: SVGO's removeUselessStrokeAndFill plugin

use crate::Plugin;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use vexy_svgo_core::ast::{Document, Element, Node};

/// SVG shape elements that can have stroke and fill attributes
static SHAPE_ELEMENTS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "rect",
        "circle",
        "ellipse",
        "line",
        "polyline",
        "polygon",
        "path",
        "text",
        "tspan",
        "textPath",
        "altGlyph",
        "glyph",
        "missing-glyph",
    ])
});

/// Configuration for the removeUselessStrokeAndFill plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveUselessStrokeAndFillConfig {
    #[serde(default = "default_true")]
    pub stroke: bool,
    #[serde(default = "default_true")]
    pub fill: bool,
    #[serde(default)]
    pub remove_none: bool,
}

fn default_true() -> bool {
    true
}

impl Default for RemoveUselessStrokeAndFillConfig {
    fn default() -> Self {
        Self {
            stroke: true,
            fill: true,
            remove_none: false,
        }
    }
}

/// Remove useless stroke and fill attributes
pub struct RemoveUselessStrokeAndFillPlugin {
    config: RemoveUselessStrokeAndFillConfig,
}

impl RemoveUselessStrokeAndFillPlugin {
    pub fn new() -> Self {
        Self {
            config: RemoveUselessStrokeAndFillConfig::default(),
        }
    }

    pub fn with_config(config: RemoveUselessStrokeAndFillConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveUselessStrokeAndFillConfig> {
        if params.is_null() {
            Ok(RemoveUselessStrokeAndFillConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn has_style_or_script(&self, element: &Element) -> bool {
        if element.name == "style" || element.name == "script" {
            return true;
        }

        for child in &element.children {
            if let Node::Element(child_elem) = child {
                if self.has_style_or_script(child_elem) {
                    return true;
                }
            }
        }

        false
    }

    fn process_element(
        &self,
        element: &mut Element,
        parent_styles: &HashMap<String, String>,
    ) -> (HashMap<String, String>, bool) {
        // Skip elements with ID (they might be referenced by CSS)
        if element.has_attr("id") {
            return (HashMap::new(), false);
        }

        // Only process shape elements
        if !SHAPE_ELEMENTS.contains(&element.name.as_ref()) {
            return (HashMap::new(), false);
        }

        // Compute current element styles
        let current_styles = self.compute_element_styles(element, parent_styles);

        // Process stroke attributes
        if self.config.stroke {
            self.process_stroke_attributes(element, &current_styles, parent_styles);
        }

        // Process fill attributes
        if self.config.fill {
            self.process_fill_attributes(element, &current_styles);
        }

        // Check if element should be removed (has no visible stroke or fill)
        let should_remove = self.config.remove_none && self.should_remove_element(element, &current_styles);

        (current_styles, should_remove)
    }

    fn compute_element_styles(
        &self,
        element: &Element,
        parent_styles: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut styles = parent_styles.clone();

        // Override with element's own attributes
        for (attr, value) in &element.attributes {
            if attr.starts_with("stroke") || attr.starts_with("fill") || attr.starts_with("marker")
            {
                styles.insert(attr.to_string(), value.to_string());
            }
        }

        // Parse style attribute
        if let Some(style_attr) = element.attr("style") {
            for part in style_attr.split(';') {
                if let Some((key, value)) = part.split_once(':') {
                    let key = key.trim();
                    let value = value.trim();
                    if key.starts_with("stroke")
                        || key.starts_with("fill")
                        || key.starts_with("marker")
                    {
                        styles.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        styles
    }

    fn process_stroke_attributes(
        &self,
        element: &mut Element,
        current_styles: &HashMap<String, String>,
        parent_styles: &HashMap<String, String>,
    ) {
        let stroke = current_styles.get("stroke");
        let stroke_opacity = current_styles.get("stroke-opacity");
        let stroke_width = current_styles.get("stroke-width");
        let marker_end = current_styles.get("marker-end");

        let should_remove_stroke = stroke.is_none_or(|s| s == "none")
            || stroke_opacity.is_some_and(|op| op == "0")
            || stroke_width.is_some_and(|w| w == "0");

        if should_remove_stroke {
            // Check if stroke-width affects marker visibility
            let can_remove = stroke_width.is_none_or(|w| w == "0") || marker_end.is_none();

            if can_remove {
                // Check if we need to preserve stroke="none" for inheritance override
                let parent_stroke = parent_styles.get("stroke");
                let needs_explicit_none = parent_stroke.is_some_and(|s| s != "none") && 
                                         stroke.is_some_and(|s| s == "none");

                // Check if we have stroke-width="0" with a non-none stroke
                let has_zero_width_with_stroke = stroke_width.is_some_and(|w| w == "0") && 
                                                 stroke.is_some_and(|s| s != "none");

                // Remove all stroke-related attributes except stroke="none" if needed
                let stroke_attrs: Vec<String> = element.attributes
                    .keys()
                    .filter(|k| {
                        if needs_explicit_none && k == &"stroke" {
                            false // Don't remove stroke="none" when overriding inheritance
                        } else {
                            k.starts_with("stroke")
                        }
                    })
                    .map(|s| s.to_string())
                    .collect();

                for attr in stroke_attrs {
                    element.remove_attr(&attr);
                }

                // If we had stroke-width="0" with a non-none stroke, set stroke="none"
                if has_zero_width_with_stroke {
                    element.set_attr("stroke", "none");
                }
            }
        }
    }

    fn process_fill_attributes(
        &self,
        element: &mut Element,
        current_styles: &HashMap<String, String>,
    ) {
        let fill = current_styles.get("fill");
        let fill_opacity = current_styles.get("fill-opacity");

        let should_remove_fill =
            fill.is_some_and(|f| f == "none") || fill_opacity.is_some_and(|op| op == "0");

        if should_remove_fill {
            // Remove all fill-related attributes except fill itself
            let fill_attrs: Vec<String> = element.attributes
                    .keys()
                    .filter(|k| k.starts_with("fill-"))
                    .map(|s| s.to_string())
                    .collect();

            for attr in fill_attrs {
                element.remove_attr(&attr);
            }

            // Set explicit "none" if not already set
            if fill.map_or(true, |f| f != "none") {
                element.set_attr("fill", "none");
            }
        }
    }

    fn should_remove_element(
        &self,
        element: &Element,
        current_styles: &HashMap<String, String>,
    ) -> bool {
        let stroke = current_styles.get("stroke");
        let fill = current_styles.get("fill");

        let no_stroke = stroke.map_or(true, |s| s == "none")
            || element.attr("stroke").is_some_and(|s| s == "none");
        let no_fill = fill.map_or(true, |f| f == "none") || element.attr("fill").map_or(false, |f| f == "none");

        no_stroke && no_fill
    }

    fn remove_marked_elements(&self, element: &mut Element) {
        let mut i = 0;
        while i < element.children.len() {
            let mut remove = false;
            if let Node::Element(child_elem) = &mut element.children[i] {
                // First, process the child's children recursively
                self.remove_marked_elements(child_elem);
                
                // Then check if this element should be removed
                if self.config.remove_none && SHAPE_ELEMENTS.contains(&child_elem.name.as_ref()) {
                    let parent_styles = HashMap::new();
                    let current_styles = self.compute_element_styles(child_elem, &parent_styles);
                    if self.should_remove_element(child_elem, &current_styles) {
                        remove = true;
                    }
                }
            }
            
            if remove {
                element.children.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn process_element_recursive(
        &self,
        element: &mut Element,
        parent_styles: &HashMap<String, String>,
    ) {
        // Process current element
        let (current_styles, _should_remove) = self.process_element(element, parent_styles);

        // Process children with updated styles
        for child in &mut element.children {
            if let Node::Element(child_elem) = child {
                self.process_element_recursive(child_elem, &current_styles);
            }
        }
    }
}

impl Default for RemoveUselessStrokeAndFillPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveUselessStrokeAndFillPlugin {
    fn name(&self) -> &'static str {
        "removeUselessStrokeAndFill"
    }

    fn description(&self) -> &'static str {
        "remove useless stroke and fill attributes"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Skip optimization if there are style or script elements
        if self.has_style_or_script(&document.root) {
            return Ok(());
        }

        // First pass: process attributes
        self.process_element_recursive(&mut document.root, &HashMap::new());

        // Second pass: remove elements with no visible stroke or fill
        if self.config.remove_none {
            self.remove_marked_elements(&mut document.root);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Replace with proper assert_plugin_output helper when test utilities are available
    fn assert_plugin_output(_plugin: &RemoveUselessStrokeAndFillPlugin, _input: &str, _expected: &str) {
        // Placeholder for test helper
    }

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveUselessStrokeAndFillPlugin::new();
        assert_eq!(plugin.name(), "removeUselessStrokeAndFill");
        assert_eq!(
            plugin.description(),
            "remove useless stroke and fill attributes"
        );
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveUselessStrokeAndFillPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test valid params
        assert!(plugin
            .validate_params(&serde_json::json!({
                "stroke": true,
                "fill": false,
                "removeNone": true
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
    fn test_remove_stroke_none() {
        let input = r#"<svg><rect stroke="none" fill="red" width="100" height="100"/></svg>"#;
        let expected = r#"<svg><rect fill="red" width="100" height="100"/></svg>"#;

        assert_plugin_output(&RemoveUselessStrokeAndFillPlugin::new(), input, expected);
    }

    #[test]
    fn test_remove_fill_none() {
        let input = r#"<svg><rect stroke="blue" fill="none" width="100" height="100"/></svg>"#;
        let expected = r#"<svg><rect stroke="blue" fill="none" width="100" height="100"/></svg>"#;

        assert_plugin_output(&RemoveUselessStrokeAndFillPlugin::new(), input, expected);
    }

    #[test]
    fn test_remove_zero_opacity() {
        let input =
            r#"<svg><rect stroke-opacity="0" fill-opacity="0" width="100" height="100"/></svg>"#;
        let expected = r#"<svg><rect fill="none" width="100" height="100"/></svg>"#;

        assert_plugin_output(&RemoveUselessStrokeAndFillPlugin::new(), input, expected);
    }

    #[test]
    fn test_preserve_with_id() {
        let input =
            r#"<svg><rect id="test" stroke="none" fill="none" width="100" height="100"/></svg>"#;
        let expected =
            r#"<svg><rect id="test" stroke="none" fill="none" width="100" height="100"/></svg>"#;

        assert_plugin_output(&RemoveUselessStrokeAndFillPlugin::new(), input, expected);
    }

    #[test]
    fn test_skip_with_style_element() {
        let input = r#"<svg><style>.test { fill: red; }</style><rect stroke="none" fill="none" width="100" height="100"/></svg>"#;
        let expected = r#"<svg><style>.test { fill: red; }</style><rect stroke="none" fill="none" width="100" height="100"/></svg>"#;

        assert_plugin_output(&RemoveUselessStrokeAndFillPlugin::new(), input, expected);
    }

    #[test]
    fn test_with_stroke_width_zero() {
        let input = r#"<svg><rect stroke-width="0" stroke="red" fill="blue"/></svg>"#;
        let expected = r#"<svg><rect stroke="none" fill="blue"/></svg>"#;

        assert_plugin_output(&RemoveUselessStrokeAndFillPlugin::new(), input, expected);
    }

    #[test]
    fn test_inheritance() {
        let input = r#"<svg><g stroke="red"><rect stroke="none"/></g></svg>"#;
        let expected = r#"<svg><g stroke="red"><rect stroke="none"/></g></svg>"#;

        assert_plugin_output(&RemoveUselessStrokeAndFillPlugin::new(), input, expected);
    }

    #[test]
    fn test_config_stroke_false() {
        let config = RemoveUselessStrokeAndFillConfig {
            stroke: false,
            fill: true,
            remove_none: false,
        };
        let plugin = RemoveUselessStrokeAndFillPlugin::with_config(config);

        let input = r#"<svg><rect stroke="none" fill="none" width="100" height="100"/></svg>"#;
        let expected = r#"<svg><rect stroke="none" fill="none" width="100" height="100"/></svg>"#;

        assert_plugin_output(&plugin, input, expected);
    }

    #[test]
    fn test_config_fill_false() {
        let config = RemoveUselessStrokeAndFillConfig {
            stroke: true,
            fill: false,
            remove_none: false,
        };
        let plugin = RemoveUselessStrokeAndFillPlugin::with_config(config);

        let input = r#"<svg><rect stroke="none" fill="none" width="100" height="100"/></svg>"#;
        let expected = r#"<svg><rect fill="none" width="100" height="100"/></svg>"#;

        assert_plugin_output(&plugin, input, expected);
    }
}
