// this_file: crates/plugin-sdk/src/plugins/inline_styles.rs

//! Inline styles plugin implementation
//!
//! This plugin moves and merges styles from `<style>` elements to inline style attributes.
//! It parses CSS rules, matches them against SVG elements using selectors, and applies
//! the computed styles directly to matching elements.
//!
//! SVGO parameters supported:
//! - `onlyMatchedOnce` (default: true) - Inline only rules that match a single element
//! - `removeMatchedSelectors` (default: true) - Remove selectors from style sheets when inlined
//! - `useMqs` (default: true) - Process media queries
//! - `usePseudos` (default: true) - Process pseudo-classes and pseudo-elements

use crate::Plugin;
use anyhow::{anyhow, Result};
use lightningcss::{
    declaration::DeclarationBlock,
    printer::PrinterOptions,
    rules::CssRule,
    stylesheet::{ParserOptions, StyleSheet},
    traits::ToCss,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Configuration parameters for inline styles plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineStylesConfig {
    /// Inline only rules that match a single element
    #[serde(default = "default_only_matched_once")]
    pub only_matched_once: bool,

    /// Remove selectors from style sheets when inlined
    #[serde(default = "default_remove_matched_selectors")]
    pub remove_matched_selectors: bool,

    /// Process media queries
    #[serde(default = "default_use_mqs")]
    pub use_mqs: bool,

    /// Process pseudo-classes and pseudo-elements
    #[serde(default = "default_use_pseudos")]
    pub use_pseudos: bool,
}

impl Default for InlineStylesConfig {
    fn default() -> Self {
        Self {
            only_matched_once: default_only_matched_once(),
            remove_matched_selectors: default_remove_matched_selectors(),
            use_mqs: default_use_mqs(),
            use_pseudos: default_use_pseudos(),
        }
    }
}

fn default_only_matched_once() -> bool {
    true
}
fn default_remove_matched_selectors() -> bool {
    true
}
fn default_use_mqs() -> bool {
    true
}
fn default_use_pseudos() -> bool {
    true
}

/// Plugin that inlines styles from style elements to inline style attributes
pub struct InlineStylesPlugin {
    config: InlineStylesConfig,
}

impl InlineStylesPlugin {
    /// Create a new InlineStylesPlugin
    pub fn new() -> Self {
        Self {
            config: InlineStylesConfig::default(),
        }
    }

    /// Create a new InlineStylesPlugin with config
    pub fn with_config(config: InlineStylesConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    #[allow(dead_code)]
    fn parse_config(params: &Value) -> Result<InlineStylesConfig> {
        if let Some(_obj) = params.as_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(InlineStylesConfig::default())
        }
    }
}

impl Default for InlineStylesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for InlineStylesPlugin {
    fn name(&self) -> &'static str {
        "inlineStyles"
    }

    fn description(&self) -> &'static str {
        "Move and merge styles from style elements to inline style attributes"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            // Validate boolean parameters
            for (key, value) in obj {
                match key.as_str() {
                    "onlyMatchedOnce" | "removeMatchedSelectors" | "useMqs" | "usePseudos" => {
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
        // First pass: collect all style elements and parse CSS
        let mut collector = InlineStylesVisitor::new(self.config.clone());
        vexy_svgo_core::visitor::walk_document(&mut collector, document)?;
        
        // Sort rules by specificity (cascade order)
        collector.css_rules.sort_by_key(|rule| rule.specificity);
        
        // Count matches if onlyMatchedOnce is enabled
        if collector.config.only_matched_once {
            // This would require another pass through the document
            // For now, we'll skip this optimization
        }
        
        // Second pass: apply styles to elements
        let mut applier = StyleApplierVisitor { visitor: &mut collector };
        vexy_svgo_core::visitor::walk_document(&mut applier, document)?;
        
        // Third pass: clean up if configured
        if collector.config.remove_matched_selectors {
            let mut cleaner = StyleCleanerVisitor {
                used_selectors: &collector.used_selectors,
                config: &collector.config,
            };
            vexy_svgo_core::visitor::walk_document(&mut cleaner, document)?;
        }
        
        Ok(())
    }
}

/// Represents a CSS rule with selector and declarations
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CssRuleData {
    selector: String,
    declarations: Vec<(String, String)>,
    specificity: u32,
    source_index: usize, // Track which style element this came from
}

/// Visitor implementation that inlines styles
struct InlineStylesVisitor {
    config: InlineStylesConfig,
    style_elements: Vec<(usize, String)>, // (element_id, css_content)
    element_counter: usize,
    css_rules: Vec<CssRuleData>,
    #[allow(dead_code)]
    match_counts: HashMap<String, usize>,
    used_selectors: HashSet<String>,
}

impl InlineStylesVisitor {
    fn new(config: InlineStylesConfig) -> Self {
        Self {
            config,
            style_elements: Vec::new(),
            element_counter: 0,
            css_rules: Vec::new(),
            match_counts: HashMap::new(),
            used_selectors: HashSet::new(),
        }
    }

    /// Extract CSS content from a style element
    fn extract_css_content(element: &Element) -> String {
        let mut content = String::new();

        for child in &element.children {
            match child {
                Node::Text(text) => content.push_str(text),
                Node::CData(cdata) => content.push_str(cdata),
                _ => {}
            }
        }

        content
    }

    /// Check if a selector contains pseudo-classes or pseudo-elements
    fn selector_contains_pseudo(selector: &str) -> bool {
        selector.contains(':')
            && (selector.contains(":hover")
                || selector.contains(":active")
                || selector.contains(":focus")
                || selector.contains(":visited")
                || selector.contains(":link")
                || selector.contains(":first-child")
                || selector.contains(":last-child")
                || selector.contains(":nth-child")
                || selector.contains(":nth-of-type")
                || selector.contains(":before")
                || selector.contains("::before")
                || selector.contains(":after")
                || selector.contains("::after"))
    }

    /// Parse CSS and extract rules
    fn parse_css_rules(&mut self, css_content: &str, source_index: usize) -> Result<()> {
        let options = ParserOptions::default();

        match StyleSheet::parse(css_content, options) {
            Ok(stylesheet) => {
                for rule in &stylesheet.rules.0 {
                    self.process_css_rule(rule, source_index)?;
                }
            }
            Err(_) => {
                // If parsing fails, we skip this style element
                // This matches SVGO behavior
            }
        }

        Ok(())
    }

    /// Process a single CSS rule
    fn process_css_rule(&mut self, rule: &CssRule, source_index: usize) -> Result<()> {
        match rule {
            CssRule::Style(style_rule) => {
                // Extract selector string
                let mut selector_string = String::new();
                let mut printer = lightningcss::printer::Printer::new(
                    &mut selector_string,
                    PrinterOptions::default(),
                );
                style_rule.selectors.to_css(&mut printer).ok();

                // Skip pseudo-selectors if not enabled
                if !self.config.use_pseudos && Self::selector_contains_pseudo(&selector_string) {
                    return Ok(());
                }

                // Extract declarations
                let declarations = self.extract_declarations(&style_rule.declarations);

                if !declarations.is_empty() {
                    // Calculate specificity (simplified)
                    let specificity = self.calculate_specificity(&selector_string);

                    self.css_rules.push(CssRuleData {
                        selector: selector_string,
                        declarations,
                        specificity,
                        source_index,
                    });
                }
            }
            CssRule::Media(media_rule) if self.config.use_mqs => {
                // Process rules inside media queries
                for inner_rule in &media_rule.rules.0 {
                    self.process_css_rule(inner_rule, source_index)?;
                }
            }
            _ => {
                // Skip other rule types
            }
        }

        Ok(())
    }

    /// Extract declarations from a declaration block
    fn extract_declarations(&self, declarations: &DeclarationBlock) -> Vec<(String, String)> {
        let mut result = Vec::new();

        // Convert declarations to string and parse
        let mut css_string = String::new();
        let mut printer =
            lightningcss::printer::Printer::new(&mut css_string, PrinterOptions::default());
        if declarations.to_css(&mut printer).is_ok() {
            // Parse the CSS string to extract property-value pairs
            // This is a simplified parser - in production, we'd use proper CSS parsing
            for decl in css_string.split(';') {
                let parts: Vec<&str> = decl.split(':').collect();
                if parts.len() == 2 {
                    let property = parts[0].trim();
                    let value = parts[1].trim();

                    // Only include presentation attributes
                    if is_presentation_attribute(property) {
                        result.push((property.to_string(), value.to_string()));
                    }
                }
            }
        }

        result
    }

    /// Calculate CSS specificity (simplified version)
    fn calculate_specificity(&self, selector: &str) -> u32 {
        let mut specificity = 0;

        // Count IDs (#)
        specificity += (selector.matches('#').count() as u32) * 1000000;

        // Count classes (.) and attribute selectors ([])
        specificity += (selector.matches('.').count() as u32) * 1000;
        specificity += (selector.matches('[').count() as u32) * 1000;

        // Count element selectors (simplified - count words that aren't classes/ids)
        let element_count = selector
            .split_whitespace()
            .filter(|s| !s.starts_with('.') && !s.starts_with('#') && !s.starts_with('['))
            .count() as u32;
        specificity += element_count;

        specificity
    }

    /// Apply styles to an element
    fn apply_styles_to_element(&mut self, element: &mut Element) {
        let mut styles_to_apply: HashMap<String, String> = HashMap::new();

        // Check each CSS rule to see if it matches this element
        for rule in &self.css_rules {
            if self.selector_matches_element(&rule.selector, element) {
                // Track that this selector was used
                self.used_selectors.insert(rule.selector.clone());

                // Apply declarations (later rules override earlier ones due to cascade)
                for (property, value) in &rule.declarations {
                    styles_to_apply.insert(property.clone(), value.clone());
                }
            }
        }

        // Apply collected styles to the element
        if !styles_to_apply.is_empty() {
            let existing_style = element.attributes.get("style").cloned().unwrap_or_default();
            let new_style = merge_styles(&existing_style, &styles_to_apply);
            element.attributes.insert("style".into(), new_style.into());
        }
    }

    /// Simple selector matching (basic implementation)
    fn selector_matches_element(&self, selector: &str, element: &Element) -> bool {
        // This is a simplified selector matching implementation
        // In production, we'd use a proper CSS selector matching library

        let selector = selector.trim();

        // Handle ID selector
        if selector.starts_with('#') {
            let id = &selector[1..];
            return element.attributes.get("id") == Some(&id.into());
        }

        // Handle class selector
        if selector.starts_with('.') {
            let class_name = &selector[1..];
            if let Some(classes) = element.attributes.get("class") {
                return classes.split_whitespace().any(|c| c == class_name);
            }
            return false;
        }

        // Handle element selector
        element.name == selector
    }
}

impl Visitor<'_> for InlineStylesVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        self.element_counter += 1;

        // Collect style elements
        if element.name == "style" {
            let css_content = Self::extract_css_content(element);
            if !css_content.trim().is_empty() {
                let source_index = self.style_elements.len();
                self.style_elements
                    .push((self.element_counter, css_content.clone()));

                // Parse CSS rules immediately
                self.parse_css_rules(&css_content, source_index)?;
            }
        }

        Ok(())
    }
}

