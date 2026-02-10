// this_file: crates/plugin-sdk/src/plugins/remove_elements_by_attr.rs

//! Removes elements by ID or class attribute
//!
//! This plugin removes arbitrary elements that match specified ID or class attributes.
//! Elements can be removed based on their id attribute or class attribute values.
//!
//! Reference: SVGO's removeElementsByAttr plugin

use std::collections::HashSet;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

use crate::Plugin;

/// Configuration for the removeElementsByAttr plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveElementsByAttrConfig {
    /// IDs of elements to remove
    #[serde(default)]
    pub id:    Vec<String>,
    /// Class names of elements to remove
    #[serde(default)]
    pub class: Vec<String>,
}

impl Default for RemoveElementsByAttrConfig {
    fn default() -> Self {
        Self {
            id:    vec![],
            class: vec![],
        }
    }
}

/// Removes elements by ID or class attribute
pub struct RemoveElementsByAttrPlugin {
    config: RemoveElementsByAttrConfig,
}

impl RemoveElementsByAttrPlugin {
    pub fn new() -> Self {
        Self {
            config: RemoveElementsByAttrConfig::default(),
        }
    }

    pub fn with_config(config: RemoveElementsByAttrConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<RemoveElementsByAttrConfig> {
        if params.is_null() {
            Ok(RemoveElementsByAttrConfig::default())
        } else if let Value::Object(obj) = params {
            let mut config = RemoveElementsByAttrConfig::default();

            // Parse IDs
            if let Some(id_value) = obj.get("id") {
                match id_value {
                    Value::String(id) => config.id.push(id.clone()),
                    Value::Array(ids) => {
                        for id in ids {
                            if let Value::String(id_str) = id {
                                config.id.push(id_str.clone());
                            }
                        }
                    },
                    _ => {},
                }
            }

            // Parse classes
            if let Some(class_value) = obj.get("class") {
                match class_value {
                    Value::String(class) => config.class.push(class.clone()),
                    Value::Array(classes) => {
                        for class in classes {
                            if let Value::String(class_str) = class {
                                config.class.push(class_str.clone());
                            }
                        }
                    },
                    _ => {},
                }
            }

            Ok(config)
        } else {
            serde_json::from_value(params.clone()).map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    fn should_remove_element(&self, element: &Element) -> bool {
        // Check if element ID matches any configured IDs
        if !self.config.id.is_empty() {
            if let Some(id) = element.attr("id") {
                if self.config.id.contains(&id.to_string()) {
                    return true;
                }
            }
        }

        // Check if element class contains any of the configured classes
        if !self.config.class.is_empty() {
            if let Some(class_attr) = element.attr("class") {
                let class_list: HashSet<&str> = class_attr.split_whitespace().collect();
                for config_class in &self.config.class {
                    if class_list.contains(config_class.as_str()) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn process_element(&self, element: &mut Element) {
        // Process children, removing elements that match the criteria
        let mut i = 0;
        while i < element.children.len() {
            let should_remove = match &element.children[i] {
                Node::Element(child_elem) => self.should_remove_element(child_elem),
                _ => false,
            };

            if should_remove {
                element.children.remove(i);
            } else {
                // Recursively process child elements
                if let Node::Element(child_elem) = &mut element.children[i] {
                    self.process_element(child_elem);
                }
                i += 1;
            }
        }
    }
}

impl Default for RemoveElementsByAttrPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveElementsByAttrPlugin {
    fn name(&self) -> &'static str {
        "removeElementsByAttr"
    }

    fn description(&self) -> &'static str {
        "removes arbitrary elements by ID or className (disabled by default)"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn configure(&mut self, params: &Value) -> Result<()> {
        self.config = Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Only proceed if we have something to remove
        if self.config.id.is_empty() && self.config.class.is_empty() {
            return Ok(());
        }

        self.process_element(&mut document.root);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use indexmap::IndexMap;
    use serde_json::json;
    use vexy_svgo_core::ast::{Document, DocumentMetadata, Element, Node};

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
                path: None,
                encoding: None,
                version: None,
                ..Default::default()
            },
            memory_budget: None,
        }
    }

    #[test]
    fn test_plugin_info() {
        let plugin = RemoveElementsByAttrPlugin::new();
        assert_eq!(plugin.name(), "removeElementsByAttr");
        assert_eq!(plugin.description(), "removes arbitrary elements by ID or className (disabled by default)");
    }

    #[test]
    fn test_param_validation() {
        let plugin = RemoveElementsByAttrPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test valid params
        assert!(plugin
            .validate_params(&json!({
                "id": "test"
            }))
            .is_ok());

        assert!(plugin
            .validate_params(&json!({
                "class": "test-class"
            }))
            .is_ok());

        assert!(plugin
            .validate_params(&json!({
                "id": ["id1", "id2"],
                "class": ["class1", "class2"]
            }))
            .is_ok());
    }

    #[test]
    fn test_parse_config_single_id() {
        let config_json = json!({
            "id": "elementToRemove"
        });

        let config = RemoveElementsByAttrPlugin::parse_config(&config_json).unwrap();
        assert_eq!(config.id, vec!["elementToRemove"]);
        assert!(config.class.is_empty());
    }

    #[test]
    fn test_parse_config_multiple_ids() {
        let config_json = json!({
            "id": ["elementToRemove1", "elementToRemove2"]
        });

        let config = RemoveElementsByAttrPlugin::parse_config(&config_json).unwrap();
        assert_eq!(config.id, vec!["elementToRemove1", "elementToRemove2"]);
        assert!(config.class.is_empty());
    }

    #[test]
    fn test_parse_config_single_class() {
        let config_json = json!({
            "class": "classToRemove"
        });

        let config = RemoveElementsByAttrPlugin::parse_config(&config_json).unwrap();
        assert!(config.id.is_empty());
        assert_eq!(config.class, vec!["classToRemove"]);
    }

    #[test]
    fn test_parse_config_multiple_classes() {
        let config_json = json!({
            "class": ["classToRemove1", "classToRemove2"]
        });

        let config = RemoveElementsByAttrPlugin::parse_config(&config_json).unwrap();
        assert!(config.id.is_empty());
        assert_eq!(config.class, vec!["classToRemove1", "classToRemove2"]);
    }

    #[test]
    fn test_parse_config_mixed() {
        let config_json = json!({
            "id": "elementToRemove",
            "class": ["classToRemove1", "classToRemove2"]
        });

        let config = RemoveElementsByAttrPlugin::parse_config(&config_json).unwrap();
        assert_eq!(config.id, vec!["elementToRemove"]);
        assert_eq!(config.class, vec!["classToRemove1", "classToRemove2"]);
    }

    #[test]
    fn test_should_remove_element_by_id() {
        let config = RemoveElementsByAttrConfig {
            id:    vec!["removeMe".to_string()],
            class: vec![],
        };
        let plugin = RemoveElementsByAttrPlugin::with_config(config);

        let mut element = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element.set_attr("id", "removeMe");

        assert!(plugin.should_remove_element(&element));

        // Test element that shouldn't be removed
        let mut element2 = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element2.set_attr("id", "keepMe");

        assert!(!plugin.should_remove_element(&element2));
    }

    #[test]
    fn test_should_remove_element_by_class() {
        let config = RemoveElementsByAttrConfig {
            id:    vec![],
            class: vec!["removeMe".to_string()],
        };
        let plugin = RemoveElementsByAttrPlugin::with_config(config);

        // Test element with matching class
        let mut element = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element.set_attr("class", "someClass removeMe anotherClass");

        assert!(plugin.should_remove_element(&element));

        // Test element that shouldn't be removed
        let mut element2 = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element2.set_attr("class", "someClass keepMe anotherClass");

        assert!(!plugin.should_remove_element(&element2));
    }

    #[test]
    fn test_apply_removes_by_id() {
        let config = RemoveElementsByAttrConfig {
            id:    vec!["elementToRemove".to_string()],
            class: vec![],
        };
        let plugin = RemoveElementsByAttrPlugin::with_config(config);
        let mut doc = create_test_document();

        // Add element to remove
        let mut element_to_remove = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element_to_remove.set_attr("id", "elementToRemove");
        doc.root.children.push(Node::Element(element_to_remove));

        // Add element to keep
        let mut element_to_keep = Element {
            name:       "circle".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element_to_keep.set_attr("id", "elementToKeep");
        doc.root.children.push(Node::Element(element_to_keep));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Should have only one element remaining
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(element) = &doc.root.children[0] {
            assert_eq!(element.name, "circle");
            assert_eq!(element.attr("id"), Some("elementToKeep"));
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_apply_removes_by_class() {
        let config = RemoveElementsByAttrConfig {
            id:    vec![],
            class: vec!["removeMe".to_string()],
        };
        let plugin = RemoveElementsByAttrPlugin::with_config(config);
        let mut doc = create_test_document();

        // Add element to remove
        let mut element_to_remove = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element_to_remove.set_attr("class", "some-class removeMe another-class");
        doc.root.children.push(Node::Element(element_to_remove));

        // Add element to keep
        let mut element_to_keep = Element {
            name:       "circle".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element_to_keep.set_attr("class", "some-class keep-me another-class");
        doc.root.children.push(Node::Element(element_to_keep));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Should have only one element remaining
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(element) = &doc.root.children[0] {
            assert_eq!(element.name, "circle");
            assert_eq!(element.attr("class"), Some("some-class keep-me another-class"));
        } else {
            panic!("Expected element");
        }
    }

    #[test]
    fn test_apply_no_config_does_nothing() {
        let plugin = RemoveElementsByAttrPlugin::new();
        let mut doc = create_test_document();

        // Add some elements
        let mut element = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        element.set_attr("id", "someId");
        doc.root.children.push(Node::Element(element));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Should still have the element
        assert_eq!(doc.root.children.len(), 1);
    }

    #[test]
    fn test_apply_recursive() {
        let config = RemoveElementsByAttrConfig {
            id:    vec!["removeMe".to_string()],
            class: vec![],
        };
        let plugin = RemoveElementsByAttrPlugin::with_config(config);
        let mut doc = create_test_document();

        // Create nested structure
        let mut nested_element = Element {
            name:       "rect".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![],
        };
        nested_element.set_attr("id", "removeMe");

        let mut group = Element {
            name:       "g".into(),
            attributes: IndexMap::new(),
            namespaces: IndexMap::new(),
            children:   vec![Node::Element(nested_element)],
        };
        group.set_attr("id", "group");

        doc.root.children.push(Node::Element(group));

        let result = plugin.apply(&mut doc);
        assert!(result.is_ok());

        // Group should remain but nested element should be removed
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(group) = &doc.root.children[0] {
            assert_eq!(group.name, "g");
            assert_eq!(group.children.len(), 0); // Nested element removed
        } else {
            panic!("Expected group element");
        }
    }
}
