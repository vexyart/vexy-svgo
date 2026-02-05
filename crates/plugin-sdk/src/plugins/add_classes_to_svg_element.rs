// this_file: crates/plugin-sdk/src/plugins/add_classes_to_svg_element.rs

//! Add classes to SVG element plugin implementation
//!
//! This plugin adds class names to the outer <svg> element.
//! It adds classes to the existing class attribute, preserving
//! any classes that are already present.
//!
//! Reference: SVGO's addClassesToSVGElement plugin

use crate::Plugin;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use vexy_svgo_core::ast::{Document, Element};

/// Configuration parameters for add classes to SVG element plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddClassesToSVGElementConfig {
    /// Single class name to add
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// Array of class names to add
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_names: Option<Vec<String>>,
}

impl Default for AddClassesToSVGElementConfig {
    fn default() -> Self {
        Self {
            class_name: None,
            class_names: None,
        }
    }
}

/// Plugin that adds classes to SVG element
pub struct AddClassesToSVGElementPlugin {
    config: AddClassesToSVGElementConfig,
}

impl AddClassesToSVGElementPlugin {
    /// Create a new AddClassesToSVGElementPlugin
    pub fn new() -> Self {
        Self {
            config: AddClassesToSVGElementConfig::default(),
        }
    }

    /// Create a new AddClassesToSVGElementPlugin with config
    pub fn with_config(config: AddClassesToSVGElementConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn parse_config(params: &Value) -> Result<AddClassesToSVGElementConfig, anyhow::Error> {
        if params.is_null() {
            Ok(AddClassesToSVGElementConfig::default())
        } else if params.is_object() {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid configuration: {}", e))
        } else {
            Err(anyhow::anyhow!("Configuration must be an object"))
        }
    }

    /// Apply classes to the root SVG element
    fn apply_classes(&self, element: &mut Element) {
        // Gather all classes to apply
        let mut classes_to_add = Vec::new();

        // Add single class if present
        if let Some(ref class_name) = self.config.class_name {
            classes_to_add.push(class_name.clone());
        }

        // Add array of classes if present
        if let Some(ref class_names) = self.config.class_names {
            classes_to_add.extend(class_names.iter().cloned());
        }

        // Get existing classes
        let mut class_set: HashSet<String> =
            if let Some(existing_class) = element.attributes.get("class") {
                existing_class
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                HashSet::new()
            };

        // Add new classes
        for class_name in classes_to_add {
            if !class_name.is_empty() {
                class_set.insert(class_name);
            }
        }

        // Update class attribute
        if !class_set.is_empty() {
            let class_string = class_set.into_iter().collect::<Vec<_>>().join(" ");
            element.attributes.insert("class".into(), class_string.into());
        }
    }
}

impl Default for AddClassesToSVGElementPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for AddClassesToSVGElementPlugin {
    fn name(&self) -> &'static str {
        "addClassesToSVGElement"
    }

