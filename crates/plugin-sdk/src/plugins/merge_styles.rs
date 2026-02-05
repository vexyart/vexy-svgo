// this_file: crates/plugin-sdk/src/plugins/merge_styles.rs

//! Merge styles plugin implementation
//!
//! This plugin merges multiple `<style>` elements into one for improved performance
//! and reduced file size. It handles media queries by wrapping content in @media rules
//! and removes empty style elements.
//!
//! SVGO parameters supported: None (this plugin takes no configuration)

use crate::Plugin;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration parameters for merge styles plugin (currently empty)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeStylesConfig {
    // No configuration options - matches SVGO behavior
}

impl Default for MergeStylesConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin that merges multiple style elements into one
pub struct MergeStylesPlugin {
    #[allow(dead_code)]
    config: MergeStylesConfig,
}

impl MergeStylesPlugin {
    /// Create a new MergeStylesPlugin
    pub fn new() -> Self {
        Self {
            config: MergeStylesConfig::default(),
        }
    }

    /// Create a new MergeStylesPlugin with config
    pub fn with_config(config: MergeStylesConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    #[allow(dead_code)]
    fn parse_config(params: &Value) -> Result<MergeStylesConfig> {
        if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(MergeStylesConfig::default())
        }
    }
}

impl Default for MergeStylesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MergeStylesPlugin {
    fn name(&self) -> &'static str {
        "mergeStyles"
    }

    fn description(&self) -> &'static str {
        "merge multiple style elements into one"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            if !obj.is_empty() {
                return Err(anyhow::anyhow!(
                    "mergeStyles plugin does not accept any parameters"
                ));
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Use a simple recursive approach instead of visitor pattern for this plugin
        self.merge_styles_recursive(&mut document.root);
        Ok(())
    }
}

impl MergeStylesPlugin {
    /// Recursively merge styles in an element and all its children
    fn merge_styles_recursive(&self, element: &mut Element) {
        // Skip foreignObject content
        if element.name == "foreignObject" {
            return;
        }

        // Process children first (post-order)
        for child in &mut element.children {
            if let Node::Element(child_element) = child {
                self.merge_styles_recursive(child_element);
            }
        }

        // Now merge styles in this element
        self.merge_styles_in_element(element);
    }

    /// Merge style elements within a single parent element
    fn merge_styles_in_element(&self, element: &mut Element) {
        let mut style_data = Vec::new();
        let mut first_style_index = None;
        let mut uses_cdata = false;

        // First pass: collect style elements and their data
        for (index, child) in element.children.iter().enumerate() {
            if let Node::Element(elem) = child {
                if elem.name == "style" && Self::is_valid_style_type(elem) {
                    let css = Self::extract_css_content(elem);

                    // Skip empty styles
                    if css.trim().is_empty() {
                        continue;
                    }

                    // Track if we need CDATA
                    if Self::has_cdata_content(elem) {
                        uses_cdata = true;
                    }

                    // Get media attribute
                    let media = elem.attributes.get("media").cloned();

                    // Save data for merging
                    style_data.push((css, media));

                    // Remember the first style element index
                    if first_style_index.is_none() {
                        first_style_index = Some(index);
                    }
                }
            }
        }

        // If we have styles to merge
        if style_data.len() > 1 {
            // Build merged CSS content
            let mut merged_css = String::new();
            for (css, media) in &style_data {
                if let Some(media_value) = media {
                    merged_css.push_str(&format!("@media {}{{{}}}", media_value, css));
                } else {
                    merged_css.push_str(css);
                }
            }

            // Replace the first style element with merged content
            if let Some(first_index) = first_style_index {
                let mut merged_style = Element::new("style");
                merged_style.name = std::borrow::Cow::Borrowed("style");

                // Add merged content as appropriate node type
                if uses_cdata {
                    merged_style.children.push(Node::CData(merged_css));
                } else {
                    merged_style.children.push(Node::Text(merged_css));
                }

                // Replace first style element
                element.children[first_index] = Node::Element(merged_style);
            }
        }

        // Second pass: remove empty styles and duplicates (keep only first merged style)
        if style_data.len() > 1 {
            let mut found_first = false;
            element.children.retain(|child| {
                if let Node::Element(elem) = child {
                    if elem.name == "style" && Self::is_valid_style_type(elem) {
                        if !found_first {
                            found_first = true;
                            true // Keep the first style element
                        } else {
                            false // Remove subsequent style elements
                        }
                    } else {
                        true // Keep non-style elements
                    }
                } else {
                    true // Keep non-element nodes
                }
            });
        } else {
            // Just remove empty styles when no merging occurred
            element.children.retain(|child| {
                if let Node::Element(elem) = child {
                    if elem.name == "style" && Self::is_valid_style_type(elem) {
                        let css = Self::extract_css_content(elem);
                        !css.trim().is_empty()
                    } else {
                        true // Keep non-style elements
                    }
                } else {
                    true // Keep non-element nodes
                }
            });
        }
    }

