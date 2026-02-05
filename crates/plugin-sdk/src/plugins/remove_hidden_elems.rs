// this_file: crates/plugin-sdk/src/plugins/remove_hidden_elems.rs

//! Remove hidden elements plugin implementation
//!
//! This plugin removes elements that are hidden through various means:
//! - display="none"
//! - visibility="hidden" or visibility="collapse"
//! - opacity="0" (optional)
//! - Zero width/height rectangles, ellipses, images
//! - Zero radius circles
//! - Empty paths
//!
//! SVGO parameters supported:
//! - `displayNone` (default: true) - Remove elements with display="none"
//! - `opacity0` (default: true) - Remove elements with opacity="0"
//! - `circleR0` (default: true) - Remove circles with r="0"
//! - `ellipseRX0` (default: true) - Remove ellipses with rx="0"
//! - `ellipseRY0` (default: true) - Remove ellipses with ry="0"
//! - `rectWidth0` (default: true) - Remove rects with width="0"
//! - `rectHeight0` (default: true) - Remove rects with height="0"
//! - `patternWidth0` (default: true) - Remove patterns with width="0"
//! - `patternHeight0` (default: true) - Remove patterns with height="0"
//! - `imageWidth0` (default: true) - Remove images with width="0"
//! - `imageHeight0` (default: true) - Remove images with height="0"
//! - `pathEmptyD` (default: true) - Remove paths with empty d attribute
//! - `polylineEmptyPoints` (default: true) - Remove polylines with empty points
//! - `polygonEmptyPoints` (default: true) - Remove polygons with empty points

use crate::Plugin;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Configuration parameters for remove hidden elems plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveHiddenElemsConfig {
    /// Remove elements with display="none"
    #[serde(default = "default_true")]
    pub display_none: bool,

    /// Remove elements with opacity="0"
    #[serde(default = "default_true")]
    pub opacity0: bool,

    /// Remove circles with r="0"
    #[serde(default = "default_true")]
    pub circle_r0: bool,

    /// Remove ellipses with rx="0"
    #[serde(default = "default_true")]
    pub ellipse_rx0: bool,

    /// Remove ellipses with ry="0"
    #[serde(default = "default_true")]
    pub ellipse_ry0: bool,

    /// Remove rects with width="0"
    #[serde(default = "default_true")]
    pub rect_width0: bool,

    /// Remove rects with height="0"
    #[serde(default = "default_true")]
    pub rect_height0: bool,

    /// Remove patterns with width="0"
    #[serde(default = "default_true")]
    pub pattern_width0: bool,

    /// Remove patterns with height="0"
    #[serde(default = "default_true")]
    pub pattern_height0: bool,

    /// Remove images with width="0"
    #[serde(default = "default_true")]
    pub image_width0: bool,

    /// Remove images with height="0"
    #[serde(default = "default_true")]
    pub image_height0: bool,

    /// Remove paths with empty d attribute
    #[serde(default = "default_true")]
    pub path_empty_d: bool,

    /// Remove polylines with empty points
    #[serde(default = "default_true")]
    pub polyline_empty_points: bool,

    /// Remove polygons with empty points
    #[serde(default = "default_true")]
    pub polygon_empty_points: bool,
}

