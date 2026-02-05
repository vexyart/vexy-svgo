// this_file: crates/plugin-sdk/src/plugins/merge_paths.rs

//! Merge paths plugin implementation
//!
//! This plugin merges multiple path elements that can be combined into a single path.
//! This is a complex optimization that can significantly reduce SVG file size.
//!
//! Note: This is a basic implementation focused on simple cases. A full implementation
//! would require advanced path analysis using the lyon crate for proper path operations
//! and sophisticated intersection detection.

use crate::Plugin;
use anyhow::Result;
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Plugin configuration for merge paths
#[derive(Debug, Clone)]
pub struct MergePathsConfig {
    pub force: bool,
    pub float_precision: u8,
    pub no_space_after_flags: bool,
}

impl Default for MergePathsConfig {
    fn default() -> Self {
        Self {
            force: false,
            float_precision: 3,
            no_space_after_flags: false,
        }
    }
}

/// Plugin that merges compatible path elements
pub struct MergePathsPlugin {
    config: MergePathsConfig,
}

impl MergePathsPlugin {
    /// Create a new MergePathsPlugin
    pub fn new() -> Self {
        Self {
            config: MergePathsConfig::default(),
        }
    }

    /// Create a new MergePathsPlugin with config
    pub fn with_config(config: MergePathsConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    #[allow(dead_code)]
    fn parse_config(params: &Value) -> Result<MergePathsConfig> {
        let mut config = MergePathsConfig::default();

        if let Some(obj) = params.as_object() {
            if let Some(force) = obj.get("force") {
                config.force = force.as_bool().unwrap_or(false);
            }
            if let Some(precision) = obj.get("floatPrecision") {
                config.float_precision = precision.as_u64().unwrap_or(3) as u8;
            }
            if let Some(no_space) = obj.get("noSpaceAfterFlags") {
                config.no_space_after_flags = no_space.as_bool().unwrap_or(false);
            }
        }

        Ok(config)
    }

    /// Check if two path elements can be merged
    #[allow(dead_code)]
    fn can_merge_paths(&self, path1: &Element, path2: &Element) -> bool {
        // Basic checks for mergeability

        // Must both be path elements
        if path1.name != "path" || path2.name != "path" {
            return false;
        }

        // Must both have path data
        let d1 = path1.attributes.get("d");
        let d2 = path2.attributes.get("d");
        if d1.is_none() || d2.is_none() {
            return false;
        }

        // Must have no child elements
        if !path1.children.is_empty() || !path2.children.is_empty() {
            return false;
        }

        // Check that style-affecting attributes are identical
        let style_attrs = [
            "fill",
            "stroke",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "stroke-dasharray",
            "stroke-dashoffset",
            "opacity",
            "fill-opacity",
            "stroke-opacity",
            "fill-rule",
            "clip-path",
            "mask",
            "filter",
            "marker-start",
            "marker-mid",
            "marker-end",
            "class",
            "style",
        ];

        for attr in &style_attrs {
            let val1 = path1.attributes.get(*attr);
            let val2 = path2.attributes.get(*attr);
            if val1 != val2 {
                return false;
            }
        }

        // Cannot merge paths with markers (would break marker positioning)
        for marker_attr in &["marker-start", "marker-mid", "marker-end"] {
            if path1.attributes.contains_key(*marker_attr)
                || path2.attributes.contains_key(*marker_attr)
            {
                return false;
            }
        }

        // Cannot merge paths with URL references (gradients, patterns, filters)
        for attr in &["fill", "stroke", "clip-path", "mask", "filter"] {
            if let Some(val) = path1.attributes.get(*attr) {
                if val.contains("url(") {
                    return false;
                }
            }
            if let Some(val) = path2.attributes.get(*attr) {
                if val.contains("url(") {
                    return false;
                }
            }
        }

        true
    }

    /// Merge two path elements' path data
    #[allow(dead_code)]
    fn merge_path_data(&self, d1: &str, d2: &str) -> String {
        if d1.is_empty() {
            return d2.to_string();
        }
        if d2.is_empty() {
            return d1.to_string();
        }

        let mut result = d1.to_string();

        // Add the second path's data
        // Note: In a full implementation, this would need proper path parsing
        // and handling of Z commands, coordinate normalization, etc.
        if !result.ends_with(' ') {
            result.push(' ');
        }
        result.push_str(d2);

        result
    }
}

impl Default for MergePathsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MergePathsPlugin {
    fn name(&self) -> &'static str {
        "mergePaths"
    }

