// this_file: crates/core/src/utils/numbers.rs

//! Number formatting utilities for SVG processing
//!
//! Common functions for working with numeric values across multiple plugins.

/// Number formatting utilities
pub struct NumberUtils;

impl NumberUtils {
    /// Format a number with specified precision
    pub fn format_number(value: f64, precision: usize) -> String {
        if value.fract() == 0.0 && value.abs() < 1e10 {
            // Integer value
            format!("{}", value as i64)
        } else {
            // Float value
            let formatted = format!("{value:.precision$}");
            // Remove trailing zeros after decimal point
            if formatted.contains('.') {
                formatted.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                formatted
            }
        }
    }

    /// Format a number with minimal precision
    pub fn format_number_minimal(value: f64) -> String {
        Self::format_number(value, 3)
    }

    /// Parse a number string to f64
    pub fn parse_number(value: &str) -> Option<f64> {
        value.trim().parse().ok()
    }

    /// Check if a value is close to zero
    pub fn is_zero(value: f64, tolerance: f64) -> bool {
        value.abs() < tolerance
    }

    /// Check if a value is close to another value
    pub fn is_close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() < tolerance
    }

    /// Round a value to a specified number of decimal places
    pub fn round_to_precision(value: f64, precision: usize) -> f64 {
        let factor = 10_f64.powi(precision as i32);
        (value * factor).round() / factor
    }

    /// Check if a number can be represented as an integer
    pub fn is_integer(value: f64) -> bool {
        value.fract() == 0.0 && value.abs() < 1e10
    }

    /// Normalize angle to 0-360 range
    pub fn normalize_angle(angle: f64) -> f64 {
        let mut result = angle % 360.0;
        if result < 0.0 {
            result += 360.0;
        }
        result
    }

    /// Convert degrees to radians
    pub fn deg_to_rad(degrees: f64) -> f64 {
        degrees * std::f64::consts::PI / 180.0
    }

    /// Convert radians to degrees
    pub fn rad_to_deg(radians: f64) -> f64 {
        radians * 180.0 / std::f64::consts::PI
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(NumberUtils::format_number(1.0, 3), "1");
        assert_eq!(NumberUtils::format_number(1.5, 3), "1.5");
        assert_eq!(NumberUtils::format_number(1.500, 3), "1.5");
        assert_eq!(NumberUtils::format_number(1.234, 2), "1.23");
    }

    #[test]
    fn test_parse_number() {
        assert_eq!(NumberUtils::parse_number("1.5"), Some(1.5));
        assert_eq!(NumberUtils::parse_number("  2.0  "), Some(2.0));
        assert_eq!(NumberUtils::parse_number("invalid"), None);
    }

    #[test]
    fn test_is_zero() {
        assert!(NumberUtils::is_zero(0.0, 1e-10));
        assert!(NumberUtils::is_zero(1e-15, 1e-10));
        assert!(!NumberUtils::is_zero(1e-5, 1e-10));
    }

    #[test]
    fn test_is_close() {
        assert!(NumberUtils::is_close(1.0, 1.0001, 0.001));
        assert!(!NumberUtils::is_close(1.0, 1.1, 0.001));
    }

    #[test]
    fn test_round_to_precision() {
        assert_eq!(NumberUtils::round_to_precision(1.2345, 2), 1.23);
        assert_eq!(NumberUtils::round_to_precision(1.2355, 2), 1.24);
    }

    #[test]
    fn test_is_integer() {
        assert!(NumberUtils::is_integer(1.0));
        assert!(NumberUtils::is_integer(42.0));
        assert!(!NumberUtils::is_integer(1.5));
    }

    #[test]
    fn test_normalize_angle() {
        assert_eq!(NumberUtils::normalize_angle(0.0), 0.0);
        assert_eq!(NumberUtils::normalize_angle(360.0), 0.0);
        assert_eq!(NumberUtils::normalize_angle(450.0), 90.0);
        assert_eq!(NumberUtils::normalize_angle(-90.0), 270.0);
    }

    #[test]
    fn test_deg_to_rad() {
        assert!((NumberUtils::deg_to_rad(180.0) - std::f64::consts::PI).abs() < 1e-10);
        assert!((NumberUtils::deg_to_rad(90.0) - std::f64::consts::PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_rad_to_deg() {
        assert!((NumberUtils::rad_to_deg(std::f64::consts::PI) - 180.0).abs() < 1e-10);
        assert!((NumberUtils::rad_to_deg(std::f64::consts::PI / 2.0) - 90.0).abs() < 1e-10);
    }
}
