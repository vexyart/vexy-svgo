// this_file: crates/plugin-sdk/src/plugins/convert_colors.rs

//! Convert colors plugin implementation
//!
//! This plugin converts colors between different formats:
//! - rgb() to hex
//! - color names to hex
//! - long hex to short hex
//! - hex to short color names
//! It follows the same logic as SVGO's convertColors plugin.

use crate::Plugin;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use vexy_svgo_core::ast::{Document, Element};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Configuration parameters for ConvertColors plugin
#[derive(Debug, Clone)]
pub struct ConvertColorsConfig {
    pub current_color: ConvertCurrentColor,
    pub names2hex: bool,
    pub rgb2hex: bool,
    pub convert_case: ConvertCase,
    pub shorthex: bool,
    pub shortname: bool,
}

#[derive(Debug, Clone)]
pub enum ConvertCurrentColor {
    False,
    Bool(bool),
    String(String),
    Regex(String), // Store as string since regex doesn't implement Clone easily
}

#[derive(Debug, Clone)]
pub enum ConvertCase {
    None,
    Lower,
    Upper,
}

impl Default for ConvertColorsConfig {
    fn default() -> Self {
        Self {
            current_color: ConvertCurrentColor::False,
            names2hex: true,
            rgb2hex: true,
            convert_case: ConvertCase::Lower,
            shorthex: true,
            shortname: true,
        }
    }
}

/// Plugin that converts colors between different formats
pub struct ConvertColorsPlugin {
    config: ConvertColorsConfig,
}

impl ConvertColorsPlugin {
    /// Create a new ConvertColorsPlugin with default configuration
    pub fn new() -> Self {
        Self {
            config: ConvertColorsConfig::default(),
        }
    }

    /// Create a new ConvertColorsPlugin with custom configuration
    pub fn with_config(config: ConvertColorsConfig) -> Self {
        Self { config }
    }
    
    /// Parse configuration from JSON parameters
    pub fn parse_config(params: &serde_json::Value) -> anyhow::Result<ConvertColorsConfig> {
        if params.is_null() {
            return Ok(ConvertColorsConfig::default());
        }
        
        let mut config = ConvertColorsConfig::default();
        
        if let Some(obj) = params.as_object() {
            for (key, value) in obj {
                match key.as_str() {
                    "currentColor" => {
                        if let Some(bool_val) = value.as_bool() {
                            config.current_color = if bool_val {
                                ConvertCurrentColor::Bool(true)
                            } else {
                                ConvertCurrentColor::Bool(false)
                            };
                        } else if let Some(str_val) = value.as_str() {
                            config.current_color = ConvertCurrentColor::String(str_val.to_string());
                        }
                    },
                    "names2hex" => {
                        if let Some(bool_val) = value.as_bool() {
                            config.names2hex = bool_val;
                        }
                    },
                    "rgb2hex" => {
                        if let Some(bool_val) = value.as_bool() {
                            config.rgb2hex = bool_val;
                        }
                    },
                    "shorthex" => {
                        if let Some(bool_val) = value.as_bool() {
                            config.shorthex = bool_val;
                        }
                    },
                    "shortname" => {
                        if let Some(bool_val) = value.as_bool() {
                            config.shortname = bool_val;
                        }
                    },
                    "convertCase" => {
                        if let Some(str_val) = value.as_str() {
                            match str_val {
                                "lower" => config.convert_case = ConvertCase::Lower,
                                "upper" => config.convert_case = ConvertCase::Upper,
                                _ => return Err(anyhow::anyhow!("Invalid convertCase value: {}", str_val)),
                            }
                        } else if value.as_bool() == Some(false) {
                            config.convert_case = ConvertCase::None;
                        }
                    },
                    _ => return Err(anyhow::anyhow!("Unknown parameter: {}", key)),
                }
            }
        }
        
        Ok(config)
    }