impl Default for RemoveHiddenElemsConfig {
    fn default() -> Self {
        Self {
            display_none: default_true(),
            opacity0: default_true(),
            circle_r0: default_true(),
            ellipse_rx0: default_true(),
            ellipse_ry0: default_true(),
            rect_width0: default_true(),
            rect_height0: default_true(),
            pattern_width0: default_true(),
            pattern_height0: default_true(),
            image_width0: default_true(),
            image_height0: default_true(),
            path_empty_d: default_true(),
            polyline_empty_points: default_true(),
            polygon_empty_points: default_true(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// Plugin that removes hidden elements
pub struct RemoveHiddenElemsPlugin {
    config: RemoveHiddenElemsConfig,
}

impl RemoveHiddenElemsPlugin {
    /// Create a new RemoveHiddenElemsPlugin
    pub fn new() -> Self {
        Self {
            config: RemoveHiddenElemsConfig::default(),
        }
    }

    /// Create a new RemoveHiddenElemsPlugin with config
    pub fn with_config(config: RemoveHiddenElemsConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn _parse_config(params: &Value) -> Result<RemoveHiddenElemsConfig> {
        if let Some(_obj) = params.as_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(RemoveHiddenElemsConfig::default())
        }
    }
}

impl Default for RemoveHiddenElemsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveHiddenElemsPlugin {
    fn name(&self) -> &'static str {
        "removeHiddenElems"
    }

    fn description(&self) -> &'static str {
        "Remove hidden elements"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            // Validate all boolean parameters
            for (key, value) in obj {
                match key.as_str() {
                    "displayNone"
                    | "opacity0"
                    | "circleR0"
                    | "ellipseRX0"
                    | "ellipseRY0"
                    | "rectWidth0"
                    | "rectHeight0"
                    | "patternWidth0"
                    | "patternHeight0"
                    | "imageWidth0"
                    | "imageHeight0"
                    | "pathEmptyD"
                    | "polylineEmptyPoints"
                    | "polygonEmptyPoints" => {
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
        let mut visitor = HiddenElemsRemovalVisitor::new(self.config.clone());
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that removes hidden elements
struct HiddenElemsRemovalVisitor {
    config: RemoveHiddenElemsConfig,
}

impl HiddenElemsRemovalVisitor {
    fn new(config: RemoveHiddenElemsConfig) -> Self {
        Self { config }
    }

    /// Check if an element is hidden and should be removed
    fn is_element_hidden(&self, element: &Element) -> bool {
        // Check display="none"
        if self.config.display_none {
            if let Some(display) = element.attributes.get("display") {
                if display == "none" {
                    return true;
                }
            }
        }

        // Check visibility="hidden" or visibility="collapse"
        if let Some(visibility) = element.attributes.get("visibility") {
            if visibility == "hidden" || visibility == "collapse" {
                return true;
            }
        }

        // Check opacity="0"
        if self.config.opacity0 {
            if let Some(opacity) = element.attributes.get("opacity") {
                if let Ok(opacity_val) = opacity.parse::<f64>() {
                    if opacity_val == 0.0 {
                        return true;
                    }
                }
            }
        }

        // Check element-specific zero dimensions
        match element.name.as_ref() {
            "circle" if self.config.circle_r0 => {
                if self.is_zero_dimension(element, "r") {
                    return true;
                }
            }
            "ellipse" => {
                if (self.config.ellipse_rx0 && self.is_zero_dimension(element, "rx"))
                    || (self.config.ellipse_ry0 && self.is_zero_dimension(element, "ry"))
                {
                    return true;
                }
            }
            "rect" => {
                if (self.config.rect_width0 && self.is_zero_dimension(element, "width"))
                    || (self.config.rect_height0 && self.is_zero_dimension(element, "height"))
                {
                    return true;
                }
            }
            "pattern" => {
                if (self.config.pattern_width0 && self.is_zero_dimension(element, "width"))
                    || (self.config.pattern_height0 && self.is_zero_dimension(element, "height"))
                {
                    return true;
                }
            }
            "image" => {
                if (self.config.image_width0 && self.is_zero_dimension(element, "width"))
                    || (self.config.image_height0 && self.is_zero_dimension(element, "height"))
                {
                    return true;
                }
            }
            "path" if self.config.path_empty_d => {
                if let Some(d) = element.attributes.get("d") {
                    if d.trim().is_empty() {
                        return true;
                    }
                } else {
                    return true;
                }
            }
            "polyline" if self.config.polyline_empty_points => {
                if let Some(points) = element.attributes.get("points") {
                    if points.trim().is_empty() {
                        return true;
                    }
                } else {
                    return true;
                }
            }
            "polygon" if self.config.polygon_empty_points => {
                if let Some(points) = element.attributes.get("points") {
                    if points.trim().is_empty() {
                        return true;
                    }
                } else {
                    return true;
                }
            }
            "line" => {
                // Line is hidden if start and end points are the same
                let x1 = self.get_numeric_attr(element, "x1").unwrap_or(0.0);
                let y1 = self.get_numeric_attr(element, "y1").unwrap_or(0.0);
                let x2 = self.get_numeric_attr(element, "x2").unwrap_or(0.0);
                let y2 = self.get_numeric_attr(element, "y2").unwrap_or(0.0);
                if x1 == x2 && y1 == y2 {
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    fn is_zero_dimension(&self, element: &Element, attr_name: &str) -> bool {
        if let Some(value) = element.attributes.get(attr_name) {
            if let Ok(num_val) = value.parse::<f64>() {
                return num_val == 0.0;
            }
        }
        false
    }

    fn get_numeric_attr(&self, element: &Element, attr_name: &str) -> Option<f64> {
        element.attributes.get(attr_name)?.parse::<f64>().ok()
    }
}

impl Visitor<'_> for HiddenElemsRemovalVisitor {
    fn visit_element_exit(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Remove hidden child elements (process children first)
        element.children.retain(|child| {
            if let Node::Element(child_element) = child {
                !self.is_element_hidden(child_element)
            } else {
                true // Keep non-element nodes
            }
        });

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
        let plugin = RemoveHiddenElemsPlugin::new();
        assert_eq!(plugin.name(), "removeHiddenElems");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveHiddenElemsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({
                "displayNone": true,
                "opacity0": false,
                "circleR0": true
            }))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"displayNone": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"unknownParam": true}))
            .is_err());
    }

    #[test]
    fn test_remove_display_none() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add elements with display attributes
        let mut rect1 = create_element("rect");
        rect1
            .attributes
            .insert("display".into(), "none".into());
        rect1
            .attributes
            .insert("width".into(), "100".into());

        let mut rect2 = create_element("rect");
        rect2
            .attributes
            .insert("display".into(), "block".into());
        rect2
            .attributes
            .insert("width".into(), "100".into());

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));

        plugin.apply(&mut doc).unwrap();

        // Only the visible rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("display"), Some("block"));
        }
    }

    #[test]
    fn test_remove_visibility_hidden() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add elements with visibility attributes
        let mut rect1 = create_element("rect");
        rect1.set_attr("visibility", "hidden");

        let mut rect2 = create_element("rect");
        rect2.set_attr("visibility", "collapse");

        let mut rect3 = create_element("rect");
        rect3.set_attr("visibility", "visible");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(rect3));

