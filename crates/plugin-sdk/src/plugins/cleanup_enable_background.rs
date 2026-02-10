// this_file: crates/plugin-sdk/src/plugins/cleanup_enable_background.rs

//! Cleanup enable-background plugin implementation
//!
//! This plugin removes or cleans up the enable-background attribute when possible.
//! It follows the same pattern as svgo's cleanupEnableBackground plugin.

use crate::Plugin;
use anyhow::Result;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Plugin that cleans up enable-background attributes
#[derive(Default)]
pub struct CleanupEnableBackgroundPlugin {}

impl CleanupEnableBackgroundPlugin {
    /// Create a new CleanupEnableBackgroundPlugin
    pub fn new() -> Self {
        Self {}
    }
}

impl Plugin for CleanupEnableBackgroundPlugin {
    fn name(&self) -> &'static str {
        "cleanupEnableBackground"
    }

    fn description(&self) -> &'static str {
        "Remove or cleanup enable-background attribute when possible"
    }

    fn validate_params(&self, _params: &serde_json::Value) -> anyhow::Result<()> {
        // This plugin has no parameters
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let mut visitor = CleanupEnableBackgroundVisitor::new();
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        CleanupEnableBackgroundVisitor::cleanup_formatting_whitespace_recursive(&mut document.root);
        Ok(())
    }
}

/// Visitor implementation that cleans up enable-background attributes
struct CleanupEnableBackgroundVisitor {
    has_background_image_filter: bool,
    has_any_filter: bool,
}

impl CleanupEnableBackgroundVisitor {
    fn new() -> Self {
        Self {
            has_background_image_filter: false,
            has_any_filter: false,
        }
    }

    fn is_viewport_element(element_name: &str) -> bool {
        matches!(
            element_name,
            "svg" | "symbol" | "image" | "foreignObject" | "pattern" | "mask"
        )
    }

    fn is_formatting_whitespace(text: &str) -> bool {
        text.trim().is_empty() && text.chars().any(|c| matches!(c, '\n' | '\r' | '\t'))
    }

    fn cleanup_formatting_whitespace(nodes: &mut Vec<Node>) {
        let has_non_whitespace_content = nodes.iter().any(|node| match node {
            Node::Element(_) => true,
            Node::Text(text) => !text.trim().is_empty(),
            _ => false,
        });

        if has_non_whitespace_content {
            nodes.retain(|node| {
                !matches!(node, Node::Text(text) if Self::is_formatting_whitespace(text))
            });
        }
    }