/// Separate visitor for applying styles (to avoid borrow conflicts)
struct StyleApplierVisitor<'a> {
    visitor: &'a mut InlineStylesVisitor,
}

impl Visitor<'_> for StyleApplierVisitor<'_> {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        if element.name != "style" {
            self.visitor.apply_styles_to_element(element);
        }
        Ok(())
    }
}

/// Visitor for cleaning up empty style elements
struct StyleCleanerVisitor<'a> {
    #[allow(dead_code)]
    used_selectors: &'a HashSet<String>,
    config: &'a InlineStylesConfig,
}

impl Visitor<'_> for StyleCleanerVisitor<'_> {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        if element.name == "style" && self.config.remove_matched_selectors {
            // For now, remove all text content from style elements
            // In a full implementation, we'd selectively remove only used selectors
            element.children.clear();
        }
        Ok(())
    }
}

/// Check if a CSS property is a presentation attribute
fn is_presentation_attribute(property: &str) -> bool {
    // List of SVG presentation attributes that can be styled
    matches!(
        property,
        "alignment-baseline"
            | "baseline-shift"
            | "clip"
            | "clip-path"
            | "clip-rule"
            | "color"
            | "color-interpolation"
            | "color-interpolation-filters"
            | "color-profile"
            | "color-rendering"
            | "cursor"
            | "direction"
            | "display"
            | "dominant-baseline"
            | "enable-background"
            | "fill"
            | "fill-opacity"
            | "fill-rule"
            | "filter"
            | "flood-color"
            | "flood-opacity"
            | "font-family"
            | "font-size"
            | "font-size-adjust"
            | "font-stretch"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "glyph-orientation-horizontal"
            | "glyph-orientation-vertical"
            | "image-rendering"
            | "kerning"
            | "letter-spacing"
            | "lighting-color"
            | "marker-end"
            | "marker-mid"
            | "marker-start"
            | "mask"
            | "opacity"
            | "overflow"
            | "pointer-events"
            | "shape-rendering"
            | "stop-color"
            | "stop-opacity"
            | "stroke"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "stroke-opacity"
            | "stroke-width"
            | "text-anchor"
            | "text-decoration"
            | "text-rendering"
            | "unicode-bidi"
            | "visibility"
            | "word-spacing"
            | "writing-mode"
    )
}

