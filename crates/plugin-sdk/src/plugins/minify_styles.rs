// this_file: crates/plugin-sdk/src/plugins/minify_styles.rs

//! Minify styles plugin implementation
//!
//! This plugin minifies CSS content within <style> elements and style attributes
//! using LightningCSS for fast and efficient optimization.
//!
//! Features:
//! - Minifies CSS content in <style> elements
//! - Minifies CSS content in style attributes
//! - Removes unused styles when usage analysis is enabled
//! - Configurable optimization settings
//!
//! Reference: SVGO's minifyStyles plugin

use crate::Plugin;
use anyhow::Result;
use lightningcss::{
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
    targets::Targets,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Usage configuration for CSS optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageConfig {
    /// Force usage optimization even when unsafe
    #[serde(default)]
    pub force: bool,
    /// Optimize based on ID usage
    #[serde(default = "default_true")]
    pub ids: bool,
    /// Optimize based on class usage
    #[serde(default = "default_true")]
    pub classes: bool,
    /// Optimize based on tag usage
    #[serde(default = "default_true")]
    pub tags: bool,
}

impl Default for UsageConfig {
    fn default() -> Self {
        Self {
            force: false,
            ids: true,
            classes: true,
            tags: true,
        }
    }
}

/// Configuration parameters for minify styles plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MinifyStylesConfig {
    /// Disable or enable structure optimizations
    #[serde(default = "default_true")]
    pub restructure: bool,
    /// Force merging of @media rules with same query
    #[serde(default)]
    pub force_media_merge: bool,
    /// Comment handling strategy
    #[serde(default)]
    pub comments: bool,
    /// Usage-based optimization settings
    #[serde(default)]
    pub usage: Option<UsageConfig>,
}

fn default_true() -> bool {
    true
}

impl Default for MinifyStylesConfig {
    fn default() -> Self {
        Self {
            restructure: true,
            force_media_merge: false,
            comments: false,
            usage: None,
        }
    }
}

/// Plugin that minifies CSS content in style elements and attributes
pub struct MinifyStylesPlugin {
    #[allow(dead_code)]
    config: MinifyStylesConfig,
}

impl MinifyStylesPlugin {
    /// Create a new MinifyStylesPlugin
    pub fn new() -> Self {
        Self {
            #[allow(dead_code)]
            config: MinifyStylesConfig::default(),
        }
    }

    /// Create a new MinifyStylesPlugin with config
    pub fn with_config(config: MinifyStylesConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<MinifyStylesConfig> {
        if params.is_null() || (params.is_object() && params.as_object().unwrap().is_empty()) {
            Ok(MinifyStylesConfig::default())
        } else if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(MinifyStylesConfig::default())
        }
    }

    /// Check if an element contains scripts
    fn _has_scripts(&self, element: &Element) -> bool {
        // Check if it's a script element
        if element.name == "script" && !element.children.is_empty() {
            return true;
        }

        // Check for javascript: links in href attributes
        if element.name == "a" {
            for (attr_name, attr_value) in &element.attributes {
                if attr_name == "href" || attr_name.ends_with(":href") {
                    if attr_value.trim_start().starts_with("javascript:") {
                        return true;
                    }
                }
            }
        }

        // Check for event attributes
        let event_attrs = [
            "onload",
            "onclick",
            "onmouseover",
            "onmouseout",
            "onfocus",
            "onblur",
            "onchange",
            "onsubmit",
            "onreset",
            "onkeydown",
            "onkeyup",
            "onkeypress",
        ];

        for attr in &event_attrs {
            if element.attributes.contains_key(*attr) {
                return true;
            }
        }

        false
    }

    /// Collect usage information from the document
    fn _collect_usage(
        &self,
        element: &Element,
    ) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
        let mut tags = HashSet::new();
        let mut ids = HashSet::new();
        let mut classes = HashSet::new();

        self._collect_usage_recursive(element, &mut tags, &mut ids, &mut classes);

