// this_file: crates/core/src/utils/attributes.rs

//! Attribute manipulation utilities
//!
//! Common functions for working with SVG attributes across multiple plugins.

use std::collections::HashSet;

use indexmap::IndexMap;

/// Common attribute manipulation functions
pub struct AttributeUtils;

impl AttributeUtils {
    /// Check if an attribute is a presentation attribute
    pub fn is_presentation_attribute(name: &str) -> bool {
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

        PRESENTATION_ATTRS.contains(&name)
    }

    /// Get default attribute values for specific elements
    pub fn get_default_value(element: &str, attribute: &str) -> Option<&'static str> {
        match (element, attribute) {
            // Fill and stroke defaults
            ("path", "fill") => Some("black"),
            ("path", "stroke") => Some("none"),
            ("circle", "fill") => Some("black"),
            ("circle", "stroke") => Some("none"),
            ("rect", "fill") => Some("black"),
            ("rect", "stroke") => Some("none"),
            ("ellipse", "fill") => Some("black"),
            ("ellipse", "stroke") => Some("none"),
            ("line", "fill") => Some("none"),
            ("line", "stroke") => Some("black"),
            ("polyline", "fill") => Some("none"),
            ("polyline", "stroke") => Some("black"),
            ("polygon", "fill") => Some("black"),
            ("polygon", "stroke") => Some("none"),

            // Positioning defaults
            ("rect", "x") => Some("0"),
            ("rect", "y") => Some("0"),
            ("circle", "cx") => Some("0"),
            ("circle", "cy") => Some("0"),
            ("ellipse", "cx") => Some("0"),
            ("ellipse", "cy") => Some("0"),
            ("line", "x1") => Some("0"),
            ("line", "y1") => Some("0"),
            ("line", "x2") => Some("0"),
            ("line", "y2") => Some("0"),

            // Stroke properties defaults
            ("path", "stroke-width") => Some("1"),
            ("circle", "stroke-width") => Some("1"),
            ("rect", "stroke-width") => Some("1"),
            ("ellipse", "stroke-width") => Some("1"),
            ("line", "stroke-width") => Some("1"),
            ("polyline", "stroke-width") => Some("1"),
            ("polygon", "stroke-width") => Some("1"),

            // Opacity defaults
            ("path", "fill-opacity") => Some("1"),
            ("path", "stroke-opacity") => Some("1"),
            ("circle", "fill-opacity") => Some("1"),
            ("circle", "stroke-opacity") => Some("1"),
            ("rect", "fill-opacity") => Some("1"),
            ("rect", "stroke-opacity") => Some("1"),

            // SVG namespace
            ("svg", "xmlns") => Some("http://www.w3.org/2000/svg"),

            // Other common defaults
            ("text", "text-anchor") => Some("start"),
            ("path", "fill-rule") => Some("nonzero"),
            ("polygon", "fill-rule") => Some("nonzero"),
            ("marker", "markerUnits") => Some("strokeWidth"),
            ("pattern", "patternUnits") => Some("objectBoundingBox"),
            ("clipPath", "clipPathUnits") => Some("userSpaceOnUse"),

            _ => None,
        }
    }

    /// Check if an attribute value is the default for the element
    pub fn is_default_value(element: &str, attribute: &str, value: &str) -> bool {
        Self::get_default_value(element, attribute).map(|default| default == value).unwrap_or(false)
    }

    /// Remove attributes with default values
    pub fn remove_default_attributes(element: &str, attributes: &mut IndexMap<String, String>) {
        attributes.retain(|name, value| !Self::is_default_value(element, name, value));
    }

    /// Sort attributes in a consistent order
    pub fn sort_attributes(attributes: &mut IndexMap<String, String>) {
        // Create a new sorted map
        let mut sorted = IndexMap::new();

        // Define attribute priority order
        let priority_attrs = &[
            "id",
            "class",
            "x",
            "y",
            "width",
            "height",
            "cx",
            "cy",
            "r",
            "rx",
            "ry",
            "x1",
            "y1",
            "x2",
            "y2",
            "points",
            "d",
            "transform",
            "viewBox",
            "preserveAspectRatio",
            "href",
            "xlink:href",
            "style",
        ];

        // Add priority attributes first
        for attr in priority_attrs {
            if let Some(value) = attributes.shift_remove(*attr) {
                sorted.insert(attr.to_string(), value);
            }
        }

        // Add remaining attributes in alphabetical order
        let mut remaining: Vec<_> = attributes.drain(..).collect();
        remaining.sort_by(|a, b| a.0.cmp(&b.0));

        for (name, value) in remaining {
            sorted.insert(name, value);
        }

        *attributes = sorted;
    }

    /// Get attributes that should be inherited from parent to child
    pub fn get_inheritable_attributes() -> &'static HashSet<&'static str> {
        static INHERITABLE_ATTRS: std::sync::OnceLock<HashSet<&'static str>> = std::sync::OnceLock::new();
        INHERITABLE_ATTRS.get_or_init(|| {
            [
                "clip-rule",
                "color",
                "color-interpolation",
                "color-interpolation-filters",
                "color-profile",
                "color-rendering",
                "cursor",
                "direction",
                "fill",
                "fill-opacity",
                "fill-rule",
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
                "marker-end",
                "marker-mid",
                "marker-start",
                "pointer-events",
                "shape-rendering",
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
                "opacity",
            ]
            .into_iter()
            .collect()
        })
    }

    /// Check if an attribute is inheritable
    pub fn is_inheritable(attribute: &str) -> bool {
        Self::get_inheritable_attributes().contains(attribute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_presentation_attribute() {
        assert!(AttributeUtils::is_presentation_attribute("fill"));
        assert!(AttributeUtils::is_presentation_attribute("stroke"));
        assert!(AttributeUtils::is_presentation_attribute("opacity"));
        assert!(!AttributeUtils::is_presentation_attribute("id"));
        assert!(!AttributeUtils::is_presentation_attribute("class"));
    }

    #[test]
    fn test_get_default_value() {
        assert_eq!(AttributeUtils::get_default_value("path", "fill"), Some("black"));
        assert_eq!(AttributeUtils::get_default_value("path", "stroke"), Some("none"));
        assert_eq!(AttributeUtils::get_default_value("svg", "xmlns"), Some("http://www.w3.org/2000/svg"));
        assert_eq!(AttributeUtils::get_default_value("unknown", "unknown"), None);
    }

    #[test]
    fn test_is_default_value() {
        assert!(AttributeUtils::is_default_value("path", "fill", "black"));
        assert!(AttributeUtils::is_default_value("path", "stroke", "none"));
        assert!(!AttributeUtils::is_default_value("path", "fill", "red"));
        assert!(!AttributeUtils::is_default_value("unknown", "unknown", "value"));
    }

    #[test]
    fn test_is_inheritable() {
        assert!(AttributeUtils::is_inheritable("fill"));
        assert!(AttributeUtils::is_inheritable("stroke"));
        assert!(AttributeUtils::is_inheritable("opacity"));
        assert!(!AttributeUtils::is_inheritable("id"));
        assert!(!AttributeUtils::is_inheritable("width"));
    }
}
