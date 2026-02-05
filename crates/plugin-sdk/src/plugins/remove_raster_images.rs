// this_file: crates/plugin-sdk/src/plugins/remove_raster_images.rs

//! Remove raster images plugin implementation
//!
//! This plugin removes raster image references in <image> elements.
//! It detects and removes images with JPEG, PNG, or GIF formats.
//!
//! Reference: SVGO's removeRasterImages plugin

use crate::Plugin;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration parameters for remove raster images plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveRasterImagesConfig {
    // No configuration options for this plugin
}

impl Default for RemoveRasterImagesConfig {
    fn default() -> Self {
        Self {}
    }
}

/// Plugin that removes raster images
pub struct RemoveRasterImagesPlugin {
    config: RemoveRasterImagesConfig,
    raster_pattern: Regex,
}

impl RemoveRasterImagesPlugin {
    /// Create a new RemoveRasterImagesPlugin
    pub fn new() -> Self {
        // Pattern to match raster image references
        // Matches: .jpg, .jpeg, .png, .gif or image/jpeg, image/png, image/gif
        let raster_pattern =
            Regex::new(r"(\.|image/)(jpe?g|png|gif)").expect("Invalid regex pattern");

        Self {
            config: RemoveRasterImagesConfig::default(),
            raster_pattern,
        }
    }

    /// Create a new RemoveRasterImagesPlugin with config
    pub fn with_config(config: RemoveRasterImagesConfig) -> Self {
        let mut plugin = Self::new();
        plugin.config = config;
        plugin
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<RemoveRasterImagesConfig> {
        if params.is_null() {
            Ok(RemoveRasterImagesConfig::default())
        } else if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Err(anyhow::anyhow!("Configuration must be an object"))
        }
    }

    /// Check if element is a raster image
    fn is_raster_image(&self, element: &Element) -> bool {
        if element.name != "image" {
            return false;
        }

        // Check xlink:href attribute
        if let Some(href) = element.attributes.get("xlink:href") {
            if self.raster_pattern.is_match(href) {
                return true;
            }
        }

        // Also check href attribute (SVG2 style)
        if let Some(href) = element.attributes.get("href") {
            if self.raster_pattern.is_match(href) {
                return true;
            }
        }

        false
    }

    /// Process element to remove raster images
    fn process_element(&self, element: &mut Element) {
        // Remove image elements that reference raster images
        element.children.retain(|child| {
            if let Node::Element(elem) = child {
                !self.is_raster_image(elem)
            } else {
                true
            }
        });

        // Recursively process children
        for child in &mut element.children {
            if let Node::Element(elem) = child {
                self.process_element(elem);
            }
        }
    }
}