    fn cleanup_formatting_whitespace_recursive(element: &mut Element<'_>) {
        let preserve = matches!(
            element.name.as_ref(),
            "text" | "tspan" | "tref" | "textPath" | "altGlyph"
        );
        for child in &mut element.children {
            if let Node::Text(text) = child {
                if !preserve && (text.contains('\n') || text.contains('\r') || text.contains('\t')) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        *text = trimmed.to_string().into();
                    }
                }
            }
        }
        Self::cleanup_formatting_whitespace(&mut element.children);
        for child in &mut element.children {
            if let Node::Element(child_element) = child {
                Self::cleanup_formatting_whitespace_recursive(child_element);
            }
        }
    }

    fn parse_number(value: &str) -> Option<f64> {
        value.trim().parse::<f64>().ok()
    }

    fn parse_enable_background(&self, value: &str) -> Option<EnableBackground> {
        let value = value.trim();
        
        if value == "new" {
            return Some(EnableBackground::New);
        }

        if value == "accumulate" {
            return Some(EnableBackground::Accumulate);
        }

        // Parse "new x y width height" format
        if value.starts_with("new ") {
            let parts: Vec<&str> = value[4..].split_whitespace().collect();
            if parts.len() == 4 {
                if let (Ok(_x), Ok(_y), Ok(_width), Ok(_height)) = (
                    parts[0].parse::<f64>(),
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                    parts[3].parse::<f64>(),
                ) {
                    return Some(EnableBackground::NewWithCoords);
                }
            }
        }

        None
    }

    fn normalize_enable_background(&self, value: &str, element: &Element) -> Option<String> {
        if !Self::is_viewport_element(element.name.as_ref()) {
            return None;
        }

        if self.has_background_image_filter || self.element_has_filter_with_background(element) {
            return Some(value.to_string());
        }

        if let Some(EnableBackground::New) = self.parse_enable_background(value) {
            if self.has_any_filter {
                return Some(value.to_string());
            }
            return None;
        }

        if let Some(EnableBackground::NewWithCoords) = self.parse_enable_background(value) {
            let parts: Vec<&str> = value[4..].split_whitespace().collect();
            if parts.len() == 4 {
                let x = Self::parse_number(parts[0]);
                let y = Self::parse_number(parts[1]);
                let width = Self::parse_number(parts[2]);
                let height = Self::parse_number(parts[3]);
                let element_width = element
                    .attributes
                    .get("width")
                    .and_then(|v| Self::parse_number(v));
                let element_height = element
                    .attributes
                    .get("height")
                    .and_then(|v| Self::parse_number(v));

                if let (
                    Some(x),
                    Some(y),
                    Some(width),
                    Some(height),
                    Some(element_width),
                    Some(element_height),
                ) = (x, y, width, height, element_width, element_height)
                {
                    let x_is_zero = x.abs() < f64::EPSILON;
                    let y_is_zero = y.abs() < f64::EPSILON;
                    let width_matches = (width - element_width).abs() < f64::EPSILON;
                    let height_matches = (height - element_height).abs() < f64::EPSILON;

                    if x_is_zero && y_is_zero && width_matches && height_matches {
                        if element.name == "svg" {
                            return None;
                        }
                        if self.has_any_filter {
                            return Some("new".to_string());
                        }
                        return None;
                    }
                }
            }
        }

        Some(value.to_string())
    }

    fn cleanup_style_attribute(style: &str) -> Option<String> {
        let mut kept_declarations = Vec::new();
        for declaration in style.split(';') {
            let trimmed = declaration.trim();
            if trimmed.is_empty() {
                continue;
            }
            let property_name = trimmed
                .split_once(':')
                .map(|(name, _)| name.trim())
                .unwrap_or("");
            if property_name == "enable-background" {
                continue;
            }
            kept_declarations.push(trimmed);
        }

        if kept_declarations.is_empty() {
            None
        } else {
            Some(kept_declarations.join("; "))
        }
    }

    fn element_has_filter_with_background(&self, element: &Element) -> bool {
        // Check if element has a filter attribute
        if element.attributes.contains_key("filter") {
            return true; // Conservative: assume filter might use background
        }

        // Check style attribute for filter property
        if let Some(style) = element.attributes.get("style") {
            if style.contains("filter:") || style.contains("filter ") {
                return true;
            }
        }

        false
    }

    fn check_for_background_image_filter(&mut self, element: &Element) {
        if element.name == "filter" {
            self.has_any_filter = true;
        }

        if element.name == "feImage" || element.name == "feBlend" {
            // Check for BackgroundImage or BackgroundAlpha usage
            if let Some(in_attr) = element.attributes.get("in") {
                if in_attr == "BackgroundImage" || in_attr == "BackgroundAlpha" {
                    self.has_background_image_filter = true;
                }
            }
            if let Some(in2_attr) = element.attributes.get("in2") {
                if in2_attr == "BackgroundImage" || in2_attr == "BackgroundAlpha" {
                    self.has_background_image_filter = true;
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum EnableBackground {
    New,
    NewWithCoords,
    Accumulate,
}

impl Visitor<'_> for CleanupEnableBackgroundVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // First check if this element has filter primitives that use background
        self.check_for_background_image_filter(element);
        
        Ok(())
    }

    fn visit_element_exit(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Process enable-background attribute after we've checked all children
        if let Some(enable_bg_value) = element.attributes.get("enable-background").cloned() {
            match self.normalize_enable_background(&enable_bg_value, element) {
                None => {
                    element.attributes.shift_remove("enable-background");
                }
                Some(normalized) => {
                    if normalized != enable_bg_value.as_ref() {
                        element
                            .attributes
                            .insert("enable-background".into(), normalized.into());
                    }
                }
            }
        }

        if let Some(style_value) = element.attributes.get("style").cloned() {
            match Self::cleanup_style_attribute(&style_value) {
                None => {
                    element.attributes.shift_remove("style");
                }
                Some(cleaned_style) => {
                    if cleaned_style != style_value.as_ref() {
                        element.attributes.insert("style".into(), cleaned_style.into());
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use vexy_svgo_core::ast::{Document, Element};

    #[test]
    fn test_plugin_creation() {
        let plugin = CleanupEnableBackgroundPlugin::new();
        assert_eq!(plugin.name(), "cleanupEnableBackground");
    }

    #[test]
    fn test_parse_enable_background() {
        let visitor = CleanupEnableBackgroundVisitor::new();

        assert_eq!(
            visitor.parse_enable_background("new"),
            Some(EnableBackground::New)
        );
        
        assert_eq!(
            visitor.parse_enable_background("accumulate"),
            Some(EnableBackground::Accumulate)
        );
        
        assert_eq!(
            visitor.parse_enable_background("new 0 0 100 100"),
            Some(EnableBackground::NewWithCoords)
        );
        
        assert_eq!(
            visitor.parse_enable_background("invalid"),
            None
        );
    }

    #[test]
    fn test_remove_default_new() {
        let plugin = CleanupEnableBackgroundPlugin::new();
        let mut doc = Document::new();

        // Add enable-background="new" to root element
        doc.root.set_attr("enable-background", "new");

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that enable-background was removed
        assert!(!doc.root.attributes.contains_key("enable-background"));
    }

    #[test]
    fn test_keep_with_coords() {
        let plugin = CleanupEnableBackgroundPlugin::new();
        let mut doc = Document::new();

        // Add enable-background with coordinates to SVG element
        doc.root.name = "svg".into();
        doc.root.set_attr("enable-background", "new 0 0 100 100");

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that enable-background was kept (SVG establishes viewport)
        assert!(doc.root.attributes.contains_key("enable-background"));
    }

    #[test]
    fn test_remove_from_non_viewport_element() {
        let plugin = CleanupEnableBackgroundPlugin::new();
        let mut doc = Document::new();

        // Create a g element with enable-background
        let mut g_element = Element::new("g");
        g_element.set_attr("enable-background", "new 0 0 100 100");
        
        doc.root.children.push(vexy_svgo_core::ast::Node::Element(g_element));

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that enable-background was removed from g element
        if let vexy_svgo_core::ast::Node::Element(ref g) = doc.root.children[0] {
            assert!(!g.attributes.contains_key("enable-background"));
        }
    }

    #[test]
    fn test_keep_with_filter() {
        let plugin = CleanupEnableBackgroundPlugin::new();
        let mut doc = Document::new();

        // Add enable-background and filter to element
        doc.root.set_attr("enable-background", "new");
        doc.root.set_attr("filter", "url(#myFilter)");

        // Apply the plugin
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that enable-background was kept due to filter
        assert!(doc.root.attributes.contains_key("enable-background"));
    }

    #[test]
    fn test_background_image_filter_detection() {
        let mut visitor = CleanupEnableBackgroundVisitor::new();
        
        // Test feBlend with BackgroundImage
        let mut element = Element::new("feBlend");
        element.set_attr("in", "BackgroundImage");
        visitor.check_for_background_image_filter(&element);
        assert!(visitor.has_background_image_filter);

        // Test feImage with BackgroundAlpha
        let mut visitor2 = CleanupEnableBackgroundVisitor::new();
        let mut element2 = Element::new("feImage");
        element2.set_attr("in2", "BackgroundAlpha");
        visitor2.check_for_background_image_filter(&element2);
        assert!(visitor2.has_background_image_filter);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(CleanupEnableBackgroundPlugin, "cleanupEnableBackground");
