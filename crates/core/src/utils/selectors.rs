// this_file: crates/core/src/utils/selectors.rs

//! CSS selector utilities for SVG processing
//!
//! Common functions for working with CSS selectors across multiple plugins.

use crate::ast::Element;

/// CSS selector utilities
pub struct SelectorUtils;

impl SelectorUtils {
    /// Check if an element matches a simple selector
    pub fn matches_selector(element: &Element, selector: &str) -> bool {
        let selector = selector.trim();

        // Handle different selector types
        if let Some(id) = selector.strip_prefix('#') {
            // ID selector
            element.attr("id") == Some(id)
        } else if let Some(class) = selector.strip_prefix('.') {
            // Class selector
            element.attr("class").is_some_and(|attr_class| attr_class.split_whitespace().any(|c| c == class))
        } else if selector.starts_with('[') && selector.ends_with(']') {
            // Attribute selector
            Self::matches_attribute_selector(element, &selector[1..selector.len() - 1])
        } else {
            // Element selector
            element.name.as_ref() == selector
        }
    }

    /// Check if an element matches an attribute selector
    fn matches_attribute_selector(element: &Element, selector: &str) -> bool {
        if let Some(eq_pos) = selector.find('=') {
            let attr_name = selector[..eq_pos].trim();
            let attr_value = selector[eq_pos + 1..].trim().trim_matches('"').trim_matches('\'');

            element.attr(attr_name) == Some(attr_value)
        } else {
            // Just attribute presence check
            element.attr(selector.trim()).is_some()
        }
    }

    /// Parse a comma-separated list of selectors
    pub fn parse_selector_list(selector_list: &str) -> Vec<String> {
        selector_list.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }

    /// Check if an element matches any of the given selectors
    pub fn matches_any_selector(element: &Element, selectors: &[String]) -> bool {
        selectors.iter().any(|s| Self::matches_selector(element, s))
    }

    /// Get the specificity of a selector (simplified version)
    pub fn get_specificity(selector: &str) -> u32 {
        let mut specificity = 0;

        // ID selectors: 100
        if selector.starts_with('#') {
            specificity += 100;
        }

        // Class selectors: 10
        if selector.starts_with('.') {
            specificity += 10;
        }

        // Attribute selectors: 10
        if selector.starts_with('[') && selector.ends_with(']') {
            specificity += 10;
        }

        // Element selectors: 1
        if !selector.starts_with('#') && !selector.starts_with('.') && !selector.starts_with('[') {
            specificity += 1;
        }

        specificity
    }

    /// Check if a selector is valid (basic validation)
    pub fn is_valid_selector(selector: &str) -> bool {
        let selector = selector.trim();

        if selector.is_empty() {
            return false;
        }

        // Check for invalid characters
        if selector.contains(' ') {
            // For now, we only support simple selectors, not descendant selectors
            return false;
        }

        // Basic validation for different selector types
        if selector.starts_with('#') {
            // ID selector
            selector.len() > 1 && selector.chars().skip(1).all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        } else if selector.starts_with('.') {
            // Class selector
            selector.len() > 1 && selector.chars().skip(1).all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        } else if selector.starts_with('[') && selector.ends_with(']') {
            // Attribute selector
            selector.len() > 2
        } else {
            // Element selector
            selector.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::ast::Element;

    fn create_test_element<'a>(name: &'a str, id: Option<&str>, class: Option<&str>) -> Element<'a> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);

        if let Some(id) = id {
            element.set_attr("id", id);
        }

        if let Some(class) = class {
            element.set_attr("class", class);
        }

        element
    }

    #[test]
    fn test_element_selector() {
        let element = create_test_element("rect", None, None);
        assert!(SelectorUtils::matches_selector(&element, "rect"));
        assert!(!SelectorUtils::matches_selector(&element, "circle"));
    }

    #[test]
    fn test_id_selector() {
        let element = create_test_element("rect", Some("my-rect"), None);
        assert!(SelectorUtils::matches_selector(&element, "#my-rect"));
        assert!(!SelectorUtils::matches_selector(&element, "#other-rect"));
    }

    #[test]
    fn test_class_selector() {
        let element = create_test_element("rect", None, Some("highlight important"));
        assert!(SelectorUtils::matches_selector(&element, ".highlight"));
        assert!(SelectorUtils::matches_selector(&element, ".important"));
        assert!(!SelectorUtils::matches_selector(&element, ".missing"));
    }

    #[test]
    fn test_attribute_selector() {
        let mut element = create_test_element("rect", None, None);
        element.set_attr("fill", "red");
        element.set_attr("stroke", "blue");

        assert!(SelectorUtils::matches_selector(&element, "[fill]"));
        assert!(SelectorUtils::matches_selector(&element, "[fill='red']"));
        assert!(SelectorUtils::matches_selector(&element, "[fill=\"red\"]"));
        assert!(!SelectorUtils::matches_selector(&element, "[fill='blue']"));
        assert!(!SelectorUtils::matches_selector(&element, "[width]"));
    }

    #[test]
    fn test_parse_selector_list() {
        let selectors = SelectorUtils::parse_selector_list("rect, circle, .highlight");
        assert_eq!(selectors, vec!["rect", "circle", ".highlight"]);
    }

    #[test]
    fn test_matches_any_selector() {
        let element = create_test_element("rect", Some("my-rect"), Some("highlight"));
        let selectors = vec!["circle".to_string(), "#my-rect".to_string()];
        assert!(SelectorUtils::matches_any_selector(&element, &selectors));

        let selectors = vec!["circle".to_string(), "#other-rect".to_string()];
        assert!(!SelectorUtils::matches_any_selector(&element, &selectors));
    }

    #[test]
    fn test_get_specificity() {
        assert_eq!(SelectorUtils::get_specificity("#my-id"), 100);
        assert_eq!(SelectorUtils::get_specificity(".my-class"), 10);
        assert_eq!(SelectorUtils::get_specificity("[attr='value']"), 10);
        assert_eq!(SelectorUtils::get_specificity("div"), 1);
    }

    #[test]
    fn test_is_valid_selector() {
        assert!(SelectorUtils::is_valid_selector("rect"));
        assert!(SelectorUtils::is_valid_selector("#my-id"));
        assert!(SelectorUtils::is_valid_selector(".my-class"));
        assert!(SelectorUtils::is_valid_selector("[attr='value']"));
        assert!(!SelectorUtils::is_valid_selector(""));
        assert!(!SelectorUtils::is_valid_selector("div p")); // No descendant selectors
    }
}