impl Default for RemoveRasterImagesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveRasterImagesPlugin {
    fn name(&self) -> &'static str {
        "removeRasterImages"
    }

    fn description(&self) -> &'static str {
        "removes raster images (disabled by default)"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        self.process_element(&mut document.root);
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
        let plugin = RemoveRasterImagesPlugin::new();
        assert_eq!(plugin.name(), "removeRasterImages");
        assert_eq!(
            plugin.description(),
            "removes raster images (disabled by default)"
        );
    }

    #[test]
    fn test_removes_jpeg_images() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add JPEG image
        let mut image = create_element("image");
        image.set_attr("xlink:href", "photo.jpg");
        doc.root.children.push(Node::Element(image));

        // Add SVG image (should be preserved)
        let mut svg_image = create_element("image");
        svg_image.set_attr("xlink:href", "icon.svg");
        doc.root.children.push(Node::Element(svg_image));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that only SVG image remains
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(elem) = &doc.root.children[0] {
            assert_eq!(elem.attr("xlink:href"), Some("icon.svg"));
        }
    }

    #[test]
    fn test_removes_png_images() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add PNG image
        let mut image = create_element("image");
        image.set_attr("xlink:href", "logo.png");
        doc.root.children.push(Node::Element(image));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that image was removed
        assert_eq!(doc.root.children.len(), 0);
    }

    #[test]
    fn test_removes_gif_images() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add GIF image
        let mut image = create_element("image");
        image.set_attr("xlink:href", "animation.gif");
        doc.root.children.push(Node::Element(image));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that image was removed
        assert_eq!(doc.root.children.len(), 0);
    }

    #[test]
    fn test_removes_data_uri_raster_images() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add data URI JPEG
        let mut image1 = create_element("image");
        image1.set_attr("xlink:href", "data:image/jpeg;base64,/9j/4AAQ...");
        doc.root.children.push(Node::Element(image1));

        // Add data URI PNG
        let mut image2 = create_element("image");
        image2.set_attr("xlink:href", "data:image/png;base64,iVBORw0KGg...");
        doc.root.children.push(Node::Element(image2));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that both images were removed
        assert_eq!(doc.root.children.len(), 0);
    }

    #[test]
    fn test_removes_svg2_href_images() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add image with SVG2 href attribute
        let mut image = create_element("image");
        image.set_attr("href", "picture.jpeg");
        doc.root.children.push(Node::Element(image));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that image was removed
        assert_eq!(doc.root.children.len(), 0);
    }

    #[test]
    fn test_preserves_vector_images() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add various vector image formats
        let mut svg_image = create_element("image");
        svg_image.set_attr("xlink:href", "vector.svg");
        doc.root.children.push(Node::Element(svg_image));

        let mut pdf_image = create_element("image");
        pdf_image.set_attr("xlink:href", "document.pdf");
        doc.root.children.push(Node::Element(pdf_image));

        let mut data_svg = create_element("image");
        data_svg.set_attr("xlink:href", "data:image/svg+xml;base64,PHN2Zy...");
        doc.root.children.push(Node::Element(data_svg));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all vector images were preserved
        assert_eq!(doc.root.children.len(), 3);
    }

    #[test]
    fn test_removes_nested_raster_images() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Create nested structure
        let mut group = create_element("g");

        let mut image1 = create_element("image");
        image1.set_attr("xlink:href", "nested.jpg");
        group.children.push(Node::Element(image1));

        let rect = create_element("rect");
        group.children.push(Node::Element(rect));

        let mut defs = create_element("defs");
        let mut pattern = create_element("pattern");
        let mut image2 = create_element("image");
        image2.set_attr("href", "pattern.png");
        pattern.children.push(Node::Element(image2));
        defs.children.push(Node::Element(pattern));
        group.children.push(Node::Element(defs));

        doc.root.children.push(Node::Element(group));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check nested removal
        if let Node::Element(g) = &doc.root.children[0] {
            // First child should be rect (image removed)
            if let Node::Element(r) = &g.children[0] {
                assert_eq!(r.name, "rect");
            }

            // Check pattern has no image
            if let Node::Element(d) = &g.children[1] {
                if let Node::Element(p) = &d.children[0] {
                    assert_eq!(p.children.len(), 0);
                }
            }
        }
    }

    #[test]
    fn test_ignores_non_image_elements() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Add elements with raster-like hrefs that aren't images
        let mut use_elem = create_element("use");
        use_elem.set_attr("xlink:href", "icon.png");
        doc.root.children.push(Node::Element(use_elem));

        let mut a_elem = create_element("a");
        a_elem.set_attr("href", "photo.jpg");
        doc.root.children.push(Node::Element(a_elem));

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that non-image elements were preserved
        assert_eq!(doc.root.children.len(), 2);
    }

    #[test]
    fn test_empty_document() {
        let plugin = RemoveRasterImagesPlugin::new();

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Apply plugin to empty document
        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveRasterImagesPlugin::new();

        // Empty object is valid
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Null is valid
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Non-object is invalid
        assert!(plugin.validate_params(&json!("invalid")).is_err());
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(RemoveRasterImagesPlugin, "removeRasterImages");
