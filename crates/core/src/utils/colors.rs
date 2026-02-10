// this_file: crates/core/src/utils/colors.rs

//! Color utilities for SVG processing
//!
//! Common functions for working with colors across multiple plugins.

use regex::Regex;

/// Color conversion utilities
pub struct ColorUtils;

impl ColorUtils {
    /// Parse RGB color string to components
    pub fn parse_rgb(value: &str) -> Option<(u8, u8, u8)> {
        let rgb_regex = Regex::new(r"^rgb\(\s*([+-]?(?:\d*\.\d+|\d+\.?)%?)\s*[,\s]+\s*([+-]?(?:\d*\.\d+|\d+\.?)%?)\s*[,\s]+\s*([+-]?(?:\d*\.\d+|\d+\.?)%?)\s*\)$").ok()?;

        if let Some(captures) = rgb_regex.captures(value) {
            let mut components = Vec::new();

            for i in 1..=3 {
                let component_str = captures.get(i)?.as_str();
                let component = if component_str.contains('%') {
                    let percentage = component_str.trim_end_matches('%').parse::<f64>().ok()?;
                    (percentage * 2.55).round() as i32
                } else {
                    component_str.parse::<i32>().ok()?
                };

                components.push((component.clamp(0, 255)) as u8);
            }

            if components.len() == 3 {
                return Some((components[0], components[1], components[2]));
            }
        }

        None
    }

    /// Convert RGB components to hex
    pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// Convert hex color to short hex if possible
    pub fn hex_to_short_hex(hex: &str) -> Option<String> {
        if hex.len() == 7 && hex.starts_with('#') {
            let chars: Vec<char> = hex.chars().collect();
            if chars[1] == chars[2] && chars[3] == chars[4] && chars[5] == chars[6] {
                return Some(format!("#{}{}{}", chars[1], chars[3], chars[5]));
            }
        }
        None
    }

    /// Check if a color is valid
    pub fn is_valid_color(color: &str) -> bool {
        // Check for hex colors
        if color.starts_with('#') && (color.len() == 4 || color.len() == 7) {
            return color.chars().skip(1).all(|c| c.is_ascii_hexdigit());
        }

        // Check for rgb colors
        if color.starts_with("rgb(") && color.ends_with(')') {
            return Self::parse_rgb(color).is_some();
        }

        // Check for named colors
        Self::is_named_color(color)
    }

    /// Check if a color is a named color
    pub fn is_named_color(color: &str) -> bool {
        const NAMED_COLORS: &[&str] = &[
            "aliceblue",
            "antiquewhite",
            "aqua",
            "aquamarine",
            "azure",
            "beige",
            "bisque",
            "black",
            "blanchedalmond",
            "blue",
            "blueviolet",
            "brown",
            "burlywood",
            "cadetblue",
            "chartreuse",
            "chocolate",
            "coral",
            "cornflowerblue",
            "cornsilk",
            "crimson",
            "cyan",
            "darkblue",
            "darkcyan",
            "darkgoldenrod",
            "darkgray",
            "darkgreen",
            "darkkhaki",
            "darkmagenta",
            "darkolivegreen",
            "darkorange",
            "darkorchid",
            "darkred",
            "darksalmon",
            "darkseagreen",
            "darkslateblue",
            "darkslategray",
            "darkturquoise",
            "darkviolet",
            "deeppink",
            "deepskyblue",
            "dimgray",
            "dodgerblue",
            "firebrick",
            "floralwhite",
            "forestgreen",
            "fuchsia",
            "gainsboro",
            "ghostwhite",
            "gold",
            "goldenrod",
            "gray",
            "green",
            "greenyellow",
            "honeydew",
            "hotpink",
            "indianred",
            "indigo",
            "ivory",
            "khaki",
            "lavender",
            "lavenderblush",
            "lawngreen",
            "lemonchiffon",
            "lightblue",
            "lightcoral",
            "lightcyan",
            "lightgoldenrodyellow",
            "lightgray",
            "lightgreen",
            "lightpink",
            "lightsalmon",
            "lightseagreen",
            "lightskyblue",
            "lightslategray",
            "lightsteelblue",
            "lightyellow",
            "lime",
            "limegreen",
            "linen",
            "magenta",
            "maroon",
            "mediumaquamarine",
            "mediumblue",
            "mediumorchid",
            "mediumpurple",
            "mediumseagreen",
            "mediumslateblue",
            "mediumspringgreen",
            "mediumturquoise",
            "mediumvioletred",
            "midnightblue",
            "mintcream",
            "mistyrose",
            "moccasin",
            "navajowhite",
            "navy",
            "oldlace",
            "olive",
            "olivedrab",
            "orange",
            "orangered",
            "orchid",
            "palegoldenrod",
            "palegreen",
            "paleturquoise",
            "palevioletred",
            "papayawhip",
            "peachpuff",
            "peru",
            "pink",
            "plum",
            "powderblue",
            "purple",
            "red",
            "rosybrown",
            "royalblue",
            "saddlebrown",
            "salmon",
            "sandybrown",
            "seagreen",
            "seashell",
            "sienna",
            "silver",
            "skyblue",
            "slateblue",
            "slategray",
            "snow",
            "springgreen",
            "steelblue",
            "tan",
            "teal",
            "thistle",
            "tomato",
            "turquoise",
            "violet",
            "wheat",
            "white",
            "whitesmoke",
            "yellow",
            "yellowgreen",
        ];

        NAMED_COLORS.contains(&color.to_lowercase().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rgb() {
        assert_eq!(ColorUtils::parse_rgb("rgb(255, 0, 0)"), Some((255, 0, 0)));
        assert_eq!(ColorUtils::parse_rgb("rgb(64 64 64)"), Some((64, 64, 64)));
        assert_eq!(ColorUtils::parse_rgb("rgb(100%, 0%, 0%)"), Some((255, 0, 0)));
        assert_eq!(ColorUtils::parse_rgb("rgb(-255, 100, 500)"), Some((0, 100, 255)));
        assert_eq!(ColorUtils::parse_rgb("invalid"), None);
    }

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(ColorUtils::rgb_to_hex(255, 0, 0), "#ff0000");
        assert_eq!(ColorUtils::rgb_to_hex(0, 255, 0), "#00ff00");
        assert_eq!(ColorUtils::rgb_to_hex(0, 0, 255), "#0000ff");
    }

    #[test]
    fn test_hex_to_short_hex() {
        assert_eq!(ColorUtils::hex_to_short_hex("#aabbcc"), Some("#abc".to_string()));
        assert_eq!(ColorUtils::hex_to_short_hex("#000000"), Some("#000".to_string()));
        assert_eq!(ColorUtils::hex_to_short_hex("#123456"), None);
    }

    #[test]
    fn test_is_valid_color() {
        assert!(ColorUtils::is_valid_color("#ff0000"));
        assert!(ColorUtils::is_valid_color("#f00"));
        assert!(ColorUtils::is_valid_color("rgb(255, 0, 0)"));
        assert!(ColorUtils::is_valid_color("red"));
        assert!(!ColorUtils::is_valid_color("invalid"));
    }

    #[test]
    fn test_is_named_color() {
        assert!(ColorUtils::is_named_color("red"));
        assert!(ColorUtils::is_named_color("blue"));
        assert!(ColorUtils::is_named_color("RED"));
        assert!(!ColorUtils::is_named_color("notacolor"));
    }
}