    fn description(&self) -> &'static str {
        "adds classnames to an outer <svg> element"
    }

    fn validate_params(&self, params: &Value) -> anyhow::Result<()> {
        let config = Self::parse_config(params)?;

        // Validate that at least one of className or classNames is specified
        if config.class_name.is_none()
            && (config.class_names.is_none() || config.class_names.as_ref().unwrap().is_empty())
        {
            return Err(anyhow::anyhow!(
                "Error in plugin \"addClassesToSVGElement\": absent parameters.\n\
                It should have a list of classes in \"classNames\" or one \"className\"."
            ));
        }

        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        // Only apply to root SVG element
        if document.root.name == "svg" {
            self.apply_classes(&mut document.root);
        }
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use serde_json::json;
    use std::borrow::Cow;
    use vexy_svgo_core::ast::{Document, Element};

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = AddClassesToSVGElementPlugin::new();
        assert_eq!(plugin.name(), "addClassesToSVGElement");
        assert_eq!(
            plugin.description(),
            "adds classnames to an outer <svg> element"
        );
    }

    #[test]
    fn test_parameter_validation_missing_params() {
        let plugin = AddClassesToSVGElementPlugin::new();

        // Invalid - no parameters
        assert!(plugin.validate_params(&json!({})).is_err());

        // Invalid - empty classNames array
        assert!(plugin
            .validate_params(&json!({
                "classNames": []
            }))
            .is_err());
    }

    #[test]
    fn test_parameter_validation_single_class() {
        let plugin = AddClassesToSVGElementPlugin::new();

        // Valid - single class
        assert!(plugin
            .validate_params(&json!({
                "className": "myClass"
            }))
            .is_ok());
    }

    #[test]
    fn test_parameter_validation_multiple_classes() {
        let plugin = AddClassesToSVGElementPlugin::new();

        // Valid - array of classes
        assert!(plugin
            .validate_params(&json!({
                "classNames": ["class1", "class2"]
            }))
            .is_ok());
    }

    #[test]
    fn test_add_single_class() {
        let config = AddClassesToSVGElementConfig {
            class_name: Some("myClass".to_string()),
            class_names: None,
        };
        let plugin = AddClassesToSVGElementPlugin::with_config(config);

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that class was added
        assert_eq!(doc.root.attr("class"), Some("myClass"));
    }

    #[test]
    fn test_add_multiple_classes() {
        let config = AddClassesToSVGElementConfig {
            class_name: None,
            class_names: Some(vec![
                "class1".to_string(),
                "class2".to_string(),
                "class3".to_string(),
            ]),
        };
        let plugin = AddClassesToSVGElementPlugin::with_config(config);

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all classes were added
        let class_attr = doc.root.attr("class").unwrap();
        let classes: HashSet<&str> = class_attr.split_whitespace().collect();
        assert!(classes.contains("class1"));
        assert!(classes.contains("class2"));
        assert!(classes.contains("class3"));
    }

    #[test]
    fn test_preserves_existing_classes() {
        let config = AddClassesToSVGElementConfig {
            class_name: Some("newClass".to_string()),
            class_names: None,
        };
        let plugin = AddClassesToSVGElementPlugin::with_config(config);

        let mut doc = Document::new();
        doc.root = create_element("svg");
        doc.root.set_attr("class", "existingClass1 existingClass2");

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that both existing and new classes are present
        let class_attr = doc.root.attr("class").unwrap();
        let classes: HashSet<&str> = class_attr.split_whitespace().collect();
        assert!(classes.contains("existingClass1"));
        assert!(classes.contains("existingClass2"));
        assert!(classes.contains("newClass"));
    }

    #[test]
    fn test_deduplicates_classes() {
        let config = AddClassesToSVGElementConfig {
            class_name: Some("duplicateClass".to_string()),
            class_names: Some(vec![
                "duplicateClass".to_string(),
                "uniqueClass".to_string(),
            ]),
        };
        let plugin = AddClassesToSVGElementPlugin::with_config(config);

        let mut doc = Document::new();
        doc.root = create_element("svg");
        doc.root.set_attr("class", "duplicateClass");

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that duplicate class appears only once
        let class_attr = doc.root.attr("class").unwrap();
        let classes: Vec<&str> = class_attr.split_whitespace().collect();
        assert_eq!(
            classes.iter().filter(|&&c| c == "duplicateClass").count(),
            1
        );
        assert!(classes.contains(&"uniqueClass"));
    }

    #[test]
    fn test_only_applies_to_svg_element() {
        let config = AddClassesToSVGElementConfig {
            class_name: Some("myClass".to_string()),
            class_names: None,
        };
        let plugin = AddClassesToSVGElementPlugin::with_config(config);

        let mut doc = Document::new();
        doc.root = create_element("div"); // Not an SVG element

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that no class was added
        assert!(!doc.root.attributes.contains_key("class"));
    }

    #[test]
    fn test_both_class_name_and_class_names() {
        let config = AddClassesToSVGElementConfig {
            class_name: Some("single".to_string()),
            class_names: Some(vec!["multiple1".to_string(), "multiple2".to_string()]),
        };
        let plugin = AddClassesToSVGElementPlugin::with_config(config);

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that all classes were added
        let class_attr = doc.root.attr("class").unwrap();
        let classes: HashSet<&str> = class_attr.split_whitespace().collect();
        assert!(classes.contains("single"));
        assert!(classes.contains("multiple1"));
        assert!(classes.contains("multiple2"));
    }

    #[test]
    fn test_empty_class_names_are_ignored() {
        let config = AddClassesToSVGElementConfig {
            class_name: Some("".to_string()), // Empty string
            class_names: Some(vec!["valid".to_string(), "".to_string()]), // Contains empty
        };
        let plugin = AddClassesToSVGElementPlugin::with_config(config);

        let mut doc = Document::new();
        doc.root = create_element("svg");

        // Apply plugin
        plugin.apply(&mut doc).unwrap();

        // Check that only valid class was added
        let class_attr = doc.root.attr("class").unwrap();
        assert_eq!(class_attr, "valid");
    }

    #[test]
    fn test_config_parsing() {
        // Test single class
        let config = AddClassesToSVGElementPlugin::parse_config(&json!({
            "className": "test"
        }))
        .unwrap();
        assert_eq!(config.class_name, Some("test".to_string()));

        // Test array of classes
        let config = AddClassesToSVGElementPlugin::parse_config(&json!({
            "classNames": ["class1", "class2"]
        }))
        .unwrap();
        assert_eq!(
            config.class_names,
            Some(vec!["class1".to_string(), "class2".to_string()])
        );
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(AddClassesToSVGElementPlugin, "addClassesToSVGElement");
