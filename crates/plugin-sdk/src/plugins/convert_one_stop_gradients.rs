// this_file: crates/plugin-sdk/src/plugins/convert_one_stop_gradients.rs

//! Converts one-stop (single color) gradients to a plain color
//!
//! This plugin identifies linear and radial gradients that contain only one stop
//! and replaces all references to these gradients with the solid color from that stop.
//! It also removes the gradient definitions and any empty defs elements that result.
//!
//! Reference: SVGO's convertOneStopGradients plugin

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::collections::COLORS_PROPS;

/// Configuration for the convertOneStopGradients plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertOneStopGradientsConfig {}

impl Default for ConvertOneStopGradientsConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Main plugin struct
pub struct ConvertOneStopGradientsPlugin {
    #[allow(dead_code)]
    config: ConvertOneStopGradientsConfig,
}

impl ConvertOneStopGradientsPlugin {
    pub fn new() -> Self {
        Self {
            config: ConvertOneStopGradientsConfig::default(),
        }
    }

    pub fn with_config(config: ConvertOneStopGradientsConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<ConvertOneStopGradientsConfig> {
        if params.is_null() {
            Ok(ConvertOneStopGradientsConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn process_element(
        &self,
        element: &mut Element,
        gradients_to_remove: &mut HashMap<String, String>,
        parent_is_defs: bool,
        affected_defs: &mut HashSet<String>,
    ) {
        // Track defs elements
        if element.name == "defs" && element.has_attr("id") {
            if let Some(id) = element.attr("id") {
                affected_defs.insert(id.to_string());
            }
        }

        // Process gradient elements
        if element.name == "linearGradient" || element.name == "radialGradient" {
            if let Some(id) = element.attr("id") {
                // Count stop elements
                let stops: Vec<&Element> = element
                    .children
                    .iter()
                    .filter_map(|child| {
                        if let Node::Element(ref elem) = child {
                            if elem.name == "stop" {
                                Some(elem)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // Check if this gradient references another gradient
                let href = element.attr("xlink:href").or_else(|| element.attr("href"));

                // If this gradient has no stops and references another, skip for now
                // (would need to resolve references, which is complex)
                if stops.is_empty() && href.is_some() {
                    return;
                }

                // Only process gradients with exactly one stop
                if stops.len() == 1 {
                    let stop = stops[0];

                    // Get the stop color
                    let stop_color = stop
                        .attr("stop-color")
                        .map(|s| s.to_string())
                        .or_else(|| {
                            // Check style attribute for stop-color
                            stop.attr("style").and_then(|style| {
                                // Simple regex-like parsing for stop-color in style
                                if let Some(idx) = style.find("stop-color:") {
                                    let start = idx + 11;
                                    let rest = &style[start..].trim_start();
                                    let end = rest.find(';').unwrap_or(rest.len());
                                    Some(rest[..end].trim().to_string())
                                } else {
                                    None
                                }
                            })
                        })
                        .unwrap_or_else(|| "black".to_string()); // Default stop-color is black

                    // Mark this gradient for removal and store its replacement color
                    gradients_to_remove.insert(id.to_string(), stop_color);

                    if parent_is_defs {
                        affected_defs.insert("parent_defs".to_string());
                    }
                }
            }
        }

        // Process child elements recursively
        let is_defs = element.name == "defs";
        for child in &mut element.children {
            if let Node::Element(ref mut child_elem) = child {
                self.process_element(child_elem, gradients_to_remove, is_defs, affected_defs);
            }
        }
    }

    fn replace_gradient_references(
        &self,
        element: &mut Element,
        gradients_to_remove: &HashMap<String, String>,
    ) {
        // Replace gradient references in color properties
        for color_prop in COLORS_PROPS.iter() {
            if let Some(value) = element.attr(*color_prop) {
                if let Some(gradient_id) = self.extract_gradient_id(&value) {
                    if let Some(replacement_color) = gradients_to_remove.get(&gradient_id) {
                        element.set_attr(*color_prop, replacement_color);
                    }
                }
            }
        }

        // Replace gradient references in style attribute
        if let Some(style) = element.attr("style") {
            let mut new_style = style.to_string();
            for (gradient_id, replacement_color) in gradients_to_remove {
                let url_pattern = format!("url(#{})", gradient_id);
                new_style = new_style.replace(&url_pattern, replacement_color);
            }
            if new_style != style.as_ref() {
                element.set_attr("style", &new_style);
            }
        }

        // Process children
        for child in &mut element.children {
            if let Node::Element(ref mut child_elem) = child {
                self.replace_gradient_references(child_elem, gradients_to_remove);
            }
        }
    }

    fn extract_gradient_id(&self, value: &str) -> Option<String> {
        if value.starts_with("url(#") && value.ends_with(')') {
            let id = &value[5..value.len() - 1];
            Some(id.to_string())
        } else {
            None
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn remove_gradients(
        &self,
        element: &mut Element,
        gradients_to_remove: &HashMap<String, String>,
    ) {
        // Remove gradient elements
        element.children.retain(|child| {
            if let Node::Element(ref elem) = child {
                if elem.name == "linearGradient" || elem.name == "radialGradient" {
                    if let Some(id) = elem.attr("id") {
                        return !gradients_to_remove.contains_key(id);
                    }
                }
            }
            true
        });

        // Process children
        for child in &mut element.children {
            if let Node::Element(ref mut child_elem) = child {
                self.remove_gradients(child_elem, gradients_to_remove);
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn remove_empty_defs(&self, element: &mut Element) {
        // Remove empty defs elements
        element.children.retain(|child| {
            if let Node::Element(ref elem) = child {
                if elem.name == "defs" && elem.children.is_empty() {
                    return false;
                }
            }
            true
        });

        // Process children
        for child in &mut element.children {
            if let Node::Element(ref mut child_elem) = child {
                self.remove_empty_defs(child_elem);
            }
        }
    }

    fn remove_unused_xlink_namespace(&self, document: &mut Document) {
        // Check if any xlink:href attributes remain
        fn check_xlink(element: &Element) -> bool {
            if element.has_attr("xlink:href") {
                return true;
            }

            for child in &element.children {
                if let Node::Element(ref elem) = child {
                    if check_xlink(elem) {
                        return true;
                    }
                }
            }
            false
        }

        let has_xlink = check_xlink(&document.root);

        // Remove xmlns:xlink if no xlink:href attributes remain
        if !has_xlink {
            document.root.namespaces.shift_remove("xlink");
            document.root.remove_attr("xmlns:xlink");
        }
    }
}

impl Default for ConvertOneStopGradientsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ConvertOneStopGradientsPlugin {
    fn name(&self) -> &'static str {
        "convertOneStopGradients"
    }

    fn description(&self) -> &'static str {
        "converts one-stop (single color) gradients to a plain color"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        let mut gradients_to_remove = HashMap::new();
        let mut affected_defs = HashSet::new();

        // First pass: identify gradients with only one stop
        self.process_element(
            &mut document.root,
            &mut gradients_to_remove,
            false,
            &mut affected_defs,
        );

        // Second pass: replace gradient references with solid colors
        if !gradients_to_remove.is_empty() {
            self.replace_gradient_references(&mut document.root, &gradients_to_remove);

            // Third pass: remove the gradient elements
            self.remove_gradients(&mut document.root, &gradients_to_remove);

            // Fourth pass: remove empty defs elements
            self.remove_empty_defs(&mut document.root);

            // Remove unused xlink namespace
            self.remove_unused_xlink_namespace(document);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use vexy_svgo_core::ast::{Document, Element, Node};

    fn create_test_document() -> Document<'static> {
        use std::collections::HashMap;
        Document {
            root: Element {
                name: "svg".into(),
                attributes: IndexMap::new(),
                namespaces: IndexMap::new(),
                children: vec![],
            },
            prologue: vec![],
            epilogue: vec![],
            metadata: vexy_svgo_core::ast::DocumentMetadata {
                path: None,
                encoding: None,
                version: None,
            },
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = ConvertOneStopGradientsPlugin::new();
        assert_eq!(plugin.name(), "convertOneStopGradients");
        assert_eq!(
            plugin.description(),
            "converts one-stop (single color) gradients to a plain color"
        );
    }

    #[test]
    fn test_param_validation() {
        let plugin = ConvertOneStopGradientsPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test empty object params
        assert!(plugin.validate_params(&serde_json::json!({})).is_ok());

        // Test invalid params
        assert!(plugin
            .validate_params(&serde_json::json!({
                "invalidParam": true
            }))
            .is_err());
    }

    #[test]
    fn test_extract_gradient_id() {
        let plugin = ConvertOneStopGradientsPlugin::new();

        // Test valid gradient ID extraction
        assert_eq!(
            plugin.extract_gradient_id("url(#myGradient)"),
            Some("myGradient".to_string())
        );
        assert_eq!(
            plugin.extract_gradient_id("url(#grad1)"),
            Some("grad1".to_string())
        );

        // Test invalid formats
        assert_eq!(plugin.extract_gradient_id("red"), None);
        assert_eq!(plugin.extract_gradient_id("url(myGradient)"), None);
        assert_eq!(plugin.extract_gradient_id("#myGradient"), None);
    }

    #[test]
    fn test_apply_with_empty_document() {
        let plugin = ConvertOneStopGradientsPlugin::new();
        let mut doc = create_test_document();

        // Should not panic with empty document
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_with_no_gradients() {
        let plugin = ConvertOneStopGradientsPlugin::new();
        let mut doc = create_test_document();

        // Add a simple rect element
        let mut rect = Element {
            name: "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };
        rect.set_attr("fill", "red");
        rect.set_attr("width", "100");
        rect.set_attr("height", "100");
        doc.root.children.push(Node::Element(rect));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Document should remain unchanged
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(rect) = &doc.root.children[0] {
            assert_eq!(rect.name, "rect");
            assert_eq!(rect.attr("fill"), Some("red"));
        }
    }

    #[test]
    fn test_one_stop_gradient_conversion() {
        let plugin = ConvertOneStopGradientsPlugin::new();
        let mut doc = create_test_document();

        // Add defs with a one-stop gradient
        let mut defs_elem = Element {
            name: "defs".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children: vec![],
        };

        let mut gradient_attrs = IndexMap::new();
        gradient_attrs.insert(Cow::Borrowed("id"), Cow::Borrowed("grad1"));

        let mut stop_attrs = IndexMap::new();
        stop_attrs.insert(Cow::Borrowed("stop-color"), Cow::Borrowed("#ff0000"));

        let gradient_elem = Element {
            name: "linearGradient".into(),
            attributes: gradient_attrs,
            namespaces: IndexMap::new(),
            children: vec![Node::Element(Element {
                name: "stop".into(),
                attributes: stop_attrs,
                namespaces: IndexMap::new(),
                children: vec![],
            })],
        };

        defs_elem.children.push(Node::Element(gradient_elem));
        doc.root.children.push(Node::Element(defs_elem));

        // Add rect using the gradient
        let mut rect_attrs = IndexMap::new();
        rect_attrs.insert(Cow::Borrowed("fill"), Cow::Borrowed("url(#grad1)"));
        rect_attrs.insert(Cow::Borrowed("width"), Cow::Borrowed("100"));
        rect_attrs.insert(Cow::Borrowed("height"), Cow::Borrowed("100"));

        doc.root.children.push(Node::Element(Element {
            name: "rect".into(),
            attributes: rect_attrs,
            namespaces: IndexMap::new(),
            children: vec![],
        }));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that gradient was removed and rect now has solid fill
        // The defs should be empty and removed
        let has_gradient = doc.root.children.iter().any(|child| {
            if let Node::Element(elem) = child {
                elem.children.iter().any(|child| {
                    if let Node::Element(e) = child {
                        e.name == "linearGradient" || e.name == "radialGradient"
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        });
        assert!(!has_gradient);

        // Find the rect and check its fill
        let rect = doc.root.children.iter().find_map(|child| {
            if let Node::Element(elem) = child {
                if elem.name == "rect" {
                    Some(elem)
                } else {
                    None
                }
            } else {
                None
            }
        });

        assert!(rect.is_some());
        let rect = rect.unwrap();
        assert_eq!(rect.attr("fill"), Some("#ff0000"));
    }
}
