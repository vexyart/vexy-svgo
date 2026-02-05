// this_file: crates/plugin-sdk/src/plugins/remove_deprecated_attrs.rs

//! Remove deprecated attributes
//!
//! This plugin removes deprecated SVG attributes from elements. It has a safe mode
//! that removes attributes known to be safe to remove, and an unsafe mode that
//! removes additional deprecated attributes that might affect rendering.
//!
//! Reference: SVGO's removeDeprecatedAttrs plugin

use crate::Plugin;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use vexy_svgo_core::ast::{Document, Element, Node};

/// Configuration for the plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveDeprecatedAttrsConfig {
    /// Whether to remove unsafe deprecated attributes
    #[serde(default)]
    pub remove_unsafe: bool,
}

impl Default for RemoveDeprecatedAttrsConfig {
    fn default() -> Self {
        Self {
            remove_unsafe: false,
        }
    }
}

/// Main plugin struct
pub struct RemoveDeprecatedAttrsPlugin {
    config: RemoveDeprecatedAttrsConfig,
}

impl RemoveDeprecatedAttrsPlugin {
    pub fn new() -> Self {
        Self {
            config: RemoveDeprecatedAttrsConfig::default(),
        }
    }

    pub fn with_config(config: RemoveDeprecatedAttrsConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveDeprecatedAttrsConfig> {
        if params.is_null() {
            Ok(RemoveDeprecatedAttrsConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn process_element(&self, element: &mut Element) {
        // Process children first
        let mut i = 0;
        while i < element.children.len() {
            if let Node::Element(child) = &mut element.children[i] {
                self.process_element(child);
            }
            i += 1;
        }

        // Get element configuration
        if let Some(elem_config) = ELEMENT_CONFIGS.get(element.name.as_ref()) {
            // Special case: Remove xml:lang if lang attribute exists
            if elem_config.attrs_groups.contains("core")
                && element.has_attr("xml:lang")
                && element.has_attr("lang")
            {
                element.remove_attr("xml:lang");
            }

            // Process deprecated attributes from attribute groups
            for attrs_group in &elem_config.attrs_groups {
                if let Some(deprecated_attrs) = ATTRS_GROUPS_DEPRECATED.get(attrs_group) {
                    self.process_attributes(element, deprecated_attrs);
                }
            }

            // Process element-specific deprecated attributes
            if let Some(ref deprecated) = elem_config.deprecated {
                self.process_attributes(element, deprecated);
            }
        }
    }

    fn process_attributes(&self, element: &mut Element, deprecated_attrs: &DeprecatedAttrs) {
        // Remove safe deprecated attributes
        for attr_name in &deprecated_attrs.safe {
            element.remove_attr(attr_name);
        }

        // Remove unsafe deprecated attributes if requested
        if self.config.remove_unsafe {
            for attr_name in &deprecated_attrs.unsafe_attrs {
                element.remove_attr(attr_name);
            }
        }
    }
}

impl Default for RemoveDeprecatedAttrsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveDeprecatedAttrsPlugin {
    fn name(&self) -> &'static str {
        "removeDeprecatedAttrs"
    }

    fn description(&self) -> &'static str {
        "removes deprecated attributes"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn apply<'a>(&self, document: &mut Document<'a>) -> Result<()> {
        self.process_element(&mut document.root);
        Ok(())
    }
}

/// Deprecated attributes structure
#[derive(Debug, Clone)]
struct DeprecatedAttrs {
    safe: HashSet<String>,
    unsafe_attrs: HashSet<String>,
}

/// Element configuration
#[derive(Debug, Clone)]
struct ElementConfig {
    attrs_groups: HashSet<&'static str>,
    deprecated: Option<DeprecatedAttrs>,
}

/// Deprecated attributes grouped by attribute group
static ATTRS_GROUPS_DEPRECATED: Lazy<HashMap<&'static str, DeprecatedAttrs>> = Lazy::new(|| {
    let mut map = HashMap::new();

    map.insert(
        "animationAttributeTarget",
        DeprecatedAttrs {
            safe: HashSet::new(),
            unsafe_attrs: vec!["attributeType"]
                .into_iter()
                .map(String::from)
                .collect(),
        },
    );

    map.insert(
        "conditionalProcessing",
        DeprecatedAttrs {
            safe: HashSet::new(),
            unsafe_attrs: vec!["requiredFeatures"]
                .into_iter()
                .map(String::from)
                .collect(),
        },
    );

    map.insert(
        "core",
        DeprecatedAttrs {
            safe: HashSet::new(),
            unsafe_attrs: vec!["xml:base", "xml:lang", "xml:space"]
                .into_iter()
                .map(String::from)
                .collect(),
        },
    );

    map.insert(
        "presentation",
        DeprecatedAttrs {
            safe: HashSet::new(),
            unsafe_attrs: vec![
                "clip",
                "color-profile",
                "enable-background",
                "glyph-orientation-horizontal",
                "glyph-orientation-vertical",
                "kerning",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        },
    );

    map
});

/// Element configurations with their attribute groups
static ELEMENT_CONFIGS: Lazy<HashMap<&'static str, ElementConfig>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Common attribute groups
    let common_groups = vec![
        "conditionalProcessing",
        "core",
        "graphicalEvent",
        "presentation",
    ];

    // Define configurations for various elements
    map.insert(
        "a",
        ElementConfig {
            attrs_groups: common_groups.iter().copied().chain(vec!["xlink"]).collect(),
            deprecated: None,
        },
    );

    map.insert(
        "circle",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "ellipse",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "g",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "image",
        ElementConfig {
            attrs_groups: common_groups.iter().copied().chain(vec!["xlink"]).collect(),
            deprecated: None,
        },
    );