    /// Check if style element has valid type attribute
    fn is_valid_style_type(element: &Element) -> bool {
        if let Some(type_attr) = element.attributes.get("type") {
            type_attr.is_empty() || type_attr == "text/css"
        } else {
            true // No type attribute is valid
        }
    }

    /// Extract CSS content from style element
    fn extract_css_content(element: &Element) -> String {
        let mut css = String::new();
        for child in &element.children {
            match child {
                Node::Text(text) => css.push_str(text),
                Node::CData(cdata) => css.push_str(cdata),
                _ => {}
            }
        }
        css
    }

    /// Check if any child has CDATA content
    fn has_cdata_content(element: &Element) -> bool {
        element
            .children
            .iter()
            .any(|child| matches!(child, Node::CData(_)))
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

    fn count_style_elements(element: &Element) -> usize {
        let mut count = 0;
        for child in &element.children {
            if let Node::Element(elem) = child {
                if elem.name == "style" {
                    count += 1;
                }
                count += count_style_elements(elem);
            }
        }
        count
    }

    fn get_style_content(element: &Element) -> Option<String> {
        for child in &element.children {
            if let Node::Element(elem) = child {
                if elem.name == "style" {
                    for style_child in &elem.children {
                        match style_child {
                            Node::Text(text) => return Some(text.clone()),
                            Node::CData(cdata) => return Some(cdata.clone()),
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = MergeStylesPlugin::new();
        assert_eq!(plugin.name(), "mergeStyles");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = MergeStylesPlugin::new();

        // Valid parameters (empty object)
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Invalid parameters (non-empty object)
        assert!(plugin.validate_params(&json!({"param": "value"})).is_err());
    }

    #[test]
    fn test_merge_multiple_styles() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create first style element
        let mut style1 = create_element("style");
        style1.children.push(Node::Text(".a{fill:red}".to_string()));

        // Create second style element
        let mut style2 = create_element("style");
        style2
            .children
            .push(Node::Text(".b{fill:blue}".to_string()));

        // Add both styles to document
        doc.root.children.push(Node::Element(style1));
        doc.root.children.push(Node::Element(style2));

        plugin.apply(&mut doc).unwrap();

        // Should have only one style element
        assert_eq!(count_style_elements(&doc.root), 1);

        // Content should be merged
        let merged_content = get_style_content(&doc.root).unwrap();
        assert!(merged_content.contains(".a{fill:red}"));
        assert!(merged_content.contains(".b{fill:blue}"));
    }

    #[test]
    fn test_merge_styles_with_media() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create style with media attribute
        let mut style1 = create_element("style");
        style1.set_attr("media", "print");
        style1.children.push(Node::Text(".a{fill:red}".to_string()));

        // Create regular style
        let mut style2 = create_element("style");
        style2
            .children
            .push(Node::Text(".b{fill:blue}".to_string()));

        // Add both styles to document
        doc.root.children.push(Node::Element(style1));
        doc.root.children.push(Node::Element(style2));

        plugin.apply(&mut doc).unwrap();

        // Should have only one style element
        assert_eq!(count_style_elements(&doc.root), 1);

        // Content should include @media wrapper
        let merged_content = get_style_content(&doc.root).unwrap();
        assert!(merged_content.contains("@media print{.a{fill:red}}"));
        assert!(merged_content.contains(".b{fill:blue}"));
    }

    #[test]
    fn test_remove_empty_styles() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create empty style
        let mut empty_style = create_element("style");
        empty_style.children.push(Node::Text("   ".to_string()));

        // Create style with content
        let mut content_style = create_element("style");
        content_style
            .children
            .push(Node::Text(".a{fill:red}".to_string()));

        // Add both styles to document
        doc.root.children.push(Node::Element(empty_style));
        doc.root.children.push(Node::Element(content_style));

        plugin.apply(&mut doc).unwrap();

        // Should have only one style element (empty one removed)
        assert_eq!(count_style_elements(&doc.root), 1);

        // Content should only be the non-empty style
        let merged_content = get_style_content(&doc.root).unwrap();
        assert_eq!(merged_content, ".a{fill:red}");
    }

    #[test]
    fn test_skip_invalid_type_attribute() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create style with invalid type
        let mut invalid_style = create_element("style");
        invalid_style.set_attr("type", "text/javascript");
        invalid_style
            .children
            .push(Node::Text("console.log('test')".to_string()));

        // Create valid style
        let mut valid_style = create_element("style");
        valid_style
            .children
            .push(Node::Text(".a{fill:red}".to_string()));

        // Add both styles to document
        doc.root.children.push(Node::Element(invalid_style));
        doc.root.children.push(Node::Element(valid_style));

        plugin.apply(&mut doc).unwrap();

        // Should have two style elements (invalid one not merged)
        assert_eq!(count_style_elements(&doc.root), 2);
    }

    #[test]
    fn test_valid_type_attributes() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create style with empty type
        let mut style1 = create_element("style");
        style1.set_attr("type", "");
        style1.children.push(Node::Text(".a{fill:red}".to_string()));

        // Create style with text/css type
        let mut style2 = create_element("style");
        style2.set_attr("type", "text/css");
        style2
            .children
            .push(Node::Text(".b{fill:blue}".to_string()));

        // Create style with no type attribute
        let mut style3 = create_element("style");
        style3
            .children
            .push(Node::Text(".c{fill:green}".to_string()));

        // Add all styles to document
        doc.root.children.push(Node::Element(style1));
        doc.root.children.push(Node::Element(style2));
        doc.root.children.push(Node::Element(style3));

        plugin.apply(&mut doc).unwrap();

        // Should have only one style element (all merged)
        assert_eq!(count_style_elements(&doc.root), 1);

        // Content should include all styles
        let merged_content = get_style_content(&doc.root).unwrap();
        assert!(merged_content.contains(".a{fill:red}"));
        assert!(merged_content.contains(".b{fill:blue}"));
        assert!(merged_content.contains(".c{fill:green}"));
    }

    #[test]
    fn test_cdata_content_type() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create style with CDATA content
        let mut style1 = create_element("style");
        style1
            .children
            .push(Node::CData(".a{fill:red}".to_string()));

        // Create style with text content
        let mut style2 = create_element("style");
        style2
            .children
            .push(Node::Text(".b{fill:blue}".to_string()));

        // Add both styles to document
        doc.root.children.push(Node::Element(style1));
        doc.root.children.push(Node::Element(style2));

        plugin.apply(&mut doc).unwrap();

        // Should have only one style element
        assert_eq!(count_style_elements(&doc.root), 1);

        // Check that result uses CDATA (since one of the sources was CDATA)
        for child in &doc.root.children {
            if let Node::Element(elem) = child {
                if elem.name == "style" {
                    // Should have CDATA content
                    assert!(elem.children.iter().any(|c| matches!(c, Node::CData(_))));
                }
            }
        }
    }

    #[test]
    fn test_no_styles_to_merge() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create a rect element (no styles)
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Should have no style elements
        assert_eq!(count_style_elements(&doc.root), 0);
    }

    #[test]
    fn test_single_style_unchanged() {
        let plugin = MergeStylesPlugin::new();
        let mut doc = Document::new();

        // Create single style element
        let mut style = create_element("style");
        style.children.push(Node::Text(".a{fill:red}".to_string()));
        doc.root.children.push(Node::Element(style));

        plugin.apply(&mut doc).unwrap();

        // Should still have one style element
        assert_eq!(count_style_elements(&doc.root), 1);

        // Content should be unchanged
        let content = get_style_content(&doc.root).unwrap();
        assert_eq!(content, ".a{fill:red}");
    }

    #[test]
    fn test_config_parsing() {
        let config = MergeStylesPlugin::parse_config(&json!({})).unwrap();
        // No fields to check since config is empty
        let _ = config;
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(MergeStylesPlugin, "mergeStyles");