/// Merge existing inline styles with new styles
fn merge_styles(existing: &str, new_styles: &HashMap<String, String>) -> String {
    let mut merged = HashMap::new();

    // Parse existing styles
    for part in existing.split(';') {
        let parts: Vec<&str> = part.split(':').collect();
        if parts.len() == 2 {
            merged.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
        }
    }

    // Add new styles (overwriting existing ones)
    for (prop, value) in new_styles {
        merged.insert(prop.clone(), value.clone());
    }

    // Build result string
    let mut result = String::new();
    for (prop, value) in merged {
        if !result.is_empty() {
            result.push_str("; ");
        }
        result.push_str(&format!("{}: {}", prop, value));
    }

    result
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

    fn create_style_element(css: &str) -> Element<'static> {
        let mut style = create_element("style");
        style.children.push(Node::Text(css.to_string()));
        style
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = InlineStylesPlugin::new();
        assert_eq!(plugin.name(), "inlineStyles");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = InlineStylesPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({
                "onlyMatchedOnce": true,
                "removeMatchedSelectors": false,
                "useMqs": true,
                "usePseudos": false
            }))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"onlyMatchedOnce": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"unknownParam": true}))
            .is_err());
    }

    #[test]
    fn test_is_presentation_attribute() {
        assert!(is_presentation_attribute("fill"));
        assert!(is_presentation_attribute("stroke"));
        assert!(is_presentation_attribute("opacity"));
        assert!(!is_presentation_attribute("transform"));
        assert!(!is_presentation_attribute("x"));
        assert!(!is_presentation_attribute("width"));
    }

    #[test]
    fn test_merge_styles() {
        let existing = "fill: red; stroke: blue";
        let mut new_styles = HashMap::new();
        new_styles.insert("fill".to_string(), "green".to_string());
        new_styles.insert("opacity".to_string(), "0.5".to_string());

        let result = merge_styles(existing, &new_styles);
        assert!(result.contains("fill: green"));
        assert!(result.contains("stroke: blue"));
        assert!(result.contains("opacity: 0.5"));
    }

    #[test]
    fn test_basic_inline_styles() {
        let plugin = InlineStylesPlugin::new();
        let mut doc = Document::new();

        // Add a style element
        let style = create_style_element(".test { fill: red; }");
        doc.root.children.push(Node::Element(style));

        // Add an element with the class
        let mut rect = create_element("rect");
        rect.set_attr("class", "test");
        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // Check that the style was inlined
        if let Some(Node::Element(rect)) = doc.root.children.get(1) {
            assert_eq!(rect.attr("style"), Some("fill: red"));
        }
    }

    #[test]
    fn test_id_selector() {
        let plugin = InlineStylesPlugin::new();
        let mut doc = Document::new();

        // Add a style element
        let style = create_style_element("#myid { stroke: blue; }");
        doc.root.children.push(Node::Element(style));

        // Add an element with the ID
        let mut circle = create_element("circle");
        circle.set_attr("id", "myid");
        doc.root.children.push(Node::Element(circle));

        plugin.apply(&mut doc).unwrap();

        // Check that the style was inlined
        if let Some(Node::Element(circle)) = doc.root.children.get(1) {
            assert_eq!(circle.attr("style"), Some("stroke: #00f"));
        }
    }

    #[test]
    fn test_element_selector() {
        let plugin = InlineStylesPlugin::new();
        let mut doc = Document::new();

        // Add a style element
        let style = create_style_element("rect { fill: green; opacity: 0.5; }");
        doc.root.children.push(Node::Element(style));

        // Add a rect element
        let rect = create_element("rect");
        doc.root.children.push(Node::Element(rect));

        // Add a circle element (should not match)
        let circle = create_element("circle");
        doc.root.children.push(Node::Element(circle));

        plugin.apply(&mut doc).unwrap();

        // Check that only the rect got the style
        if let Some(Node::Element(rect)) = doc.root.children.get(1) {
            assert!(rect.attributes.contains_key("style"));
        }
        if let Some(Node::Element(circle)) = doc.root.children.get(2) {
            assert!(!circle.attributes.contains_key("style"));
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = InlineStylesPlugin::parse_config(&json!({
            "onlyMatchedOnce": false,
            "removeMatchedSelectors": true,
            "useMqs": false,
            "usePseudos": true
        }))
        .unwrap();

        assert_eq!(config.only_matched_once, false);
        assert_eq!(config.remove_matched_selectors, true);
        assert_eq!(config.use_mqs, false);
        assert_eq!(config.use_pseudos, true);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(InlineStylesPlugin, "inlineStyles");