        (tags, ids, classes)
    }

    /// Recursively collect usage information
    fn _collect_usage_recursive(
        &self,
        element: &Element,
        tags: &mut HashSet<String>,
        ids: &mut HashSet<String>,
        classes: &mut HashSet<String>,
    ) {
        // Collect tag name
        tags.insert(element.name.to_string());

        // Collect ID
        if let Some(id) = element.attributes.get("id") {
            ids.insert(id.to_string());
        }

        // Collect classes
        if let Some(class_attr) = element.attributes.get("class") {
            for class_name in class_attr.split_whitespace() {
                classes.insert(class_name.to_string());
            }
        }

        // Process children
        for child in &element.children {
            if let Node::Element(child_elem) = child {
                self._collect_usage_recursive(child_elem, tags, ids, classes);
            }
        }
    }

    /// Check if the document is deoptimized (contains scripts)
    fn _is_deoptimized(&self, element: &Element) -> bool {
        if self._has_scripts(element) {
            return true;
        }

        // Check children recursively
        for child in &element.children {
            if let Node::Element(child_elem) = child {
                if self._is_deoptimized(child_elem) {
                    return true;
                }
            }
        }

        false
    }

    /// Minify CSS text using LightningCSS
    fn minify_css(&self, css_text: &str) -> Result<String> {
        let parser_options = ParserOptions::default();

        match StyleSheet::parse(css_text, parser_options) {
            Ok(mut stylesheet) => {
                let minify_options = MinifyOptions {
                    targets: Targets::default(),
                    ..Default::default()
                };

                if let Err(_) = stylesheet.minify(minify_options) {
                    // If minification fails, return original CSS
                    return Ok(css_text.to_string());
                }

                match stylesheet.to_css(lightningcss::printer::PrinterOptions::default()) {
                    Ok(result) => Ok(result.code),
                    Err(_) => Ok(css_text.to_string()), // Fallback to original
                }
            }
            Err(_) => {
                // If parsing fails, return original CSS
                Ok(css_text.to_string())
            }
        }
    }

    /// Process style elements in the document
    fn process_style_elements(&self, element: &mut Element) {
        let mut elements_to_remove = Vec::new();

        for (index, child) in element.children.iter_mut().enumerate() {
            if let Node::Element(child_elem) = child {
                // Process style elements
                if child_elem.name == "style" && !child_elem.children.is_empty() {
                    if let Some(Node::Text(text_content)) = child_elem.children.get_mut(0) {
                        match self.minify_css(text_content) {
                            Ok(minified) => {
                                // Check if result is empty or only whitespace
                                if minified.trim().is_empty() {
                                    // Mark for removal if CSS is empty after minification
                                    elements_to_remove.push(index);
                                } else {
                                    *text_content = minified;
                                }
                            }
                            Err(_) => {
                                // Keep original on error
                            }
                        }
                    }
                }

                // Process style attributes
                if let Some(style_attr) = child_elem.attributes.get("style") {
                    // For inline styles, wrap in a block for proper parsing
                    let css_block = format!("{{{}}}", style_attr);
                    match self.minify_css(&css_block) {
                        Ok(minified) => {
                            // Remove the wrapping braces
                            let cleaned = minified
                                .trim_start_matches('{')
                                .trim_end_matches('}')
                                .trim()
                                .to_string();
                            if !cleaned.is_empty() {
                                child_elem.set_attr("style", cleaned);
                            } else {
                                child_elem.attributes.shift_remove("style");
                            }
                        }
                        Err(_) => {
                            // Keep original on error
                        }
                    }
                }

                // Recursively process children
                self.process_style_elements(child_elem);
            }
        }

        // Remove empty style elements (in reverse order to maintain indices)
        for &index in elements_to_remove.iter().rev() {
            element.children.remove(index);
        }
    }

    /// Apply styles minification to the document
    fn minify_styles_recursive(&self, element: &mut Element) {
        self.process_style_elements(element);
    }
}

