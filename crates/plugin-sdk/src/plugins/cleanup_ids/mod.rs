// this_file: crates/plugin-sdk/src/plugins/cleanup_ids.rs

//! Cleanup IDs plugin implementation
//!
//! This plugin removes unused IDs and minifies used IDs to save space.
//! It's careful not to break references and respects various preservation options.
//!
//! SVGO parameters supported:
//! - `remove` (default: true) - Remove unused IDs
//! - `minify` (default: true) - Minify used IDs
//! - `preserve` (default: []) - Array of IDs to preserve
//! - `preservePrefixes` (default: []) - Array of ID prefixes to preserve
//! - `force` (default: false) - Process even if scripts/styles are present

use crate::Plugin;
use anyhow::{anyhow, Result};
use vexy_svgo_core::error::VexyError;

pub mod collector;
pub mod renamer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;

use self::renamer::{find_references, update_reference_value, IdGenerator, REFERENCES_PROPS};

/// Configuration parameters for cleanup IDs plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupIdsConfig {
    /// Remove unused IDs
    #[serde(default = "default_remove")]
    pub remove: bool,

    /// Minify used IDs
    #[serde(default = "default_minify")]
    pub minify: bool,

    /// Array of IDs to preserve
    #[serde(default)]
    pub preserve: Vec<String>,

    /// Array of ID prefixes to preserve
    #[serde(default)]
    pub preserve_prefixes: Vec<String>,

    /// Process even if scripts/styles are present
    #[serde(default)]
    pub force: bool,
}

impl Default for CleanupIdsConfig {
    fn default() -> Self {
        Self {
            remove: default_remove(),
            minify: default_minify(),
            preserve: Vec::new(),
            preserve_prefixes: Vec::new(),
            force: false,
        }
    }
}

fn default_remove() -> bool {
    true
}
fn default_minify() -> bool {
    true
}

/// Plugin that removes unused IDs and minifies used IDs
pub struct CleanupIdsPlugin {
    config: CleanupIdsConfig,
}

impl CleanupIdsPlugin {
    /// Create a new CleanupIdsPlugin
    pub fn new() -> Self {
        Self {
            config: CleanupIdsConfig::default(),
        }
    }

    /// Create a new CleanupIdsPlugin with config
    pub fn with_config(config: CleanupIdsConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    #[allow(dead_code)]
    fn parse_config(params: &Value) -> Result<CleanupIdsConfig> {
        if let Some(_obj) = params.as_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(CleanupIdsConfig::default())
        }
    }
}

impl Default for CleanupIdsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for CleanupIdsPlugin {
    fn name(&self) -> &'static str {
        "cleanupIds"
    }

    fn description(&self) -> &'static str {
        "Remove unused IDs and minify used IDs"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            // Validate parameters
            for (key, value) in obj {
                match key.as_str() {
                    "remove" | "minify" | "force" => {
                        if !value.is_boolean() {
                            return Err(anyhow!("{} must be a boolean", key));
                        }
                    }
                    "preserve" | "preservePrefixes" => {
                        if !value.is_array() {
                            return Err(anyhow!("{} must be an array", key));
                        }
                        if let Some(arr) = value.as_array() {
                            for item in arr {
                                if !item.is_string() {
                                    return Err(anyhow!("{} must contain only strings", key));
                                }
                            }
                        }
                    }
                    _ => return Err(anyhow!("Unknown parameter: {}", key)),
                }
            }
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // First pass: collect all IDs and references
        let mut collector = CleanupIdsVisitor::new(self.config.clone());
        vexy_svgo_core::visitor::walk_document(&mut collector, document)?;
        
        // Check if we should skip processing
        if !collector.config.force && (collector.has_scripts || collector.has_styles) {
            return Ok(());
        }

        // Determine which IDs are used
        let used_ids: HashSet<String> = collector.references_by_id.keys().cloned().collect();

        // Generate ID mappings for minification
        let mut id_mappings = HashMap::new();
        if collector.config.minify {
            let mut id_generator = IdGenerator::new();
            let mut new_id_set = HashSet::new();

            // Process referenced IDs
            for id in &used_ids {
                if !collector.is_id_preserved(id) && collector.node_by_id.contains_key(id) {
                    // Generate unique minified ID
                    let mut new_id = id_generator.next();
                    while collector.is_id_preserved(&new_id)
                        || collector.node_by_id.contains_key(&new_id)
                        || new_id_set.contains(&new_id)
                    {
                        new_id = id_generator.next();
                    }
                    new_id_set.insert(new_id.clone());
                    id_mappings.insert(id.clone(), new_id);
                }
            }
        }

        // Second pass: apply changes
        let mut applier = IdApplierVisitor {
            config: &collector.config,
            node_by_id: &collector.node_by_id,
            used_ids: &used_ids,
            id_mappings: &id_mappings,
            current_path: Vec::new(),
        };
        vexy_svgo_core::visitor::walk_document(&mut applier, document)?;

        Ok(())
    }
}

