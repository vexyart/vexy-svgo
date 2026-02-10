// this_file: crates/plugin-sdk/src/plugins/convert_transform.rs

//! Collapses multiple transformations and optimizes it
//!
//! This plugin collapses multiple transformations, converts matrices to short aliases,
//! converts long transform notations to short ones, and removes useless transforms.
//!
//! Reference: SVGO's convertTransform plugin

use crate::Plugin;
use anyhow::Result;
use nalgebra::Matrix3;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::f64::consts::PI;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration for the convertTransform plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertTransformConfig {
    /// Convert matrices to shorter aliases (translate, scale, rotate)
    #[serde(default = "default_true")]
    pub convert_to_shorts: bool,
    /// Precision for degree values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deg_precision: Option<u8>,
    /// Precision for float values
    #[serde(default = "default_float_precision")]
    pub float_precision: u8,
    /// Precision for transform values (matrix parameters)
    #[serde(default = "default_transform_precision")]
    pub transform_precision: u8,
    /// Convert matrices to transform functions
    #[serde(default = "default_true")]
    pub matrix_to_transform: bool,
    /// Use short translate notation
    #[serde(default = "default_true")]
    pub short_translate: bool,
    /// Use short scale notation
    #[serde(default = "default_true")]
    pub short_scale: bool,
    /// Use short rotate notation
    #[serde(default = "default_true")]
    pub short_rotate: bool,
    /// Remove useless transforms
    #[serde(default = "default_true")]
    pub remove_useless: bool,
    /// Collapse multiple transforms into one
    #[serde(default = "default_true")]
    pub collapse_into_one: bool,
    /// Include leading zero
    #[serde(default = "default_true")]
    pub leading_zero: bool,
    /// Add extra space for negative values
    #[serde(default)]
    pub negative_extra_space: bool,
}

fn default_true() -> bool {
    true
}

fn default_float_precision() -> u8 {
    3
}

fn default_transform_precision() -> u8 {
    5
}

impl Default for ConvertTransformConfig {
    fn default() -> Self {
        Self {
            convert_to_shorts: true,
            deg_precision: None,
            float_precision: 3,
            transform_precision: 5,
            matrix_to_transform: true,
            short_translate: true,
            short_scale: true,
            short_rotate: true,
            remove_useless: true,
            collapse_into_one: true,
            leading_zero: true,
            negative_extra_space: false,
        }
    }
}

/// Represents a single transform operation
#[derive(Debug, Clone, PartialEq)]
struct Transform {
    name: String,
    data: Vec<f64>,
}

impl Transform {
    /// Create a new transform
    fn new(name: String, data: Vec<f64>) -> Self {
        Self { name, data }
    }

    /// Convert to matrix representation
    fn to_matrix(&self) -> Matrix3<f64> {
        match self.name.as_str() {
            "matrix" => {
                if self.data.len() >= 6 {
                    Matrix3::new(
                        self.data[0],
                        self.data[2],
                        self.data[4],
                        self.data[1],
                        self.data[3],
                        self.data[5],
                        0.0,
                        0.0,
                        1.0,
                    )
                } else {
                    Matrix3::identity()
                }
            }
            "translate" => {
                let tx = self.data.first().copied().unwrap_or(0.0);
                let ty = self.data.get(1).copied().unwrap_or(0.0);
                Matrix3::new(1.0, 0.0, tx, 0.0, 1.0, ty, 0.0, 0.0, 1.0)
            }
            "scale" => {
                let sx = self.data.first().copied().unwrap_or(1.0);
                let sy = self.data.get(1).copied().unwrap_or(sx);
                Matrix3::new(sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0)
            }
            "rotate" => {
                let angle = self.data.first().copied().unwrap_or(0.0) * PI / 180.0;
                let cx = self.data.get(1).copied().unwrap_or(0.0);
                let cy = self.data.get(2).copied().unwrap_or(0.0);

                let cos_a = angle.cos();
                let sin_a = angle.sin();

                if cx == 0.0 && cy == 0.0 {
                    Matrix3::new(cos_a, -sin_a, 0.0, sin_a, cos_a, 0.0, 0.0, 0.0, 1.0)
                } else {
                    // rotate(angle, cx, cy) = translate(cx, cy) rotate(angle) translate(-cx, -cy)
                    let translate_to = Matrix3::new(1.0, 0.0, cx, 0.0, 1.0, cy, 0.0, 0.0, 1.0);
                    let rotate = Matrix3::new(cos_a, -sin_a, 0.0, sin_a, cos_a, 0.0, 0.0, 0.0, 1.0);
                    let translate_back = Matrix3::new(1.0, 0.0, -cx, 0.0, 1.0, -cy, 0.0, 0.0, 1.0);
                    translate_to * rotate * translate_back
                }
            }
            "skewX" => {
                let angle = self.data.first().copied().unwrap_or(0.0) * PI / 180.0;
                Matrix3::new(1.0, angle.tan(), 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
            }
            "skewY" => {
                let angle = self.data.first().copied().unwrap_or(0.0) * PI / 180.0;
                Matrix3::new(1.0, 0.0, 0.0, angle.tan(), 1.0, 0.0, 0.0, 0.0, 1.0)
            }
            _ => Matrix3::identity(),
        }
    }
}

/// Plugin to convert and optimize transform attributes
pub struct ConvertTransformPlugin {
    config: ConvertTransformConfig,
}

impl ConvertTransformPlugin {
    pub fn new() -> Self {
        Self {
            config: ConvertTransformConfig::default(),
        }
    }