    fn description(&self) -> &'static str {
        "Merge compatible path elements"
    }

    fn validate_params(&self, params: &Value) -> anyhow::Result<()> {
        // Validate configuration parameters
        if let Some(obj) = params.as_object() {
            if let Some(force) = obj.get("force") {
                if !force.is_boolean() {
                    return Err(anyhow::anyhow!("force parameter must be a boolean"));
                }
            }
            if let Some(precision) = obj.get("floatPrecision") {
                if !precision.is_u64() {
                    return Err(anyhow::anyhow!("floatPrecision parameter must be a number"));
                }
            }
            if let Some(no_space) = obj.get("noSpaceAfterFlags") {
                if !no_space.is_boolean() {
                    return Err(anyhow::anyhow!(
                        "noSpaceAfterFlags parameter must be a boolean"
                    ));
                }
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let mut visitor = PathMergeVisitor::new(self.config.clone());
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that merges path elements
struct PathMergeVisitor {
    #[allow(dead_code)]
    config: MergePathsConfig,
}

impl PathMergeVisitor {
    fn new(config: MergePathsConfig) -> Self {
        Self { config }
    }

    /// Process children of an element to merge consecutive compatible paths
    fn merge_paths_in_children(&self, element: &mut Element) {
        let mut i = 0;
        while i < element.children.len() {
            // Check if current element is a path
            let is_path = match element.children.get(i) {
                Some(Node::Element(elem)) => elem.name == "path",
                _ => false,
            };

            if is_path {
                // Collect information about mergeable paths starting from position i
                let mut merge_info = Vec::new();
                let mut j = i;

                // Look for consecutive path elements that can be merged
                while j < element.children.len() {
                    if let Some(Node::Element(ref path)) = element.children.get(j) {
                        if path.name == "path" {
                            // Check if this path can be merged with the first one
                            if j == i {
                                // First path - always include
                                merge_info.push((
                                    j,
                                    path.attributes.get("d").cloned().unwrap_or_default(),
                                ));
                                j += 1;
                            } else {
                                // Check if this path can be merged with the first path
                                if let Some(Node::Element(ref first_path)) = element.children.get(i)
                                {
                                    if self.can_merge_paths(first_path, path) {
                                        merge_info.push((
                                            j,
                                            path.attributes.get("d").cloned().unwrap_or_default(),
                                        ));
                                        j += 1;
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        } else {
                            // Non-path element, stop looking for consecutive paths
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // If we found paths to merge, do the merge
                if merge_info.len() > 1 {
                    // Merge all the path data
                    let mut merged_data = String::new();
                    for (_, data) in &merge_info {
                        merged_data = self.merge_path_data(&merged_data, data);
                    }

                    // Update the first path with the merged data
                    if let Some(Node::Element(ref mut first_path)) = element.children.get_mut(i) {
                        first_path.attributes.insert("d".into(), merged_data.into());
                    }

                    // Remove the merged paths (remove in reverse order to maintain indices)
                    // Skip the first element (index 0) since that's the one we're keeping
                    for (idx, _) in merge_info.iter().rev().take(merge_info.len() - 1) {
                        element.children.remove(*idx);
                    }
                }
            }
            i += 1;
        }
    }

    /// Check if two path elements can be merged
    fn can_merge_paths(&self, path1: &Element, path2: &Element) -> bool {
        // Must both be path elements
        if path1.name != "path" || path2.name != "path" {
            return false;
        }

        // Must both have path data
        let d1 = path1.attributes.get("d");
        let d2 = path2.attributes.get("d");
        if d1.is_none() || d2.is_none() {
            return false;
        }

        // Must have no child elements
        if !path1.children.is_empty() || !path2.children.is_empty() {
            return false;
        }

        // Check that style-affecting attributes are identical
        let style_attrs = [
            "fill",
            "stroke",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "stroke-dasharray",
            "stroke-dashoffset",
            "opacity",
            "fill-opacity",
            "stroke-opacity",
            "fill-rule",
            "clip-path",
            "mask",
            "filter",
            "marker-start",
            "marker-mid",
            "marker-end",
            "class",
            "style",
        ];

        for attr in &style_attrs {
            let val1 = path1.attributes.get(*attr);
            let val2 = path2.attributes.get(*attr);
            if val1 != val2 {
                return false;
            }
        }

        // Cannot merge paths with markers (would break marker positioning)
        for marker_attr in &["marker-start", "marker-mid", "marker-end"] {
            if path1.attributes.contains_key(*marker_attr)
                || path2.attributes.contains_key(*marker_attr)
            {
                return false;
            }
        }

        // Cannot merge paths with URL references (gradients, patterns, filters)
        for attr in &["fill", "stroke", "clip-path", "mask", "filter"] {
            if let Some(val) = path1.attributes.get(*attr) {
                if val.contains("url(") {
                    return false;
                }
            }
            if let Some(val) = path2.attributes.get(*attr) {
                if val.contains("url(") {
                    return false;
                }
            }
        }

        true
    }

    /// Merge two path elements' path data
    fn merge_path_data(&self, d1: &str, d2: &str) -> String {
        if d1.is_empty() {
            return d2.to_string();
        }
        if d2.is_empty() {
            return d1.to_string();
        }

        let mut result = d1.to_string();

        // Add the second path's data
        // Note: In a full implementation, this would need proper path parsing
        // and handling of Z commands, coordinate normalization, etc.
        if !result.ends_with(' ') {
            result.push(' ');
        }
        result.push_str(d2);

        result
    }
}

impl Visitor<'_> for PathMergeVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Process children to merge consecutive compatible paths
        self.merge_paths_in_children(element);
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

    fn create_path_element(d: &str) -> Element<'static> {
        let mut element = create_element("path");
        element.set_attr("d", d);
        element
    }

    fn create_path_element_with_attrs(d: &str, attrs: &[(&str, &str)]) -> Element<'static> {
        let mut element = create_path_element(d);
        for (key, value) in attrs {
            element.set_attr(*key, *value);
        }
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = MergePathsPlugin::new();
        assert_eq!(plugin.name(), "mergePaths");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = MergePathsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin.validate_params(&json!({"force": true})).is_ok());
        assert!(plugin
            .validate_params(&json!({"floatPrecision": 5}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"noSpaceAfterFlags": true}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"force": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"floatPrecision": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"noSpaceAfterFlags": "invalid"}))
            .is_err());
    }

    #[test]
    fn test_can_merge_paths() {
        let plugin = MergePathsPlugin::new();

        // Case 1: Can merge - identical paths
        let path1 = create_path_element("M0 0L10 10");
        let path2 = create_path_element("M20 20L30 30");
        assert!(plugin.can_merge_paths(&path1, &path2));

        // Case 2: Can merge - same fill
        let path1 = create_path_element_with_attrs("M0 0L10 10", &[("fill", "red")]);
        let path2 = create_path_element_with_attrs("M20 20L30 30", &[("fill", "red")]);
        assert!(plugin.can_merge_paths(&path1, &path2));

        // Case 3: Cannot merge - different fill
        let path1 = create_path_element_with_attrs("M0 0L10 10", &[("fill", "red")]);
        let path2 = create_path_element_with_attrs("M20 20L30 30", &[("fill", "blue")]);
        assert!(!plugin.can_merge_paths(&path1, &path2));

        // Case 4: Cannot merge - has markers
        let path1 =
            create_path_element_with_attrs("M0 0L10 10", &[("marker-start", "url(#marker)")]);
        let path2 = create_path_element("M20 20L30 30");
        assert!(!plugin.can_merge_paths(&path1, &path2));

        // Case 5: Cannot merge - has URL reference
        let path1 = create_path_element_with_attrs("M0 0L10 10", &[("fill", "url(#gradient)")]);
        let path2 = create_path_element("M20 20L30 30");
        assert!(!plugin.can_merge_paths(&path1, &path2));

        // Case 6: Cannot merge - missing path data
        let path1 = create_element("path");
        let path2 = create_path_element("M20 20L30 30");
        assert!(!plugin.can_merge_paths(&path1, &path2));
    }

    #[test]
    fn test_merge_path_data() {
        let plugin = MergePathsPlugin::new();

        let result = plugin.merge_path_data("M0 0L10 10", "M20 20L30 30");
        assert_eq!(result, "M0 0L10 10 M20 20L30 30");

        let result = plugin.merge_path_data("M0 0L10 10 ", "M20 20L30 30");
        assert_eq!(result, "M0 0L10 10 M20 20L30 30");

        let result = plugin.merge_path_data("", "M20 20L30 30");
        assert_eq!(result, "M20 20L30 30");
    }

    #[test]
    fn test_plugin_apply_basic_merge() {
        let plugin = MergePathsPlugin::new();
        let mut doc = Document::new();

        // Create two mergeable paths
        let path1 = create_path_element("M0 0L10 10");
        let path2 = create_path_element("M20 20L30 30");

        doc.root.children.push(Node::Element(path1));
        doc.root.children.push(Node::Element(path2));

        plugin.apply(&mut doc).unwrap();

        // Should have merged into one path
        assert_eq!(doc.root.children.len(), 1);

        if let Some(Node::Element(merged_path)) = doc.root.children.first() {
            assert_eq!(merged_path.name, "path");
            assert_eq!(merged_path.attr("d"), Some("M0 0L10 10 M20 20L30 30"));
        }
    }

    #[test]
    fn test_plugin_apply_no_merge_different_attrs() {
        let plugin = MergePathsPlugin::new();
        let mut doc = Document::new();

        // Create two paths with different attributes
        let path1 = create_path_element_with_attrs("M0 0L10 10", &[("fill", "red")]);
        let path2 = create_path_element_with_attrs("M20 20L30 30", &[("fill", "blue")]);

        doc.root.children.push(Node::Element(path1));
        doc.root.children.push(Node::Element(path2));

        plugin.apply(&mut doc).unwrap();

        // Should not have merged
        assert_eq!(doc.root.children.len(), 2);
    }

    #[test]
    fn test_plugin_apply_merge_with_same_attrs() {
        let plugin = MergePathsPlugin::new();
        let mut doc = Document::new();

        // Create two paths with same attributes
        let path1 =
            create_path_element_with_attrs("M0 0L10 10", &[("fill", "red"), ("stroke", "blue")]);
        let path2 =
            create_path_element_with_attrs("M20 20L30 30", &[("fill", "red"), ("stroke", "blue")]);

        doc.root.children.push(Node::Element(path1));
        doc.root.children.push(Node::Element(path2));

        plugin.apply(&mut doc).unwrap();

        // Should have merged into one path
        assert_eq!(doc.root.children.len(), 1);

        if let Some(Node::Element(merged_path)) = doc.root.children.first() {
            assert_eq!(merged_path.name, "path");
            assert_eq!(merged_path.attr("d"), Some("M0 0L10 10 M20 20L30 30"));
            assert_eq!(merged_path.attr("fill"), Some("red"));
            assert_eq!(merged_path.attr("stroke"), Some("blue"));
        }
    }

    #[test]
    fn test_plugin_apply_merge_three_paths() {
        let plugin = MergePathsPlugin::new();
        let mut doc = Document::new();

        // Create three mergeable paths
        let path1 = create_path_element("M0 0L10 10");
        let path2 = create_path_element("M20 20L30 30");
        let path3 = create_path_element("M40 40L50 50");

        doc.root.children.push(Node::Element(path1));
        doc.root.children.push(Node::Element(path2));
        doc.root.children.push(Node::Element(path3));

        plugin.apply(&mut doc).unwrap();

        // Should have merged into one path
        assert_eq!(doc.root.children.len(), 1);

        if let Some(Node::Element(merged_path)) = doc.root.children.first() {
            assert_eq!(merged_path.name, "path");
            assert_eq!(
                merged_path.attr("d"),
                Some("M0 0L10 10 M20 20L30 30 M40 40L50 50")
            );
        }
    }

    #[test]
    fn test_plugin_apply_mixed_elements() {
        let plugin = MergePathsPlugin::new();
        let mut doc = Document::new();

        // Create mixed elements with paths
        let path1 = create_path_element("M0 0L10 10");
        let rect = create_element("rect");
        let path2 = create_path_element("M20 20L30 30");

        doc.root.children.push(Node::Element(path1));
        doc.root.children.push(Node::Element(rect));
        doc.root.children.push(Node::Element(path2));

        plugin.apply(&mut doc).unwrap();

        // Should not have merged paths separated by other elements
        assert_eq!(doc.root.children.len(), 3);

        // Check that paths are still paths
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.name, "path");
        }
        if let Some(Node::Element(elem)) = doc.root.children.get(1) {
            assert_eq!(elem.name, "rect");
        }
        if let Some(Node::Element(elem)) = doc.root.children.get(2) {
            assert_eq!(elem.name, "path");
        }
    }

    #[test]
    fn test_parse_config() {
        // Test default config
        let config = MergePathsPlugin::parse_config(&json!({})).unwrap();
        assert_eq!(config.force, false);
        assert_eq!(config.float_precision, 3);
        assert_eq!(config.no_space_after_flags, false);

        // Test custom config
        let config = MergePathsPlugin::parse_config(&json!({
            "force": true,
            "floatPrecision": 5,
            "noSpaceAfterFlags": true
        }))
        .unwrap();
        assert_eq!(config.force, true);
        assert_eq!(config.float_precision, 5);
        assert_eq!(config.no_space_after_flags, true);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(MergePathsPlugin, "mergePaths");