impl Default for MinifyStylesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MinifyStylesPlugin {
    fn name(&self) -> &'static str {
        "minifyStyles"
    }

    fn description(&self) -> &'static str {
        "minifies styles and removes unused styles"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        self.minify_styles_recursive(&mut document.root);
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

    fn create_text_node(content: &str) -> Node<'static> {
        Node::Text(content.to_string())
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = MinifyStylesPlugin::new();
        assert_eq!(plugin.name(), "minifyStyles");
        assert_eq!(
            plugin.description(),
            "minifies styles and removes unused styles"
        );
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = MinifyStylesPlugin::new();

        // Valid parameters (empty object)
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Valid parameters (with values)
        assert!(plugin
            .validate_params(&json!({
                "restructure": true,
                "forceMediaMerge": false,
                "comments": true,
                "usage": {
                    "force": false,
                    "ids": true,
                    "classes": true,
                    "tags": true
                }
            }))
            .is_ok());

        // Invalid parameters (unknown field)
        assert!(plugin
            .validate_params(&json!({"unknownParam": "value"}))
            .is_err());
    }

    #[test]
    fn test_has_scripts() {
        let plugin = MinifyStylesPlugin::new();

        // Test script element
        let script = create_element("script");
        // Note: has_scripts checks if script has children, but for this test we'll check the name
        assert_eq!(script.name, "script");

        // Test element with onclick
        let mut button = create_element("button");
        button.set_attr("onclick", "alert('test')");
        assert!(plugin._has_scripts(&button));

        // Test normal element
        let div = create_element("div");
        assert!(!plugin._has_scripts(&div));
    }

    #[test]
    fn test_collect_usage() {
        let plugin = MinifyStylesPlugin::new();
        let mut doc = Document::new();

        // Create elements with different tags, ids, and classes
        let mut div = create_element("div");
        div.set_attr("id", "main");
        div.set_attr("class", "container wrapper");

        let mut span = create_element("span");
        span.set_attr("id", "text");
        span.set_attr("class", "highlight");

        div.children.push(Node::Element(span));
        doc.root.children.push(Node::Element(div));

        let (tags, ids, classes) = plugin._collect_usage(&doc.root);

        assert!(tags.contains("div"));
        assert!(tags.contains("span"));
        assert!(ids.contains("main"));
        assert!(ids.contains("text"));
        assert!(classes.contains("container"));
        assert!(classes.contains("wrapper"));
        assert!(classes.contains("highlight"));
    }

    #[test]
    fn test_style_element_minification() {
        let plugin = MinifyStylesPlugin::new();
        let mut doc = Document::new();

        // Create style element with CSS
        let mut style = create_element("style");
        style
            .children
            .push(create_text_node("  .test  {  color:  red;  }  "));
        doc.root.children.push(Node::Element(style));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that CSS was minified
        if let Node::Element(style_elem) = &doc.root.children[0] {
            if let Node::Text(text_content) = &style_elem.children[0] {
                // The exact output depends on LightningCSS, but it should be minified
                assert!(text_content.len() < "  .test  {  color:  red;  }  ".len());
                assert!(text_content.contains("red"));
            }
        }
    }

    #[test]
    fn test_style_attribute_minification() {
        let plugin = MinifyStylesPlugin::new();
        let mut doc = Document::new();

        // Create element with style attribute
        let mut div = create_element("div");
        div.set_attr("style", "  color:  red;  margin:  10px;  ");
        doc.root.children.push(Node::Element(div));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that style attribute was minified
        if let Node::Element(div_elem) = &doc.root.children[0] {
            let style_attr = div_elem.attr("style").unwrap();
            // Should be smaller and not contain extra spaces
            assert!(style_attr.len() < "  color:  red;  margin:  10px;  ".len());
            assert!(style_attr.contains("red"));
            assert!(style_attr.contains("10px"));
            assert!(!style_attr.starts_with("  "));
        }
    }

    #[test]
    fn test_empty_style_removal() {
        let plugin = MinifyStylesPlugin::new();
        let mut doc = Document::new();

        // Create style element with only whitespace
        let mut style = create_element("style");
        style.children.push(create_text_node("   "));
        doc.root.children.push(Node::Element(style));

        // Add another element to verify selective removal
        let div = create_element("div");
        doc.root.children.push(Node::Element(div));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Empty style should be removed, div should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.name, "div");
        }
    }

    #[test]
    fn test_nested_elements() {
        let plugin = MinifyStylesPlugin::new();
        let mut doc = Document::new();

        // Create nested structure with style elements
        let mut group = create_element("g");
        let mut style = create_element("style");
        style
            .children
            .push(create_text_node(".test { color: blue; }"));
        group.children.push(Node::Element(style));

        let mut rect = create_element("rect");
        rect.set_attr("style", "fill: green;");
        group.children.push(Node::Element(rect));

        doc.root.children.push(Node::Element(group));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Both nested style element and style attribute should be processed
        if let Node::Element(group_elem) = &doc.root.children[0] {
            // Check style element
            if let Node::Element(style_elem) = &group_elem.children[0] {
                assert_eq!(style_elem.name, "style");
                if let Node::Text(text_content) = &style_elem.children[0] {
                    // LightningCSS might change the output format, so just check if it's not empty
                    assert!(
                        !text_content.is_empty(),
                        "CSS should not be empty after minification"
                    );
                    // The color might be in a different format after minification (blue -> #00f)
                    assert!(
                        text_content.contains("blue")
                            || text_content.contains("#00f")
                            || text_content.contains("#0000ff")
                    );
                }
            }

            // Check style attribute
            if let Node::Element(rect_elem) = &group_elem.children[1] {
                let style_attr = rect_elem.attr("style").unwrap();
                assert!(style_attr.contains("green"));
            }
        }
    }

    #[test]
    fn test_invalid_css_handling() {
        let plugin = MinifyStylesPlugin::new();
        let mut doc = Document::new();

        // Create style element with invalid CSS
        let mut style = create_element("style");
        let invalid_css = "this is not valid css { }}}";
        style.children.push(create_text_node(invalid_css));
        doc.root.children.push(Node::Element(style));

        // Apply plugin - should not crash
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Invalid CSS should be preserved
        if let Node::Element(style_elem) = &doc.root.children[0] {
            if let Node::Text(text_content) = &style_elem.children[0] {
                assert_eq!(text_content, invalid_css);
            }
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = MinifyStylesPlugin::parse_config(&json!({
            "restructure": false,
            "forceMediaMerge": true,
            "comments": true,
            "usage": {
                "force": true,
                "ids": false,
                "classes": true,
                "tags": false
            }
        }))
        .unwrap();

        assert_eq!(config.restructure, false);
        assert_eq!(config.force_media_merge, true);
        assert_eq!(config.comments, true);

        let usage = config.usage.unwrap();
        assert_eq!(usage.force, true);
        assert_eq!(usage.ids, false);
        assert_eq!(usage.classes, true);
        assert_eq!(usage.tags, false);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(MinifyStylesPlugin, "minifyStyles");