    pub fn with_config(config: ConvertTransformConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<ConvertTransformConfig> {
        if params.is_null() {
            Ok(ConvertTransformConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    /// Parse transform string to transform operations
    fn parse_transform_string(&self, transform_str: &str) -> Vec<Transform> {
        let mut transforms = Vec::new();

        // Regex pattern to match transform functions
        let re = regex::Regex::new(
            r"\s*(matrix|translate|scale|rotate|skewX|skewY)\s*\(\s*([^)]*)\s*\)",
        )
        .unwrap();

        for cap in re.captures_iter(transform_str) {
            if let (Some(name_match), Some(data_match)) = (cap.get(1), cap.get(2)) {
                let name = name_match.as_str().to_string();
                let data_str = data_match.as_str();

                // Parse numeric values
                let data: Vec<f64> = data_str
                    .split(',')
                    .flat_map(|s| s.split_whitespace())
                    .filter_map(|s| s.parse().ok())
                    .collect();

                transforms.push(Transform::new(name, data));
            }
        }

        transforms
    }

    /// Convert transforms to optimized form
    fn optimize_transforms(&self, transforms: Vec<Transform>) -> Vec<Transform> {
        if transforms.is_empty() {
            return transforms;
        }

        let mut result = transforms;

        // Collapse into one matrix if requested
        if self.config.collapse_into_one && result.len() > 1 {
            let mut combined_matrix = Matrix3::identity();
            for transform in &result {
                combined_matrix *= transform.to_matrix();
            }
            result = vec![self.matrix_to_transform(combined_matrix)];
        }

        // Convert to shorts if requested
        if self.config.convert_to_shorts {
            result = result
                .into_iter()
                .map(|t| self.convert_to_short(t))
                .collect();
        }

        // Remove useless transforms
        if self.config.remove_useless {
            result = self.remove_useless_transforms(result);
        }

        result
    }

    /// Convert matrix to transform function
    fn matrix_to_transform(&self, matrix: Matrix3<f64>) -> Transform {
        let a = matrix[(0, 0)];
        let b = matrix[(1, 0)];
        let c = matrix[(0, 1)];
        let d = matrix[(1, 1)];
        let e = matrix[(0, 2)];
        let f = matrix[(1, 2)];

        // Check for simple transforms first
        if self.config.matrix_to_transform {
            // Pure translation
            if a == 1.0 && b == 0.0 && c == 0.0 && d == 1.0 {
                return Transform::new("translate".to_string(), vec![e, f]);
            }

            // Pure scale
            if b == 0.0 && c == 0.0 && e == 0.0 && f == 0.0 {
                return Transform::new("scale".to_string(), vec![a, d]);
            }

            // Pure rotation (no translation)
            if e == 0.0 && f == 0.0 && (a * a + b * b - 1.0).abs() < 1e-10 {
                let angle = b.atan2(a) * 180.0 / PI;
                return Transform::new("rotate".to_string(), vec![angle]);
            }
        }

        // Fallback to matrix
        Transform::new("matrix".to_string(), vec![a, b, c, d, e, f])
    }

    /// Convert transform to shorter notation if possible
    fn convert_to_short(&self, transform: Transform) -> Transform {
        match transform.name.as_str() {
            "translate" => {
                if self.config.short_translate
                    && transform.data.len() >= 2
                    && transform.data[1] == 0.0
                {
                    Transform::new("translate".to_string(), vec![transform.data[0]])
                } else {
                    transform
                }
            }
            "scale" => {
                if self.config.short_scale
                    && transform.data.len() >= 2
                    && transform.data[0] == transform.data[1]
                {
                    Transform::new("scale".to_string(), vec![transform.data[0]])
                } else {
                    transform
                }
            }
            _ => transform,
        }
    }

    /// Remove useless transforms
    fn remove_useless_transforms(&self, transforms: Vec<Transform>) -> Vec<Transform> {
        transforms
            .into_iter()
            .filter(|t| !self.is_useless_transform(t))
            .collect()
    }

    /// Check if transform is useless (identity)
    fn is_useless_transform(&self, transform: &Transform) -> bool {
        match transform.name.as_str() {
            "translate" => {
                transform.data.is_empty()
                    || (!transform.data.is_empty()
                        && transform.data[0] == 0.0
                        && (transform.data.len() == 1 || transform.data[1] == 0.0))
            }
            "scale" => {
                transform.data.is_empty()
                    || (!transform.data.is_empty()
                        && transform.data[0] == 1.0
                        && (transform.data.len() == 1 || transform.data[1] == 1.0))
            }
            "rotate" => transform.data.is_empty() || transform.data[0] == 0.0,
            "skewX" | "skewY" => transform.data.is_empty() || transform.data[0] == 0.0,
            "matrix" => {
                transform.data.len() >= 6 &&
                transform.data[0] == 1.0 && // a
                transform.data[1] == 0.0 && // b
                transform.data[2] == 0.0 && // c
                transform.data[3] == 1.0 && // d
                transform.data[4] == 0.0 && // e
                transform.data[5] == 0.0 // f
            }
            _ => false,
        }
    }

    /// Convert transforms back to string
    fn transforms_to_string(&self, transforms: Vec<Transform>) -> String {
        if transforms.is_empty() {
            return String::new();
        }

        transforms
            .iter()
            .map(|t| {
                let data_str = t
                    .data
                    .iter()
                    .map(|&val| self.format_number(val))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}({})", t.name, data_str)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Format number according to precision settings
    fn format_number(&self, val: f64) -> String {
        let precision = self.config.float_precision;

        let formatted = if precision == 0 {
            format!("{:.0}", val)
        } else {
            format!("{:.prec$}", val, prec = precision as usize)
        };

        // Remove trailing zeros after decimal point
        if formatted.contains('.') {
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            formatted
        }
    }

    /// Process element recursively
    fn process_element(&self, element: &mut Element) {
        // Process transform attribute
        if let Some(transform_value) = element.attr("transform") {
            let transforms = self.parse_transform_string(transform_value);
            let optimized = self.optimize_transforms(transforms);

            if optimized.is_empty() {
                element.remove_attr("transform");
            } else {
                let new_value = self.transforms_to_string(optimized);
                element.set_attr("transform", &new_value);
            }
        }

        // Process gradientTransform attribute
        if let Some(transform_value) = element.attr("gradientTransform") {
            let transforms = self.parse_transform_string(transform_value);
            let optimized = self.optimize_transforms(transforms);

            if optimized.is_empty() {
                element.remove_attr("gradientTransform");
            } else {
                let new_value = self.transforms_to_string(optimized);
                element.set_attr("gradientTransform", &new_value);
            }
        }

        // Process patternTransform attribute
        if let Some(transform_value) = element.attr("patternTransform") {
            let transforms = self.parse_transform_string(transform_value);
            let optimized = self.optimize_transforms(transforms);

            if optimized.is_empty() {
                element.remove_attr("patternTransform");
            } else {
                let new_value = self.transforms_to_string(optimized);
                element.set_attr("patternTransform", &new_value);
            }
        }

        // Process children
        let mut i = 0;
        while i < element.children.len() {
            if let Node::Element(child) = &mut element.children[i] {
                self.process_element(child);
            }
            i += 1;
        }
    }
}

impl Default for ConvertTransformPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ConvertTransformPlugin {
    fn name(&self) -> &'static str {
        "convertTransform"
    }

    fn description(&self) -> &'static str {
        "collapses multiple transformations and optimizes it"
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
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde_json::json;
    use std::borrow::Cow;

    fn create_test_element_with_transform(transform: &str) -> Element {
        let mut attributes = IndexMap::new();
        attributes.insert(Cow::Borrowed("transform"), Cow::Owned(transform.to_string()));
        Element {
            name: "rect".into(),
            attributes,
            children: vec![],
            namespaces: IndexMap::new(),
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = ConvertTransformPlugin::new();
        assert_eq!(plugin.name(), "convertTransform");
        assert_eq!(
            plugin.description(),
            "collapses multiple transformations and optimizes it"
        );
    }

    #[test]
    fn test_param_validation() {
        let plugin = ConvertTransformPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test valid params
        assert!(plugin
            .validate_params(&json!({
                "convertToShorts": true,
                "floatPrecision": 3,
                "transformPrecision": 5,
                "matrixToTransform": true,
                "shortTranslate": true,
                "shortScale": true,
                "shortRotate": true,
                "removeUseless": true,
                "collapseIntoOne": true,
                "leadingZero": true,
                "negativeExtraSpace": false
            }))
            .is_ok());

        // Test invalid params
        assert!(plugin
            .validate_params(&json!({
                "invalidParam": true
            }))
            .is_err());
    }

    #[test]
    fn test_parse_transform_string() {
        let plugin = ConvertTransformPlugin::new();
        let transforms = plugin.parse_transform_string("translate(10,20) scale(2)");

        assert_eq!(transforms.len(), 2);
        assert_eq!(transforms[0].name, "translate");
        assert_eq!(transforms[0].data, vec![10.0, 20.0]);
        assert_eq!(transforms[1].name, "scale");
        assert_eq!(transforms[1].data, vec![2.0]);
    }

    #[test]
    fn test_remove_useless_transforms() {
        let plugin = ConvertTransformPlugin::new();

        // Identity transforms
        assert!(
            plugin.is_useless_transform(&Transform::new("translate".to_string(), vec![0.0, 0.0]))
        );
        assert!(plugin.is_useless_transform(&Transform::new("scale".to_string(), vec![1.0, 1.0])));
        assert!(plugin.is_useless_transform(&Transform::new("rotate".to_string(), vec![0.0])));

        // Non-identity transforms
        assert!(
            !plugin.is_useless_transform(&Transform::new("translate".to_string(), vec![10.0, 0.0]))
        );
        assert!(!plugin.is_useless_transform(&Transform::new("scale".to_string(), vec![2.0, 1.0])));
        assert!(!plugin.is_useless_transform(&Transform::new("rotate".to_string(), vec![45.0])));
    }

    #[test]
    fn test_plugin_removes_identity_transform() {
        let mut doc = Document::default();
        doc.root = create_test_element_with_transform("translate(0,0)");

        let plugin = ConvertTransformPlugin::new();
        plugin.apply(&mut doc).unwrap();

        // Transform should be removed
        assert!(!doc.root.has_attr("transform"));
    }

    #[test]
    fn test_plugin_optimizes_transform() {
        let mut doc = Document::default();
        doc.root = create_test_element_with_transform("translate(10,0)");

        let config = ConvertTransformConfig {
            short_translate: true,
            ..Default::default()
        };
        let plugin = ConvertTransformPlugin::with_config(config);
        plugin.apply(&mut doc).unwrap();

        // Should be shortened to single parameter
        assert_eq!(doc.root.attr("transform"), Some("translate(10)"));
    }
}