    /// Color name keywords to hex values
    fn color_names() -> &'static HashMap<&'static str, &'static str> {
        static COLOR_NAMES: std::sync::OnceLock<HashMap<&'static str, &'static str>> =
            std::sync::OnceLock::new();
        COLOR_NAMES.get_or_init(|| {
            [
                ("black", "#000000"),
                ("silver", "#c0c0c0"),
                ("gray", "#808080"),
                ("white", "#ffffff"),
                ("maroon", "#800000"),
                ("red", "#ff0000"),
                ("purple", "#800080"),
                ("fuchsia", "#ff00ff"),
                ("green", "#008000"),
                ("lime", "#00ff00"),
                ("olive", "#808000"),
                ("yellow", "#ffff00"),
                ("navy", "#000080"),
                ("blue", "#0000ff"),
                ("teal", "#008080"),
                ("aqua", "#00ffff"),
                // Extended color names
                ("aliceblue", "#f0f8ff"),
                ("antiquewhite", "#faebd7"),
                ("aquamarine", "#7fffd4"),
                ("azure", "#f0ffff"),
                ("beige", "#f5f5dc"),
                ("bisque", "#ffe4c4"),
                ("blanchedalmond", "#ffebcd"),
                ("blueviolet", "#8a2be2"),
                ("brown", "#a52a2a"),
                ("burlywood", "#deb887"),
                ("cadetblue", "#5f9ea0"),
                ("chartreuse", "#7fff00"),
                ("chocolate", "#d2691e"),
                ("coral", "#ff7f50"),
                ("cornflowerblue", "#6495ed"),
                ("cornsilk", "#fff8dc"),
                ("crimson", "#dc143c"),
                ("cyan", "#00ffff"),
                ("darkblue", "#00008b"),
                ("darkcyan", "#008b8b"),
                ("darkgoldenrod", "#b8860b"),
                ("darkgray", "#a9a9a9"),
                ("darkgreen", "#006400"),
                ("darkkhaki", "#bdb76b"),
                ("darkmagenta", "#8b008b"),
                ("darkolivegreen", "#556b2f"),
                ("darkorange", "#ff8c00"),
                ("darkorchid", "#9932cc"),
                ("darkred", "#8b0000"),
                ("darksalmon", "#e9967a"),
                ("darkseagreen", "#8fbc8f"),
                ("darkslateblue", "#483d8b"),
                ("darkslategray", "#2f4f4f"),
                ("darkturquoise", "#00ced1"),
                ("darkviolet", "#9400d3"),
                ("deeppink", "#ff1493"),
                ("deepskyblue", "#00bfff"),
                ("dimgray", "#696969"),
                ("dodgerblue", "#1e90ff"),
                ("firebrick", "#b22222"),
                ("floralwhite", "#fffaf0"),
                ("forestgreen", "#228b22"),
                ("gainsboro", "#dcdcdc"),
                ("ghostwhite", "#f8f8ff"),
                ("gold", "#ffd700"),
                ("goldenrod", "#daa520"),
                ("greenyellow", "#adff2f"),
                ("honeydew", "#f0fff0"),
                ("hotpink", "#ff69b4"),
                ("indianred", "#cd5c5c"),
                ("indigo", "#4b0082"),
                ("ivory", "#fffff0"),
                ("khaki", "#f0e68c"),
                ("lavender", "#e6e6fa"),
                ("lavenderblush", "#fff0f5"),
                ("lawngreen", "#7cfc00"),
                ("lemonchiffon", "#fffacd"),
                ("lightblue", "#add8e6"),
                ("lightcoral", "#f08080"),
                ("lightcyan", "#e0ffff"),
                ("lightgoldenrodyellow", "#fafad2"),
                ("lightgray", "#d3d3d3"),
                ("lightgreen", "#90ee90"),
                ("lightpink", "#ffb6c1"),
                ("lightsalmon", "#ffa07a"),
                ("lightseagreen", "#20b2aa"),
                ("lightskyblue", "#87cefa"),
                ("lightslategray", "#778899"),
                ("lightsteelblue", "#b0c4de"),
                ("lightyellow", "#ffffe0"),
                ("limegreen", "#32cd32"),
                ("linen", "#faf0e6"),
                ("magenta", "#ff00ff"),
                ("mediumaquamarine", "#66cdaa"),
                ("mediumblue", "#0000cd"),
                ("mediumorchid", "#ba55d3"),
                ("mediumpurple", "#9370db"),
                ("mediumseagreen", "#3cb371"),
                ("mediumslateblue", "#7b68ee"),
                ("mediumspringgreen", "#00fa9a"),
                ("mediumturquoise", "#48d1cc"),
                ("mediumvioletred", "#c71585"),
                ("midnightblue", "#191970"),
                ("mintcream", "#f5fffa"),
                ("mistyrose", "#ffe4e1"),
                ("moccasin", "#ffe4b5"),
                ("navajowhite", "#ffdead"),
                ("oldlace", "#fdf5e6"),
                ("olivedrab", "#6b8e23"),
                ("orange", "#ffa500"),
                ("orangered", "#ff4500"),
                ("orchid", "#da70d6"),
                ("palegoldenrod", "#eee8aa"),
                ("palegreen", "#98fb98"),
                ("paleturquoise", "#afeeee"),
                ("palevioletred", "#db7093"),
                ("papayawhip", "#ffefd5"),
                ("peachpuff", "#ffdab9"),
                ("peru", "#cd853f"),
                ("pink", "#ffc0cb"),
                ("plum", "#dda0dd"),
                ("powderblue", "#b0e0e6"),
                ("rosybrown", "#bc8f8f"),
                ("royalblue", "#4169e1"),
                ("saddlebrown", "#8b4513"),
                ("salmon", "#fa8072"),
                ("sandybrown", "#f4a460"),
                ("seagreen", "#2e8b57"),
                ("seashell", "#fff5ee"),
                ("sienna", "#a0522d"),
                ("skyblue", "#87ceeb"),
                ("slateblue", "#6a5acd"),
                ("slategray", "#708090"),
                ("snow", "#fffafa"),
                ("springgreen", "#00ff7f"),
                ("steelblue", "#4682b4"),
                ("tan", "#d2b48c"),
                ("thistle", "#d8bfd8"),
                ("tomato", "#ff6347"),
                ("turquoise", "#40e0d0"),
                ("violet", "#ee82ee"),
                ("wheat", "#f5deb3"),
                ("whitesmoke", "#f5f5f5"),
                ("yellowgreen", "#9acd32"),
            ]
            .into_iter()
            .collect()
        })
    }

    /// Short color names for hex values
    fn color_short_names() -> &'static HashMap<&'static str, &'static str> {
        static COLOR_SHORT_NAMES: std::sync::OnceLock<HashMap<&'static str, &'static str>> =
            std::sync::OnceLock::new();
        COLOR_SHORT_NAMES.get_or_init(|| {
            [
                ("#000000", "black"),
                ("#000", "black"),
                ("#000080", "navy"),
                ("#008", "navy"),
                ("#008000", "green"),
                ("#080", "green"),
                ("#008080", "teal"),
                ("#088", "teal"),
                ("#4b0082", "indigo"),
                ("#800000", "maroon"),
                ("#800", "maroon"),
                ("#800080", "purple"),
                ("#808", "purple"),
                ("#808000", "olive"),
                ("#880", "olive"),
                ("#808080", "gray"),
                ("#888", "gray"),
                ("#a0522d", "sienna"),
                ("#a52a2a", "brown"),
                ("#c0c0c0", "silver"),
                ("#ccc", "silver"),
                ("#cd853f", "peru"),
                ("#d2b48c", "tan"),
                ("#da70d6", "orchid"),
                ("#dda0dd", "plum"),
                ("#ee82ee", "violet"),
                ("#f0e68c", "khaki"),
                ("#f0ffff", "azure"),
                ("#f5deb3", "wheat"),
                ("#f5f5dc", "beige"),
                ("#fa8072", "salmon"),
                ("#faf0e6", "linen"),
                ("#ff0000", "red"),
                ("#f00", "red"),
                ("#ff6347", "tomato"),
                ("#ff7f50", "coral"),
                ("#ffa500", "orange"),
                ("#ffc0cb", "pink"),
                ("#ffd700", "gold"),
                ("#ffe4c4", "bisque"),
                ("#fffafa", "snow"),
                ("#fffff0", "ivory"),
                ("#ffffff", "white"),
                ("#fff", "white"),
            ]
            .into_iter()
            .collect()
        })
    }

    /// Color properties that can contain color values
    fn color_props() -> &'static std::collections::HashSet<&'static str> {
        static COLOR_PROPS: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
            std::sync::OnceLock::new();
        COLOR_PROPS.get_or_init(|| {
            [
                "color",
                "fill",
                "stroke",
                "stop-color",
                "flood-color",
                "lighting-color",
            ]
            .into_iter()
            .collect()
        })
    }

    /// Convert RGB components to hex
    fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    /// Convert hex color to short hex if possible
    fn hex_to_short_hex(hex: &str) -> Option<String> {
        if hex.len() == 7 && hex.starts_with('#') {
            let chars: Vec<char> = hex.chars().collect();
            if chars[1] == chars[2] && chars[3] == chars[4] && chars[5] == chars[6] {
                return Some(format!("#{}{}{}", chars[1], chars[3], chars[5]));
            }
        }
        None
    }

    /// Parse RGB function
    fn parse_rgb(value: &str) -> Option<(u8, u8, u8)> {
        // RGB regex pattern
        let rgb_regex = Regex::new(r"^rgb\(\s*([+-]?(?:\d*\.\d+|\d+\.?)%?)\s*[,\s]+\s*([+-]?(?:\d*\.\d+|\d+\.?)%?)\s*[,\s]+\s*([+-]?(?:\d*\.\d+|\d+\.?)%?)\s*\)$").ok()?;

        if let Some(captures) = rgb_regex.captures(value) {
            let mut components = Vec::new();

            for i in 1..=3 {
                let component_str = captures.get(i)?.as_str();
                let component = if component_str.contains('%') {
                    // Percentage value
                    let percentage = component_str.trim_end_matches('%').parse::<f64>().ok()?;
                    (percentage * 2.55).round() as i32
                } else {
                    // Numeric value
                    component_str.parse::<i32>().ok()?
                };

                // Clamp to 0-255 range
                components.push((component.max(0).min(255)) as u8);
            }

            if components.len() == 3 {
                return Some((components[0], components[1], components[2]));
            }
        }

        None
    }

    /// Check if value includes URL reference
    fn includes_url_reference(value: &str) -> bool {
        value.contains("url(")
    }

    /// Convert color value
    fn convert_color_value(&self, value: &str, in_mask: bool) -> String {
        let mut val = value.to_string();
        
        // Check for special values that should not be converted to currentColor
        let special_values = ["none", "inherit", "transparent", "currentColor"];
        let is_special = special_values.iter().any(|&special| val.eq_ignore_ascii_case(special));

        // Convert colors to currentColor
        match &self.config.current_color {
            ConvertCurrentColor::Bool(true) => {
                if !in_mask && !is_special {
                    val = "currentColor".to_string();
                }
            }
            ConvertCurrentColor::String(ref target) => {
                if !in_mask && !is_special && val == *target {
                    val = "currentColor".to_string();
                }
            }
            ConvertCurrentColor::Regex(ref pattern) => {
                if !in_mask && !is_special {
                    if let Ok(regex) = Regex::new(pattern) {
                        if regex.is_match(&val) {
                            val = "currentColor".to_string();
                        }
                    }
                }
            }
            _ => {}
        }

        // Convert color name keyword to long hex
        if self.config.names2hex {
            let color_name = val.to_lowercase();
            if let Some(hex_value) = Self::color_names().get(color_name.as_str()) {
                val = hex_value.to_string();
            }
        }

        // Convert rgb() to long hex
        if self.config.rgb2hex {
            if let Some((r, g, b)) = Self::parse_rgb(&val) {
                val = Self::rgb_to_hex(r, g, b);
            }
        }

        // Convert case (but not for special values)
        let is_special_for_case = ["none", "inherit", "transparent", "currentColor"]
            .iter()
            .any(|&special| val.eq_ignore_ascii_case(special));
        if !Self::includes_url_reference(&val) && !is_special_for_case {
            match self.config.convert_case {
                ConvertCase::Lower => val = val.to_lowercase(),
                ConvertCase::Upper => val = val.to_uppercase(),
                ConvertCase::None => {}
            }
        }

        // Convert long hex to short hex
        if self.config.shorthex {
            if let Some(short_hex) = Self::hex_to_short_hex(&val) {
                val = short_hex;
            }
        }

        // Convert hex to short name (but not inside masks)
        if self.config.shortname && !in_mask {
            let color_name = val.to_lowercase();
            if let Some(short_name) = Self::color_short_names().get(color_name.as_str()) {
                val = short_name.to_string();
            }
        }


        val
    }
}