    map.insert(
        "line",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "path",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "polygon",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "polyline",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "rect",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "svg",
        ElementConfig {
            attrs_groups: vec![
                "conditionalProcessing",
                "core",
                "documentEvent",
                "graphicalEvent",
                "presentation",
            ]
            .into_iter()
            .collect(),
            deprecated: None,
        },
    );

    map.insert(
        "text",
        ElementConfig {
            attrs_groups: common_groups.clone().into_iter().collect(),
            deprecated: None,
        },
    );

    map.insert(
        "use",
        ElementConfig {
            attrs_groups: common_groups.iter().copied().chain(vec!["xlink"]).collect(),
            deprecated: None,
        },
    );

    // Animation elements
    map.insert(
        "animate",
        ElementConfig {
            attrs_groups: vec![
                "conditionalProcessing",
                "core",
                "animationEvent",
                "xlink",
                "animationAttributeTarget",
                "animationTiming",
                "animationValue",
                "animationAddition",
                "presentation",
            ]
            .into_iter()
            .collect(),
            deprecated: None,
        },
    );

    map.insert(
        "animateTransform",
        ElementConfig {
            attrs_groups: vec![
                "conditionalProcessing",
                "core",
                "animationEvent",
                "xlink",
                "animationAttributeTarget",
                "animationTiming",
                "animationValue",
                "animationAddition",
            ]
            .into_iter()
            .collect(),
            deprecated: None,
        },
    );

    // Add more elements as needed
    map
});

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_document() -> Document<'static> {
        let mut doc = Document::default();

        let mut svg = Element {
            name: "svg".into(),
            namespaces: IndexMap::new(),
            attributes: IndexMap::new(),
            children: vec![],
        };
        svg.set_attr("xml:lang", "en");
        svg.set_attr("lang", "en");
        svg.set_attr("xml:space", "preserve");

        let mut rect = Element {
            name: "rect".into(),
            namespaces: IndexMap::new(),
            attributes: IndexMap::new(),
            children: vec![],
        };
        rect.set_attr("x", "0");
        rect.set_attr("y", "0");
        rect.set_attr("width", "100");
        rect.set_attr("height", "100");
        rect.set_attr("enable-background", "new");
        rect.set_attr("clip", "rect(0 0 100 100)");

        svg.children.push(Node::Element(rect));
        doc.root = svg;
        doc
    }

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveDeprecatedAttrsPlugin::new();
        assert_eq!(plugin.name(), "removeDeprecatedAttrs");
        assert_eq!(plugin.description(), "removes deprecated attributes");
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveDeprecatedAttrsPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test valid params
        assert!(plugin
            .validate_params(&json!({
                "removeUnsafe": true
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
    fn test_remove_xml_lang_when_lang_exists() {
        let mut doc = create_test_document();
        let plugin = RemoveDeprecatedAttrsPlugin::new();

        plugin.apply(&mut doc).unwrap();

        // xml:lang should be removed because lang exists
        assert_eq!(doc.root.attr("xml:lang"), None);
        assert_eq!(doc.root.attr("lang"), Some("en"));
        // xml:space should still exist (unsafe attribute)
        assert_eq!(doc.root.attr("xml:space"), Some("preserve"));
    }

    #[test]
    fn test_remove_unsafe_attributes() {
        let mut doc = create_test_document();
        let config = RemoveDeprecatedAttrsConfig {
            remove_unsafe: true,
        };
        let plugin = RemoveDeprecatedAttrsPlugin::with_config(config);

        plugin.apply(&mut doc).unwrap();

        // xml:space should be removed with removeUnsafe
        assert_eq!(doc.root.attr("xml:space"), None);

        // Check rect element - unsafe presentation attributes should be removed
        if let Some(Node::Element(ref rect)) = doc.root.children.first() {
            assert_eq!(rect.attr("enable-background"), None);
            assert_eq!(rect.attr("clip"), None);
            // Regular attributes should remain
            assert_eq!(rect.attr("width"), Some("100"));
        }
    }

    #[test]
    fn test_keep_xml_lang_without_lang() {
        let mut doc = Document::default();

        let mut svg = Element {
            name: "svg".into(),
            namespaces: IndexMap::new(),
            attributes: IndexMap::new(),
            children: vec![],
        };
        svg.set_attr("xml:lang", "en");
        // No lang attribute

        doc.root = svg;

        let plugin = RemoveDeprecatedAttrsPlugin::new();

        plugin.apply(&mut doc).unwrap();

        // xml:lang should be kept because lang doesn't exist
        assert_eq!(doc.root.attr("xml:lang"), Some("en"));
    }

    #[test]
    fn test_animation_attribute_target() {
        let mut doc = Document::default();

        let mut svg = Element {
            name: "svg".into(),
            namespaces: IndexMap::new(),
            attributes: IndexMap::new(),
            children: vec![],
        };

        let mut animate = Element {
            name: "animate".into(),
            namespaces: IndexMap::new(),
            attributes: IndexMap::new(),
            children: vec![],
        };
        animate.set_attr("attributeType", "XML");
        animate.set_attr("attributeName", "x");

        svg.children.push(Node::Element(animate));
        doc.root = svg;

        let config = RemoveDeprecatedAttrsConfig {
            remove_unsafe: true,
        };
        let plugin = RemoveDeprecatedAttrsPlugin::with_config(config);

        plugin.apply(&mut doc).unwrap();

        // attributeType is an unsafe deprecated attribute
        if let Some(Node::Element(ref animate)) = doc.root.children.first() {
            assert_eq!(animate.attr("attributeType"), None);
            assert_eq!(animate.attr("attributeName"), Some("x"));
        }
    }
}