/// Visitor implementation that cleans up IDs
struct CleanupIdsVisitor {
    config: CleanupIdsConfig,
    has_scripts: bool,
    has_styles: bool,
    node_by_id: HashMap<String, ElementInfo>,
    references_by_id: HashMap<String, Vec<Reference>>,
    current_path: Vec<usize>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ElementInfo {
    path: Vec<usize>,
    element_name: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Reference {
    path: Vec<usize>,
    attr_name: String,
}

impl CleanupIdsVisitor {
    fn new(config: CleanupIdsConfig) -> Self {
        Self {
            config,
            has_scripts: false,
            has_styles: false,
            node_by_id: HashMap::new(),
            references_by_id: HashMap::new(),
            current_path: Vec::new(),
        }
    }

    fn check_for_scripts(&mut self, element: &Element) {
        // Check for script elements
        if element.name == "script"
            && element
                .children
                .iter()
                .any(|n| matches!(n, Node::Text(_) | Node::CData(_)))
        {
            self.has_scripts = true;
            return;
        }

        // Check for javascript: links
        if element.name == "a" {
            if let Some(href) = element.attributes.get("href") {
                if href.trim_start().starts_with("javascript:") {
                    self.has_scripts = true;
                    return;
                }
            }
        }

        // Check for event attributes
        for (attr_name, _) in &element.attributes {
            if attr_name.starts_with("on") {
                self.has_scripts = true;
                return;
            }
        }
    }

    fn check_for_styles(&mut self, element: &Element) {
        if element.name == "style"
            && element
                .children
                .iter()
                .any(|n| matches!(n, Node::Text(_) | Node::CData(_)))
        {
            self.has_styles = true;
        }
    }

    fn is_id_preserved(&self, id: &str) -> bool {
        self.config.preserve.contains(&id.to_string())
            || self
                .config
                .preserve_prefixes
                .iter()
                .any(|prefix| id.starts_with(prefix))
    }

    fn collect_references(&mut self, element: &Element) {
        for (attr_name, attr_value) in &element.attributes {
            let ids = find_references(attr_name, attr_value);
            for id in ids {
                self.references_by_id
                    .entry(id)
                    .or_insert_with(Vec::new)
                    .push(Reference {
                        path: self.current_path.clone(),
                        attr_name: attr_name.to_string(),
                    });
            }
        }
    }
}

impl Visitor<'_> for CleanupIdsVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Update path
        let _child_index = self.current_path.len();
        self.current_path.push(0);

        // Check for scripts and styles
        if !self.has_scripts {
            self.check_for_scripts(element);
        }
        if !self.has_styles {
            self.check_for_styles(element);
        }

        // Collect ID if present
        if let Some(id) = element.attributes.get("id") {
            self.node_by_id.insert(
                id.to_string(),
                ElementInfo {
                    path: self.current_path.clone(),
                    element_name: element.name.to_string(),
                },
            );
        }

        // Collect references
        self.collect_references(element);

        Ok(())
    }

    fn visit_element_exit(&mut self, _element: &mut Element<'_>) -> Result<(), VexyError> {
        // Update path
        self.current_path.pop();
        if !self.current_path.is_empty() {
            let last_idx = self.current_path.len() - 1;
            self.current_path[last_idx] += 1;
        }
        Ok(())
    }
}

/// Separate visitor for applying ID changes
struct IdApplierVisitor<'a> {
    config: &'a CleanupIdsConfig,
    #[allow(dead_code)]
    node_by_id: &'a HashMap<String, ElementInfo>,
    used_ids: &'a HashSet<String>,
    id_mappings: &'a HashMap<String, String>,
    current_path: Vec<usize>,
}

impl<'a> IdApplierVisitor<'a> {
    fn is_id_preserved(&self, id: &str) -> bool {
        self.config.preserve.contains(&id.to_string())
            || self
                .config
                .preserve_prefixes
                .iter()
                .any(|prefix| id.starts_with(prefix))
    }
}