impl Default for ConvertColorsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::PluginWithParams for ConvertColorsPlugin {
    type Config = ConvertColorsConfig;
    
    fn with_config(config: Self::Config) -> Self {
        Self::with_config(config)
    }
    
    fn parse_config(params: &serde_json::Value) -> anyhow::Result<Self::Config> {
        Self::parse_config(params)
    }
}

impl Plugin for ConvertColorsPlugin {
    fn name(&self) -> &'static str {
        "convertColors"
    }

    fn description(&self) -> &'static str {
        "Convert colors: rgb() to #rrggbb and #rrggbb to #rgb"
    }

    fn validate_params(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        if let Some(obj) = params.as_object() {
            for (key, value) in obj {
                match key.as_str() {
                    "currentColor" => {
                        if !value.is_boolean() && !value.is_string() {
                            return Err(anyhow::anyhow!(
                                "currentColor must be a boolean or string"
                            ));
                        }
                    }
                    "names2hex" | "rgb2hex" | "shorthex" | "shortname" => {
                        if !value.is_boolean() {
                            return Err(anyhow::anyhow!("{} must be a boolean", key));
                        }
                    }
                    "convertCase" => {
                        if let Some(case_str) = value.as_str() {
                            match case_str {
                                "lower" | "upper" => {}
                                _ => {
                                    return Err(anyhow::anyhow!(
                                        "convertCase must be 'lower' or 'upper'"
                                    ))
                                }
                            }
                        } else if value.as_bool() == Some(false) {
                            // false is allowed
                        } else {
                            return Err(anyhow::anyhow!(
                                "convertCase must be false, 'lower', or 'upper'"
                            ));
                        }
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unknown parameter: {}", key));
                    }
                }
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let mut visitor = ColorConversionVisitor::new(self.config.clone());
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that converts colors
struct ColorConversionVisitor {
    config: ConvertColorsConfig,
    mask_counter: usize,
}

impl ColorConversionVisitor {
    fn new(config: ConvertColorsConfig) -> Self {
        Self {
            config,
            mask_counter: 0,
        }
    }
}

impl Visitor<'_> for ColorConversionVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Track mask elements
        if element.name == "mask" {
            self.mask_counter += 1;
        }

        // Convert color attributes
        let plugin = ConvertColorsPlugin::with_config(self.config.clone());
        let mut attrs_to_update = Vec::new();

        for (attr_name, attr_value) in &element.attributes {
            if ConvertColorsPlugin::color_props().contains(attr_name.as_ref()) {
                let converted_value = plugin.convert_color_value(attr_value, self.mask_counter > 0);
                if converted_value != *attr_value {
                    attrs_to_update.push((attr_name.clone(), converted_value));
                }
            }
        }

        // Update attributes
        for (attr_name, new_value) in attrs_to_update {
            element.attributes.insert(attr_name, new_value.into());
        }

        Ok(())
    }

    fn visit_element_exit(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Track mask elements
        if element.name == "mask" {
            self.mask_counter = self.mask_counter.saturating_sub(1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::borrow::Cow;
    use vexy_svgo_core::ast::{Document, Element, Node};

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    fn create_element_with_attrs(name: &'static str, attrs: &[(&str, &str)]) -> Element<'static> {
        let mut element = create_element(name);
        for (key, value) in attrs {
            element.set_attr(*key, *value);
        }
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = ConvertColorsPlugin::new();
        assert_eq!(plugin.name(), "convertColors");
        assert!(plugin.config.names2hex);
        assert!(plugin.config.rgb2hex);
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = ConvertColorsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin.validate_params(&json!({"names2hex": true})).is_ok());
        assert!(plugin
            .validate_params(&json!({"convertCase": "lower"}))
            .is_ok());
        assert!(plugin
            .validate_params(&json!({"convertCase": false}))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"names2hex": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"convertCase": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"invalidParam": true}))
            .is_err());
    }

    #[test]
    fn test_color_names() {
        let names = ConvertColorsPlugin::color_names();
        assert_eq!(names.get("black"), Some(&"#000000"));
        assert_eq!(names.get("white"), Some(&"#ffffff"));
        assert_eq!(names.get("red"), Some(&"#ff0000"));
    }

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(ConvertColorsPlugin::rgb_to_hex(255, 0, 0), "#FF0000");
        assert_eq!(ConvertColorsPlugin::rgb_to_hex(0, 255, 0), "#00FF00");
        assert_eq!(ConvertColorsPlugin::rgb_to_hex(0, 0, 255), "#0000FF");
        assert_eq!(ConvertColorsPlugin::rgb_to_hex(64, 64, 64), "#404040");
    }

    #[test]
    fn test_hex_to_short_hex() {
        assert_eq!(
            ConvertColorsPlugin::hex_to_short_hex("#aabbcc"),
            Some("#abc".to_string())
        );
        assert_eq!(
            ConvertColorsPlugin::hex_to_short_hex("#000000"),
            Some("#000".to_string())
        );
        assert_eq!(ConvertColorsPlugin::hex_to_short_hex("#123456"), None);
        assert_eq!(ConvertColorsPlugin::hex_to_short_hex("#abc"), None);
    }

    #[test]
    fn test_parse_rgb() {
        assert_eq!(
            ConvertColorsPlugin::parse_rgb("rgb(255, 0, 0)"),
            Some((255, 0, 0))
        );
        assert_eq!(
            ConvertColorsPlugin::parse_rgb("rgb(64, 64, 64)"),
            Some((64, 64, 64))
        );
        assert_eq!(
            ConvertColorsPlugin::parse_rgb("rgb(64 64 64)"),
            Some((64, 64, 64))
        );
        assert_eq!(
            ConvertColorsPlugin::parse_rgb("rgb(100%, 0%, 0%)"),
            Some((255, 0, 0))
        );
        assert_eq!(
            ConvertColorsPlugin::parse_rgb("rgb(-255, 100, 500)"),
            Some((0, 100, 255))
        );
        assert_eq!(ConvertColorsPlugin::parse_rgb("invalid"), None);
    }

    #[test]
    fn test_convert_color_value() {
        let plugin = ConvertColorsPlugin::new();

        // Color name conversion (shortname now enabled by default)
        assert_eq!(plugin.convert_color_value("black", false), "black");
        assert_eq!(plugin.convert_color_value("RED", false), "red");

        // RGB conversion
        assert_eq!(plugin.convert_color_value("rgb(255, 0, 0)", false), "red");
        assert_eq!(
            plugin.convert_color_value("rgb(64, 64, 64)", false),
            "#404040"
        );

        // Hex shortening
        assert_eq!(plugin.convert_color_value("#aabbcc", false), "#abc");
        assert_eq!(plugin.convert_color_value("#000000", false), "black");

        // No change for invalid values
        assert_eq!(plugin.convert_color_value("invalid", false), "invalid");
    }

    #[test]
    fn test_convert_color_value_with_current_color() {
        let mut config = ConvertColorsConfig::default();
        config.current_color = ConvertCurrentColor::Bool(true);
        // Disable other conversions for this test
        config.names2hex = false;
        config.rgb2hex = false;
        config.shorthex = false;
        config.shortname = false;
        
        let plugin = ConvertColorsPlugin::with_config(config);
        
        // Regular colors should be converted to currentColor
        assert_eq!(plugin.convert_color_value("black", false), "currentColor");
        assert_eq!(plugin.convert_color_value("RED", false), "currentColor");
        assert_eq!(plugin.convert_color_value("rgb(255, 0, 0)", false), "currentColor");
        assert_eq!(plugin.convert_color_value("#ff0000", false), "currentColor");
        
        // Special values should NOT be converted to currentColor and should preserve their case
        assert_eq!(plugin.convert_color_value("none", false), "none");
        assert_eq!(plugin.convert_color_value("NONE", false), "NONE"); // case preserved for special values
        assert_eq!(plugin.convert_color_value("inherit", false), "inherit");
        assert_eq!(plugin.convert_color_value("transparent", false), "transparent");
        assert_eq!(plugin.convert_color_value("currentColor", false), "currentColor");
    }
    
    #[test]
    fn test_convert_color_value_in_mask() {
        let mut config = ConvertColorsConfig::default();
        config.current_color = ConvertCurrentColor::Bool(true);
        
        let plugin = ConvertColorsPlugin::with_config(config);
        
        // Inside mask (in_mask=true): Should NOT convert to currentColor but should do other conversions
        assert_eq!(plugin.convert_color_value("white", true), "#fff");  // names2hex -> shorthex (shortname disabled in masks)
        assert_eq!(plugin.convert_color_value("black", true), "#000");  // names2hex -> shorthex (shortname disabled in masks)
        assert_eq!(plugin.convert_color_value("red", true), "#f00");    // names2hex -> shorthex (shortname disabled in masks)
        
        // Outside mask (in_mask=false): Should convert to currentColor
        assert_eq!(plugin.convert_color_value("white", false), "currentColor");
        assert_eq!(plugin.convert_color_value("black", false), "currentColor");
        assert_eq!(plugin.convert_color_value("red", false), "currentColor");
    }

    #[test]
    fn test_plugin_apply() {
        let plugin = ConvertColorsPlugin::new();
        let mut doc = Document::new();

        // Create element with color attributes
        let element = create_element_with_attrs(
            "rect",
            &[
                ("fill", "red"),
                ("stroke", "rgb(0, 255, 0)"),
                ("color", "#aabbcc"),
            ],
        );

        doc.root.children.push(Node::Element(element));

        plugin.apply(&mut doc).unwrap();

        if let Some(Node::Element(rect)) = doc.root.children.first() {
            assert_eq!(rect.attr("fill"), Some("red"));
            assert_eq!(rect.attr("stroke"), Some("#0f0"));
            assert_eq!(rect.attr("color"), Some("#abc"));
        }
    }

    #[test]
    fn test_mask_counter() {
        let plugin = ConvertColorsPlugin::new();
        let mut doc = Document::new();

        // Create mask element with nested colored element
        let mut mask = create_element("mask");
        let colored_element = create_element_with_attrs("rect", &[("fill", "red")]);
        mask.children.push(Node::Element(colored_element));

        doc.root.children.push(Node::Element(mask));

        plugin.apply(&mut doc).unwrap();

        // Should still convert colors inside mask but not to color names
        if let Some(Node::Element(mask_elem)) = doc.root.children.first() {
            if let Some(Node::Element(rect)) = mask_elem.children.first() {
                assert_eq!(rect.attr("fill"), Some("#f00"));  // red -> #ff0000 -> #f00 (shortname disabled in masks)
            }
        }
    }
}