        plugin.apply(&mut doc).unwrap();

        // Only the visible rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("visibility"), Some("visible"));
        }
    }

    #[test]
    fn test_remove_opacity_zero() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add elements with opacity
        let mut rect1 = create_element("rect");
        rect1.set_attr("opacity", "0");

        let mut rect2 = create_element("rect");
        rect2.set_attr("opacity", "0.0");

        let mut rect3 = create_element("rect");
        rect3.set_attr("opacity", "0.5");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(rect3));

        plugin.apply(&mut doc).unwrap();

        // Only the non-zero opacity rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("opacity"), Some("0.5"));
        }
    }

    #[test]
    fn test_remove_zero_dimension_circle() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add circles with different radii
        let mut circle1 = create_element("circle");
        circle1.set_attr("r", "0");

        let mut circle2 = create_element("circle");
        circle2.set_attr("r", "10");

        doc.root.children.push(Node::Element(circle1));
        doc.root.children.push(Node::Element(circle2));

        plugin.apply(&mut doc).unwrap();

        // Only non-zero radius circle should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("r"), Some("10"));
        }
    }

    #[test]
    fn test_remove_zero_dimension_rect() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add rects with zero dimensions
        let mut rect1 = create_element("rect");
        rect1.set_attr("width", "0");
        rect1.set_attr("height", "100");

        let mut rect2 = create_element("rect");
        rect2.set_attr("width", "100");
        rect2.set_attr("height", "0");

        let mut rect3 = create_element("rect");
        rect3.set_attr("width", "100");
        rect3.set_attr("height", "100");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(rect3));

        plugin.apply(&mut doc).unwrap();

        // Only non-zero dimension rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("width"), Some("100"));
            assert_eq!(elem.attr("height"), Some("100"));
        }
    }

    #[test]
    fn test_remove_empty_path() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add paths with different d attributes
        let mut path1 = create_element("path");
        path1.set_attr("d", "");

        let mut path2 = create_element("path");
        path2.set_attr("d", "   ");

        let mut path3 = create_element("path");
        path3.set_attr("d", "M10 10 L20 20");

        let path4 = create_element("path"); // No d attribute

        doc.root.children.push(Node::Element(path1));
        doc.root.children.push(Node::Element(path2));
        doc.root.children.push(Node::Element(path3));
        doc.root.children.push(Node::Element(path4));

        plugin.apply(&mut doc).unwrap();

        // Only path with valid d should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("d"), Some("M10 10 L20 20"));
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = RemoveHiddenElemsPlugin::_parse_config(&json!({
            "displayNone": false,
            "opacity0": true,
            "circleR0": false,
            "pathEmptyD": true
        }))
        .unwrap();

        assert_eq!(config.display_none, false);
        assert_eq!(config.opacity0, true);
        assert_eq!(config.circle_r0, false);
        assert_eq!(config.path_empty_d, true);
    }

    #[test]
    fn test_line_same_coordinates() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add lines with same start/end points
        let mut line1 = create_element("line");
        line1.attributes.insert("x1".into(), "10".into());
        line1.attributes.insert("y1".into(), "10".into());
        line1.attributes.insert("x2".into(), "10".into());
        line1.attributes.insert("y2".into(), "10".into());

        let mut line2 = create_element("line");
        line2.attributes.insert("x1".into(), "10".into());
        line2.attributes.insert("y1".into(), "10".into());
        line2.attributes.insert("x2".into(), "20".into());
        line2.attributes.insert("y2".into(), "20".into());

        doc.root.children.push(Node::Element(line1));
        doc.root.children.push(Node::Element(line2));

        plugin.apply(&mut doc).unwrap();

        // Only line with different coordinates should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("x2"), Some("20"));
        }
    }

    #[test]
    fn test_selective_removal_config() {
        let config = RemoveHiddenElemsConfig {
            display_none: false,
            opacity0: true,
            ..Default::default()
        };
        let plugin = RemoveHiddenElemsPlugin::with_config(config);
        let mut doc = Document::new();

        // Add elements that would normally be removed
        let mut rect1 = create_element("rect");
        rect1
            .attributes
            .insert("display".into(), "none".into());

        let mut rect2 = create_element("rect");
        rect2
            .attributes
            .insert("opacity".into(), "0".into());

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));

        plugin.apply(&mut doc).unwrap();

        // display="none" should be kept, opacity="0" should be removed
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("display"), Some("none"));
        }
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveHiddenElemsPlugin, "removeHiddenElems");