impl Visitor<'_> for IdApplierVisitor<'_> {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Update path
        let _child_index = self.current_path.len();
        self.current_path.push(0);

        // Process ID attribute
        if let Some(id) = element.attributes.get("id").cloned() {
            if let Some(new_id) = self.id_mappings.get(id.as_ref()) {
                // Update to minified ID
                element.attributes.insert("id".into(), new_id.clone().into());
            } else if self.config.remove
                && !self.used_ids.contains(id.as_ref())
                && !self.is_id_preserved(&id)
            {
                // Remove unused ID
                element.attributes.shift_remove("id");
            }
        }

        // Update references in attributes
        let mut updates = Vec::new();
        for (attr_name, attr_value) in &element.attributes {
            if REFERENCES_PROPS.contains(&attr_name.as_ref())
                || attr_name == "style"
                || attr_name == "href"
                || attr_name == "xlink:href"
                || attr_name == "begin"
            {
                let new_value = update_reference_value(attr_value, self.id_mappings);
                if new_value != *attr_value {
                    updates.push((attr_name.clone(), new_value));
                }
            }
        }

        // Apply updates
        for (attr_name, new_value) in updates {
            element.attributes.insert(attr_name, new_value.into());
        }

        Ok(())
    }

    fn visit_element_exit(&mut self, _element: &mut Element<'_>) -> Result<(), VexyError> {
        // Update path
        self.current_path.pop();
        if !self.current_path.is_empty() {
            let last_idx = self.current_path.len() - 1;
            self.current_path[last_idx] += 1;
        }
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
        let plugin = CleanupIdsPlugin::new();
        assert_eq!(plugin.name(), "cleanupIds");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = CleanupIdsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({
                "remove": true,
                "minify": false,
                "preserve": ["id1", "id2"],
                "preservePrefixes": ["prefix_"],
                "force": true
            }))
            .is_ok());

        // Invalid parameters
        assert!(plugin
            .validate_params(&json!({"remove": "invalid"}))
            .is_err());
        assert!(plugin
            .validate_params(&json!({"preserve": "not_array"}))
            .is_err());
        assert!(plugin.validate_params(&json!({"preserve": [123]})).is_err());
        assert!(plugin
            .validate_params(&json!({"unknownParam": true}))
            .is_err());
    }

    #[test]
    fn test_find_references() {
        // Test href references
        assert_eq!(find_references("href", "#myid"), vec!["myid"]);
        assert_eq!(find_references("xlink:href", "#test"), vec!["test"]);

        // Test URL references
        assert_eq!(find_references("fill", "url(#gradient)"), vec!["gradient"]);
        assert_eq!(find_references("fill", "url('#pattern')"), vec!["pattern"]);
        assert_eq!(find_references("fill", "url(\"#mask\")"), vec!["mask"]);

        // Test style attribute
        assert_eq!(
            find_references("style", "fill: url(#grad1); stroke: url(#grad2)"),
            vec!["grad1", "grad2"]
        );

        // Test begin attribute
        assert_eq!(find_references("begin", "elem1.end"), vec!["elem1"]);
    }

    #[test]
    fn test_id_generator() {
        let mut gen = IdGenerator::new();
        assert_eq!(gen.next(), "a");
        assert_eq!(gen.next(), "b");
        assert_eq!(gen.next(), "c");

        // Test rollover - generate IDs until we reach 'Z', then test wraparound
        // Skip to near the end (we already have 'a', 'b', 'c', so skip 47 more to get to 'Y')
        for _ in 0..47 {
            gen.next();
        }
        assert_eq!(gen.next(), "Y");
        assert_eq!(gen.next(), "Z");
        assert_eq!(gen.next(), "aa");
        assert_eq!(gen.next(), "ab");
    }

    #[test]
    fn test_remove_unused_ids() {
        let plugin = CleanupIdsPlugin::new();
        let mut doc = Document::new();

        // Add elements with IDs
        let mut rect1 = create_element("rect");
        rect1.set_attr("id", "used");

        let mut rect2 = create_element("rect");
        rect2.set_attr("id", "unused");

        let mut use_elem = create_element("use");
        use_elem.set_attr("href", "#used");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(use_elem));

        plugin.apply(&mut doc).unwrap();

        // Check that used ID is kept and unused is removed
        if let Some(Node::Element(rect1)) = doc.root.children.get(0) {
            assert!(rect1.attributes.contains_key("id"));
        }
        if let Some(Node::Element(rect2)) = doc.root.children.get(1) {
            assert!(!rect2.attributes.contains_key("id"));
        }
    }

    #[test]
    fn test_minify_ids() {
        let config = CleanupIdsConfig {
            remove: false,
            minify: true,
            ..Default::default()
        };
        let plugin = CleanupIdsPlugin::with_config(config);
        let mut doc = Document::new();

        // Add element with long ID
        let mut rect = create_element("rect");
        rect.set_attr("id", "very_long_identifier");

        let mut use_elem = create_element("use");
        use_elem.set_attr("href", "#very_long_identifier");

        doc.root.children.push(Node::Element(rect));
        doc.root.children.push(Node::Element(use_elem));

        plugin.apply(&mut doc).unwrap();

        // Check that ID was minified
        if let Some(Node::Element(rect)) = doc.root.children.get(0) {
            let id = rect.attr("id").unwrap();
            assert!(id.len() < "very_long_identifier".len());
            assert_eq!(id, "a"); // First generated ID should be "a"
        }

        // Check that reference was updated
        if let Some(Node::Element(use_elem)) = doc.root.children.get(1) {
            assert_eq!(use_elem.attr("href"), Some("#a"));
        }
    }

    #[test]
    fn test_preserve_ids() {
        let config = CleanupIdsConfig {
            remove: true,
            minify: true,
            preserve: vec!["keep_this".to_string()],
            preserve_prefixes: vec!["pres_".to_string()],
            ..Default::default()
        };
        let plugin = CleanupIdsPlugin::with_config(config);
        let mut doc = Document::new();

        // Add elements with IDs to preserve
        let mut rect1 = create_element("rect");
        rect1.set_attr("id", "keep_this");

        let mut rect2 = create_element("rect");
        rect2.set_attr("id", "pres_something");

        let mut rect3 = create_element("rect");
        rect3.set_attr("id", "remove_this");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(rect3));

        plugin.apply(&mut doc).unwrap();

        // Check preserved IDs
        if let Some(Node::Element(rect1)) = doc.root.children.get(0) {
            assert_eq!(rect1.attr("id"), Some("keep_this"));
        }
        if let Some(Node::Element(rect2)) = doc.root.children.get(1) {
            assert_eq!(rect2.attr("id"), Some("pres_something"));
        }
        if let Some(Node::Element(rect3)) = doc.root.children.get(2) {
            assert!(!rect3.attributes.contains_key("id"));
        }
    }

    #[test]
    fn test_skip_with_scripts() {
        let plugin = CleanupIdsPlugin::new();
        let mut doc = Document::new();

        // Add script element
        let mut script = create_element("script");
        script
            .children
            .push(Node::Text("console.log('test');".to_string()));

        // Add element with ID
        let mut rect = create_element("rect");
        rect.set_attr("id", "should_not_be_removed");

        doc.root.children.push(Node::Element(script));
        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // ID should still be there because of script
        if let Some(Node::Element(rect)) = doc.root.children.get(1) {
            assert_eq!(rect.attr("id"), Some("should_not_be_removed"));
        }
    }

    #[test]
    fn test_force_with_scripts() {
        let config = CleanupIdsConfig {
            remove: true,
            force: true,
            ..Default::default()
        };
        let plugin = CleanupIdsPlugin::with_config(config);
        let mut doc = Document::new();

        // Add script element
        let mut script = create_element("script");
        script
            .children
            .push(Node::Text("console.log('test');".to_string()));

        // Add element with unused ID
        let mut rect = create_element("rect");
        rect.set_attr("id", "should_be_removed");

        doc.root.children.push(Node::Element(script));
        doc.root.children.push(Node::Element(rect));

        plugin.apply(&mut doc).unwrap();

        // ID should be removed because force is true
        if let Some(Node::Element(rect)) = doc.root.children.get(1) {
            assert!(!rect.attributes.contains_key("id"));
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = CleanupIdsPlugin::parse_config(&json!({
            "remove": false,
            "minify": true,
            "preserve": ["id1", "id2"],
            "preservePrefixes": ["prefix_"],
            "force": false
        }))
        .unwrap();

        assert_eq!(config.remove, false);
        assert_eq!(config.minify, true);
        assert_eq!(config.preserve, vec!["id1", "id2"]);
        assert_eq!(config.preserve_prefixes, vec!["prefix_"]);
        assert_eq!(config.force, false);
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(CleanupIdsPlugin, "cleanupIds");