// Custom fixture tests for ConvertColorsPlugin with parameter support
#[cfg(test)]
mod fixture_tests {
    use super::*;
    
    // TODO: Re-enable this test when test utilities are properly imported
    // #[test]
    // fn test_plugin_with_fixtures() {
    //     let fixtures = load_plugin_fixtures("convertColors").unwrap();
    //     
    //     if fixtures.is_empty() {
    //         println!("No fixtures found for plugin: convertColors");
    //         return;
    //     }
    //     
    //     for fixture in fixtures {
    //         println!("Testing fixture: {}", fixture.name);
    //         
    //         // Create plugin instance with parameters
    //         let mut plugin = if let Some(ref params) = fixture.params {
    //             let config = ConvertColorsPlugin::parse_config(params).unwrap_or_else(|e| {
    //                 panic!("Failed to parse config for fixture {}: {}", fixture.name, e)
    //             });
    //             ConvertColorsPlugin::with_config(config)
    //         } else {
    //             ConvertColorsPlugin::default()
    //         };
    //         
    //         // Apply plugin to input
    //         let result = apply_plugin_to_svg(&mut plugin, &fixture.input, fixture.params.as_ref()).unwrap_or_else(|e| {
    //             panic!("Failed to apply plugin to fixture {}: {}", fixture.name, e)
    //         });
    //         
    //         // Compare result with expected output
    //         if !compare_svg(&result, &fixture.expected) {
    //             panic!(
    //                 "Fixture {} failed\nInput:\n{}\nExpected:\n{}\nActual:\n{}",
    //                 fixture.name, fixture.input, fixture.expected, result
    //             );
    //         }
    //     }
    // }
}
