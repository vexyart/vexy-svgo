// this_file: crates/plugin-sdk/src/plugins/cleanup_enable_background.rs

//! Cleanup enable-background plugin implementation
//!
//! This plugin removes or cleans up the enable-background attribute when possible.
//! It follows the same pattern as svgo's cleanupEnableBackground plugin.

use crate::Plugin;
use anyhow::Result;
use vexy_svgo_core::ast::{Document, Element};
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
        Ok(())
    }
}

/// Visitor implementation that cleans up enable-background attributes
struct CleanupEnableBackgroundVisitor {
    has_background_image_filter: bool,
}

impl CleanupEnableBackgroundVisitor {
    fn new() -> Self {
        Self {
            has_background_image_filter: false,
        }
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

    fn should_remove_enable_background(&self, value: &str, element: &Element) -> bool {
        // Don't remove if there's a BackgroundImage filter that might use it
        if self.has_background_image_filter {
            return false;
        }

        // Remove if it's just "new" (the default value)
        if let Some(EnableBackground::New) = self.parse_enable_background(value) {
            return true;
        }

        // Check if this element or its children have filters that might use background
        if self.element_has_filter_with_background(element) {
            return false;
        }

        // Remove if enable-background is on an element that doesn't establish a viewport
        // (only svg, symbol, image, foreignObject establish new viewports)
        if !matches!(element.name.as_ref(), "svg" | "symbol" | "image" | "foreignObject") {
            return true;
        }

        false
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
            if self.should_remove_enable_background(&enable_bg_value, element) {
                element.attributes.shift_remove("enable-background");
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