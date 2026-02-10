// this_file: crates/plugin-sdk/src/plugins/prefix_ids.rs

//! Prefix IDs and class names with a given string or auto-generated prefix
//!
//! This plugin adds a prefix to all IDs and optionally class names in the SVG document.
//! It also updates all references to these IDs in attributes like href, fill, etc.
//!
//! Reference: SVGO's prefixIds plugin

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::collections::REFERENCES_PROPS;

use crate::Plugin;

/// Configuration for the prefix IDs plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrefixIdsConfig {
    /// The prefix to use. Can be a string or auto-generated from file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix:             Option<String>,
    /// Delimiter between prefix and original ID
    #[serde(default = "default_delim")]
    pub delim:              String,
    /// Whether to prefix IDs
    #[serde(default = "default_true")]
    pub prefix_ids:         bool,
    /// Whether to prefix class names
    #[serde(default = "default_true")]
    pub prefix_class_names: bool,
}

fn default_delim() -> String {
    "__".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for PrefixIdsConfig {
    fn default() -> Self {
        Self {
            prefix:             None,
            delim:              default_delim(),
            prefix_ids:         default_true(),
            prefix_class_names: default_true(),
        }
    }
}

/// Prefix IDs plugin
pub struct PrefixIdsPlugin {
    config: PrefixIdsConfig,
}

impl PrefixIdsPlugin {
    pub fn new() -> Self {
        Self {
            config: PrefixIdsConfig::default(),
        }
    }

    pub fn with_config(config: PrefixIdsConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<PrefixIdsConfig> {
        if params.is_null() {
            Ok(PrefixIdsConfig::default())
        } else {
            serde_json::from_value(params.clone()).map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn get_basename(path: &str) -> String {
        // Extract everything after latest slash or backslash
        if let Some(captures) = Regex::new(r"[/\\]?([^/\\]+)$").unwrap().captures(path) {
            if let Some(matched) = captures.get(1) {
                return matched.as_str().to_string();
            }
        }
        String::new()
    }

    fn escape_identifier_name(s: &str) -> String {
        s.replace(['.', ' '], "_")
    }

    fn generate_prefix(&self, document: &Document) -> String {
        if let Some(prefix) = &self.config.prefix {
            return format!("{}{}", prefix, self.config.delim);
        }

        if let Some(path) = &document.metadata.path {
            let basename = Self::get_basename(path);
            if !basename.is_empty() {
                return format!("{}{}", Self::escape_identifier_name(&basename), self.config.delim);
            }
        }

        format!("prefix{}", self.config.delim)
    }

    fn prefix_id(&self, prefix: &str, id: &str) -> String {
        if id.starts_with(prefix) {
            id.to_string()
        } else {
            format!("{}{}", prefix, id)
        }
    }

    fn prefix_reference(&self, prefix: &str, reference: &str) -> Option<String> {
        reference.strip_prefix('#').map(|id| format!("#{}", self.prefix_id(prefix, id)))
    }

    fn process_element(&self, element: &mut Element, prefix: &str) {
        // Prefix ID attribute
        if self.config.prefix_ids {
            if let Some(id) = element.attr("id") {
                if !id.is_empty() {
                    element.set_attr("id", self.prefix_id(prefix, &id));
                }
            }
        }

        // Prefix class attribute
        if self.config.prefix_class_names {
            if let Some(class) = element.attr("class") {
                if !class.is_empty() {
                    let classes: Vec<String> =
                        class.split_whitespace().map(|name| self.prefix_id(prefix, name)).collect();
                    element.set_attr("class", classes.join(" "));
                }
            }
        }

        // Prefix href and xlink:href attributes
        for attr_name in ["href", "xlink:href"] {
            if let Some(href) = element.attr(attr_name) {
                if !href.is_empty() {
                    if let Some(prefixed) = self.prefix_reference(prefix, &href) {
                        element.set_attr(attr_name, prefixed);
                    }
                }
            }
        }

        // Prefix URL references in specific attributes
        for attr_name in REFERENCES_PROPS.iter() {
            if let Some(attr_value) = element.attr(attr_name) {
                if !attr_value.is_empty() {
                    let processed = self.process_url_references(&attr_value, prefix);
                    element.set_attr(*attr_name, processed);
                }
            }
        }

        // Prefix begin/end attributes (for animation)
        for attr_name in ["begin", "end"] {
            if let Some(attr_value) = element.attr(attr_name) {
                if !attr_value.is_empty() {
                    let processed = self.process_animation_references(&attr_value, prefix);
                    element.set_attr(attr_name, processed);
                }
            }
        }

        // Process style elements
        if element.name == "style" {
            let mut new_children = Vec::new();
            for child in &element.children {
                match child {
                    Node::Text(text) => {
                        let processed = self.process_style_content(text, prefix);
                        new_children.push(Node::Text(processed.into()));
                    },
                    _ => new_children.push(child.clone()),
                }
            }
            element.children = new_children;
        }

        // Process children recursively
        let mut i = 0;
        while i < element.children.len() {
            if let Node::Element(child) = &mut element.children[i] {
                self.process_element(child, prefix);
            }
            i += 1;
        }
    }

    fn process_url_references(&self, value: &str, prefix: &str) -> String {
        // Match url() with double quotes, single quotes, or no quotes
        let url_double_quote = Regex::new(r#"\burl\("(#[^"]+)"\)"#).unwrap();
        let url_single_quote = Regex::new(r#"\burl\('(#[^']+)'\)"#).unwrap();
        let url_no_quote = Regex::new(r#"\burl\((#[^)]+)\)"#).unwrap();

        let mut result = value.to_string();

        // Process double-quoted URLs
        result = url_double_quote
            .replace_all(&result, |caps: &regex::Captures| {
                let url = caps.get(1).unwrap().as_str();
                if let Some(prefixed) = self.prefix_reference(prefix, url) {
                    format!(r#"url("{}")"#, prefixed)
                } else {
                    caps.get(0).unwrap().as_str().to_string()
                }
            })
            .to_string();

        // Process single-quoted URLs
        result = url_single_quote
            .replace_all(&result, |caps: &regex::Captures| {
                let url = caps.get(1).unwrap().as_str();
                if let Some(prefixed) = self.prefix_reference(prefix, url) {
                    format!(r#"url('{}')"#, prefixed)
                } else {
                    caps.get(0).unwrap().as_str().to_string()
                }
            })
            .to_string();

        // Process unquoted URLs
        result = url_no_quote
            .replace_all(&result, |caps: &regex::Captures| {
                let url = caps.get(1).unwrap().as_str();
                if let Some(prefixed) = self.prefix_reference(prefix, url) {
                    format!("url({})", prefixed)
                } else {
                    caps.get(0).unwrap().as_str().to_string()
                }
            })
            .to_string();

        result
    }

    fn process_animation_references(&self, value: &str, prefix: &str) -> String {
        let parts: Vec<String> = value
            .split(';')
            .map(|part| {
                let trimmed = part.trim();
                if trimmed.ends_with(".end") || trimmed.ends_with(".start") {
                    let mut split_parts = trimmed.split('.');
                    if let Some(id) = split_parts.next() {
                        let postfix = split_parts.collect::<Vec<_>>().join(".");
                        format!("{}.{}", self.prefix_id(prefix, id), postfix)
                    } else {
                        trimmed.to_string()
                    }
                } else {
                    trimmed.to_string()
                }
            })
            .collect();

        parts.join("; ")
    }

    fn process_style_content(&self, content: &str, prefix: &str) -> String {
        let mut result = content.to_string();

        // Simple patterns for ID and class selectors (without full CSS parsing)
        if self.config.prefix_ids {
            // Match #id selectors
            let id_regex = Regex::new(r"#([a-zA-Z][\w-]*)").unwrap();
            result = id_regex
                .replace_all(&result, |caps: &regex::Captures| {
                    let id = caps.get(1).unwrap().as_str();
                    format!("#{}", self.prefix_id(prefix, id))
                })
                .to_string();
        }

        if self.config.prefix_class_names {
            // Match .class selectors
            let class_regex = Regex::new(r"\.([a-zA-Z][\w-]*)").unwrap();
            result = class_regex
                .replace_all(&result, |caps: &regex::Captures| {
                    let class = caps.get(1).unwrap().as_str();
                    format!(".{}", self.prefix_id(prefix, class))
                })
                .to_string();
        }

        // Also handle url() references in CSS
        result = self.process_url_references(&result, prefix);

        result
    }
}

impl Default for PrefixIdsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PrefixIdsPlugin {
    fn name(&self) -> &'static str {
        "prefixIds"
    }

    fn description(&self) -> &'static str {
        "prefix IDs"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply<'a>(&self, document: &mut Document<'a>) -> Result<()> {
        let prefix = self.generate_prefix(document);
        self.process_element(&mut document.root, &prefix);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use vexy_svgo_core::ast::{Document, DocumentMetadata, Element};

    use super::*;

    fn create_test_document() -> Document<'static> {
        Document {
            root:          Element {
                name:       "svg".into(),
                attributes: IndexMap::new(),
                namespaces: IndexMap::new(),
                children:   vec![],
            },
            prologue:      vec![],
            epilogue:      vec![],
            metadata:      DocumentMetadata {
                path:       None,
                encoding:   None,
                version:    None,
                standalone: None,
            },
            memory_budget: None,
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = PrefixIdsPlugin::new();
        assert_eq!(plugin.name(), "prefixIds");
        assert_eq!(plugin.description(), "prefix IDs");
    }

    #[test]
    fn test_param_validation() {
        let plugin = PrefixIdsPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test valid params
        assert!(plugin
            .validate_params(&serde_json::json!({
                "prefix": "custom",
                "delim": "_",
                "prefixIds": false,
                "prefixClassNames": true
            }))
            .is_ok());

        // Test invalid params
        assert!(plugin
            .validate_params(&serde_json::json!({
                "invalidParam": true
            }))
            .is_err());
    }

    #[test]
    fn test_get_basename() {
        assert_eq!(PrefixIdsPlugin::get_basename("/path/to/file.svg"), "file.svg");
        assert_eq!(PrefixIdsPlugin::get_basename("C:\\path\\to\\file.svg"), "file.svg");
        assert_eq!(PrefixIdsPlugin::get_basename("file.svg"), "file.svg");
        assert_eq!(PrefixIdsPlugin::get_basename(""), "");
    }

    #[test]
    fn test_escape_identifier_name() {
        assert_eq!(PrefixIdsPlugin::escape_identifier_name("my file.svg"), "my_file_svg");
        assert_eq!(PrefixIdsPlugin::escape_identifier_name("normal"), "normal");
    }

    #[test]
    fn test_generate_prefix() {
        // Test with custom prefix
        let config = PrefixIdsConfig {
            prefix:             Some("custom".to_string()),
            delim:              "__".to_string(),
            prefix_ids:         true,
            prefix_class_names: true,
        };
        let plugin = PrefixIdsPlugin::with_config(config);
        let doc = create_test_document();
        assert_eq!(plugin.generate_prefix(&doc), "custom__");

        // Test with file path
        let plugin = PrefixIdsPlugin::new();
        let mut doc = create_test_document();
        doc.metadata.path = Some("/path/to/test.svg".to_string());
        assert_eq!(plugin.generate_prefix(&doc), "test_svg__");

        // Test default
        let plugin = PrefixIdsPlugin::new();
        let doc = create_test_document();
        assert_eq!(plugin.generate_prefix(&doc), "prefix__");
    }

    #[test]
    fn test_prefix_id() {
        let plugin = PrefixIdsPlugin::new();

        // Test normal prefixing
        assert_eq!(plugin.prefix_id("test__", "myid"), "test__myid");

        // Test when already prefixed
        assert_eq!(plugin.prefix_id("test__", "test__myid"), "test__myid");
    }

    #[test]
    fn test_prefix_reference() {
        let plugin = PrefixIdsPlugin::new();

        // Test valid reference
        assert_eq!(plugin.prefix_reference("test__", "#myid"), Some("#test__myid".to_string()));

        // Test invalid reference
        assert_eq!(plugin.prefix_reference("test__", "myid"), None);
    }

    #[test]
    fn test_apply_with_ids() {
        let plugin = PrefixIdsPlugin::new();
        let mut doc = create_test_document();

        // Add element with ID
        let mut rect = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        rect.set_attr("id", "myId");
        doc.root.children.push(Node::Element(rect));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that ID was prefixed
        if let Node::Element(rect) = &doc.root.children[0] {
            assert_eq!(rect.attr("id"), Some("prefix__myId"));
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_apply_with_href() {
        let plugin = PrefixIdsPlugin::new();
        let mut doc = create_test_document();

        // Add element with href
        let mut use_elem = Element {
            name:       "use".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        use_elem.set_attr("href", "#myTarget");
        doc.root.children.push(Node::Element(use_elem));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that href was prefixed
        if let Node::Element(use_elem) = &doc.root.children[0] {
            assert_eq!(use_elem.attr("href"), Some("#prefix__myTarget"));
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_apply_with_custom_config() {
        let config = PrefixIdsConfig {
            prefix:             Some("custom".to_string()),
            delim:              "_".to_string(),
            prefix_ids:         true,
            prefix_class_names: true,
        };
        let plugin = PrefixIdsPlugin::with_config(config);
        let mut doc = create_test_document();

        // Add element with ID
        let mut rect = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        rect.set_attr("id", "myId");
        doc.root.children.push(Node::Element(rect));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Check that ID was prefixed with custom config
        if let Node::Element(rect) = &doc.root.children[0] {
            assert_eq!(rect.attr("id"), Some("custom_myId"));
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_process_url_references() {
        let plugin = PrefixIdsPlugin::new();

        // Test double-quoted URL
        assert_eq!(plugin.process_url_references("url(\"#icon\")", "pre_"), "url(\"#pre_icon\")");

        // Test single-quoted URL
        assert_eq!(plugin.process_url_references(r#"url('#icon')"#, "pre_"), r#"url('#pre_icon')"#);

        // Test unquoted URL
        assert_eq!(plugin.process_url_references("url(#icon)", "pre_"), "url(#pre_icon)");

        // Test non-reference URL
        assert_eq!(plugin.process_url_references("url(http://example.com)", "pre_"), "url(http://example.com)");
    }

    #[test]
    fn test_process_animation_references() {
        let plugin = PrefixIdsPlugin::new();

        assert_eq!(plugin.process_animation_references("elem1.end", "pre_"), "pre_elem1.end");

        assert_eq!(
            plugin.process_animation_references("elem1.start; elem2.end", "pre_"),
            "pre_elem1.start; pre_elem2.end"
        );

        assert_eq!(plugin.process_animation_references("5s", "pre_"), "5s");
    }

    #[test]
    fn test_process_style_content() {
        let plugin = PrefixIdsPlugin::new();

        let style = "#myId { fill: red; } .myClass { stroke: blue; } rect { fill: url(#grad); }";
        let processed = plugin.process_style_content(style, "pre_");

        assert!(processed.contains("#pre_myId"));
        assert!(processed.contains(".pre_myClass"));
        assert!(processed.contains("url(#pre_grad)"));
    }
}
