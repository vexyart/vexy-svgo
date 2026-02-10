// this_file: crates/plugin-sdk/tests/registry_test.rs

//! Integration tests for the plugin registry system with multiple plugins

use serde_json::json;
use std::borrow::Cow;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::{PluginConfig, PluginRegistry};
use vexy_svgo_plugin_sdk::{Plugin, plugins::{CollapseGroupsPlugin, RemoveCommentsPlugin, RemoveEmptyAttrsPlugin}};

#[test]
fn test_registry_with_multiple_plugins() {
    let mut registry = PluginRegistry::new();

    // Register multiple plugins
    registry.register("removeComments", || RemoveCommentsPlugin::new());
    registry.register("removeEmptyAttrs", || RemoveEmptyAttrsPlugin::new());
    registry.register("collapseGroups", || CollapseGroupsPlugin::new());

    // Verify plugins are registered
    assert!(registry.create_plugin("removeComments").is_some());
    assert!(registry.create_plugin("removeEmptyAttrs").is_some());
    assert!(registry.create_plugin("collapseGroups").is_some());
    assert!(registry.create_plugin("nonexistent").is_none());

    let plugin_names = registry.plugin_names();
    assert!(plugin_names.contains(&"removeComments"));
    assert!(plugin_names.contains(&"removeEmptyAttrs"));
    assert!(plugin_names.contains(&"collapseGroups"));
    assert_eq!(plugin_names.len(), 3);
}

#[test]
fn test_plugin_pipeline_execution() {
    let mut registry = PluginRegistry::new();
    registry.register("removeComments", || RemoveCommentsPlugin::new());
    registry.register("removeEmptyAttrs", || RemoveEmptyAttrsPlugin::new());
    registry.register("collapseGroups", || CollapseGroupsPlugin::new());

    // Create test document
    let mut doc = create_complex_test_document();

    // Count initial state
    let initial_comments = count_comments(&doc.root);
    let initial_empty_attrs = count_empty_attributes(&doc.root);
    assert!(initial_comments > 0);
    assert!(initial_empty_attrs > 0);

    // Configure plugins
    let configs = vec![
        PluginConfig::WithParams {
            name: "removeComments".to_string(),
            params: json!({"preservePatterns": true}),
        },
        PluginConfig::WithParams {
            name: "removeEmptyAttrs".to_string(),
            params: json!({}),
        },
    ];

    // Apply plugins
    let result = registry.apply_plugins(&mut doc, &configs);
    assert!(result.is_ok());

    // Verify both plugins were applied
    let final_comments = count_comments(&doc.root);
    let final_empty_attrs = count_empty_attributes(&doc.root);

    // Comments should be reduced (but not all removed due to preserve patterns)
    assert!(final_comments < initial_comments);
    assert!(final_comments > 0); // Legal comments should remain

    // Empty attributes should be completely removed
    assert_eq!(final_empty_attrs, 0);
}

#[test]
fn test_disabled_plugins() {
    let mut registry = PluginRegistry::new();
    registry.register("removeComments", || RemoveCommentsPlugin::new());
    registry.register("removeEmptyAttrs", || RemoveEmptyAttrsPlugin::new());

    let mut doc = create_complex_test_document();
    let initial_comments = count_comments(&doc.root);
    let initial_empty_attrs = count_empty_attributes(&doc.root);

    // Configure with disabled plugins - NOTE: Using empty vec to disable removeComments
    let configs = vec![
        PluginConfig::WithParams {
            name: "removeEmptyAttrs".to_string(),
            params: json!({}),
        },
    ];

    registry.apply_plugins(&mut doc, &configs).unwrap();

    // Only the enabled plugin should have been applied
    assert_eq!(count_comments(&doc.root), initial_comments); // Unchanged
    assert_eq!(count_empty_attributes(&doc.root), 0); // Changed
}

#[test]
fn test_plugin_parameter_validation() {
    let mut registry = PluginRegistry::new();
    registry.register("removeComments", || RemoveCommentsPlugin::new());

    let mut doc = Document::new();

    // Valid parameters should work
    let valid_config = PluginConfig::WithParams {
        name: "removeComments".to_string(),
        params: json!({"preservePatterns": true}),
    };

    let result = registry.apply_plugin(&mut doc, &valid_config);
    assert!(result.is_ok());

    // Invalid parameters should fail
    let invalid_config = PluginConfig::WithParams {
        name: "removeComments".to_string(),
        params: json!({"preservePatterns": "invalid"}),
    };

    let result = registry.apply_plugin(&mut doc, &invalid_config);
    assert!(result.is_err());
}

#[test]
fn test_nonexistent_plugin() {
    let registry = PluginRegistry::new();
    let mut doc = Document::new();

    let config = PluginConfig::WithParams {
        name: "nonexistentPlugin".to_string(),
        params: json!({}),
    };

    // Should succeed but do nothing for nonexistent plugins
    let result = registry.apply_plugin(&mut doc, &config);
    assert!(result.is_ok());
}

#[test]
fn test_plugin_order_matters() {
    let mut registry = PluginRegistry::new();
    registry.register("removeComments", || RemoveCommentsPlugin::with_preserve_patterns(false));
    registry.register("removeEmptyAttrs", || RemoveEmptyAttrsPlugin::new());

    // Create two identical documents
    let mut doc1 = create_complex_test_document();
    let mut doc2 = create_complex_test_document();

    // Apply plugins in different orders
    let configs1 = vec![
        PluginConfig::WithParams {
            name: "removeComments".to_string(),
            params: json!({}),
        },
        PluginConfig::WithParams {
            name: "removeEmptyAttrs".to_string(),
            params: json!({}),
        },
    ];

    let configs2 = vec![
        PluginConfig::WithParams {
            name: "removeEmptyAttrs".to_string(),
            params: json!({}),
        },
        PluginConfig::WithParams {
            name: "removeComments".to_string(),
            params: json!({}),
        },
    ];

    registry.apply_plugins(&mut doc1, &configs1).unwrap();
    registry.apply_plugins(&mut doc2, &configs2).unwrap();

    // Final state should be the same regardless of order for these plugins
    assert_eq!(count_comments(&doc1.root), count_comments(&doc2.root));
    assert_eq!(
        count_empty_attributes(&doc1.root),
        count_empty_attributes(&doc2.root)
    );
}

// Helper functions

fn create_complex_test_document() -> Document<'static> {
    let mut doc = Document::new();

    doc.root.name = Cow::Borrowed("svg");
    doc.root
        .attributes
        .insert(Cow::Borrowed("width"), Cow::Borrowed("100"));
    doc.root
        .attributes
        .insert(Cow::Borrowed("height"), Cow::Borrowed("100"));
    doc.root
        .attributes
        .insert(Cow::Borrowed("fill"), Cow::Borrowed(""));
    doc.root
        .attributes
        .insert(Cow::Borrowed("stroke"), Cow::Borrowed("  "));
    doc.root
        .attributes
        .insert(Cow::Borrowed("class"), Cow::Borrowed(""));

    // Add various comments
    doc.root
        .children
        .push(Node::Comment("Regular comment 1".to_string().into_boxed_str()));
    doc.root
        .children
        .push(Node::Comment("! Legal comment".to_string().into_boxed_str()));
    doc.root
        .children
        .push(Node::Comment("Regular comment 2".to_string().into_boxed_str()));

    // Add nested elements with empty attributes
    let mut group = Element::new("g");
    group
        .attributes
        .insert(Cow::Borrowed("id"), Cow::Borrowed("group1"));
    group
        .attributes
        .insert(Cow::Borrowed("transform"), Cow::Borrowed(""));
    group
        .attributes
        .insert(Cow::Borrowed("opacity"), Cow::Borrowed(""));

    group
        .children
        .push(Node::Comment("Nested comment".to_string().into_boxed_str()));

    let mut rect = Element::new("rect");
    rect.attributes.insert(Cow::Borrowed("x"), Cow::Borrowed("10"));
    rect.attributes.insert(Cow::Borrowed("y"), Cow::Borrowed("10"));
    rect.attributes
        .insert(Cow::Borrowed("width"), Cow::Borrowed("50"));
    rect.attributes
        .insert(Cow::Borrowed("height"), Cow::Borrowed("50"));
    rect.attributes
        .insert(Cow::Borrowed("fill"), Cow::Borrowed("blue"));
    rect.attributes.insert(Cow::Borrowed("stroke"), Cow::Borrowed(""));
    rect.attributes
        .insert(Cow::Borrowed("data-test"), Cow::Borrowed("   "));

    group.children.push(Node::Element(rect));
    doc.root.children.push(Node::Element(group));

    doc
}

fn count_comments(element: &Element) -> usize {
    let mut count = 0;
    for child in &element.children {
        match child {
            Node::Comment(_) => count += 1,
            Node::Element(elem) => count += count_comments(elem),
            _ => {}
        }
    }
    count
}

fn count_empty_attributes(element: &Element) -> usize {
    let mut count = element
        .attributes
        .values()
        .filter(|v| v.trim().is_empty())
        .count();

    for child in &element.children {
        if let Node::Element(elem) = child {
            count += count_empty_attributes(elem);
        }
    }
    count
}
